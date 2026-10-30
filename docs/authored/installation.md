## Installation

### Installation Script

```bash
curl -fsSL https://portals.works/api/px/download | bash
```

The one-liner installs the `px` CLI, the bundled `px-mcp-server`, and the agent skills to `~/.agents/skills`. The website command records one download per browser profile with a random ID. After a successful install, the installer sends a separate anonymous install count. Run `export PX_TELEMETRY=0` before installation to opt out of install counting. The MCP server is dormant by default; agent clients start it on demand over stdio so sandboxed agents can use PX through host-side CLI proxy calls.

**Windows PowerShell**

```powershell
iex (irm 'https://portals.works/api/px/download?platform=windows')
```

To opt out of successful-install counting in PowerShell, run `$env:PX_TELEMETRY='0'` before the command. The PX website provides a command with a persistent anonymous browser ID and campaign attribution.

<!-- ### CLI & Server (Rust — compile from source)

```bash
git clone https://github.com/portalshq/px.git
cd px
cargo build --release

# Binaries land in target/release/
#   px          — CLI tool
#   px-server   — HTTP resolver server
```

### Python SDK (prebuilt wheel, no Rust needed)

```bash
pip install narrativeengine
```

```python
from narrativeengine import create_block, generate_candidate, render_lore_summary

block = create_block("char-1", "A brave adventurer")
candidate = generate_candidate(block)
```

### TypeScript SDK (prebuilt binary, no Rust needed)

```bash
npm install @portalshq/narrativeengine
```

```typescript
import { createBlock } from "@portalshq/narrativeengine";

const block = createBlock("char-1", "A brave adventurer");
``` -->

---

## Quick Start

```bash
# Initialize a repository (prompts for provider on first run)
px init toystory

# Initialize with local provider
px init toystory --provider local

# Configure provider only (no repository)
px init --provider local

# Initialize with remote provider
px init --provider remote --remote-url lore://127.0.0.1:41337 --workspace-id my-workspace

# Initialize with Portals Cloud
px auth login
px init --provider portals-cloud

# Inspect or clear the OS-keyring-backed session
px auth status
px auth logout

# Check system status
px status

# Run diagnostics
px doctor

# Run diagnostics with auto-repair
px doctor --repair
```

Portals Cloud uses `grpcs://lore.portals.works` on standard TLS port 443. Login is
the only interactive VCS step; repository operations remain noninteractive and
return an actionable `px auth login` error when credentials are missing or
expired. Lore automatically exchanges the eight-hour login session for a
five-minute token scoped to the single repository used by init, clone, pull,
sync, push, and locking. CI uses a revocable service-account API key
exchange; do not store long-lived bearer tokens in CI variables.

`px install lore` installs the exact `portalshq/lore` release compiled into
that Px version. It downloads the installer from the same release tag,
verifies its pinned SHA-256 before execution, and explicitly selects the
Portals fork. It never executes the mutable `main` installer or silently falls
back to an upstream Lore binary. Production release metadata binds this Lore
client version to Px's signed checksum manifest.

### Local Lore server lifecycle

With the `local` provider, PX manages a machine-local `loreserver` for you:

- **Automatic startup**: `px init` and every remote-backed command (`list`,
  `resolve`, `pull`, `push`, `history`, `presign`, `query`, branch ops) runs a
  provider-readiness check that starts the server when it isn't running —
  including after reboots, crashes, and stale PID locks.
- **Fixed data directory**: the server always launches as
  `loreserver --config <px_home>/lore/config`, where `<px_home>` resolves as
  `--base-dir`, then `$PX_DIR`, then `~/.px`. Persistent state lives in
  `<px_home>/lore/store/` (immutable + mutable stores), logs in
  `<px_home>/lore/logs/loreserver.log`, and the PID lock in
  `<px_home>/lore/pid`.
- **Listener identity**: ports 41337 (gRPC/QUIC) and 41339 (HTTP health) are
  per-machine singletons. Before adopting an already-listening server, PX
  verifies the owning process was spawned with this PX home's `--config`
  directory (paths are canonicalized). A `loreserver` bound to a *different*
  config dir is terminated so the correct one can start; a non-Lore process
  is never killed and produces an actionable error instead.
- **Version parity**: the readiness check re-verifies the installed Lore
  version against the release pin before starting or reusing a server.
- **Store ownership caveat**: PX guarantees the *server* uses the right
  config/store; integrity of the store's contents is Lore's own domain —
  `px doctor` and `lore repository verify` are the tools for diagnosing
  repository-level issues.

CI reads the API key from its secret store and passes it to Lore over stdin,
so the secret is absent from process arguments and command logs:

```bash
export PORTALS_CLOUD_API_KEY="${CI_PORTALS_CLOUD_API_KEY}"
px auth login --api-key
```

Use `--api-key-env NAME` to select a different secret environment variable.

### Create a Repository

```bash
# Initialize a new repository
px init toystory

# See what you created
ls toystory/
# → .px/  repository.yaml  characters/  locations/  scenes/  props/
```

### Create & Inspect Entities

```bash
# Create a character
px create character woody -u toystory -n "Woody"

# Create a location
px create location andys-room -u toystory -n "Andy's Room"

# Set properties
px set px://toystory/character/woody toy_type human
px set px://toystory/character/woody homeworld "px://toystory/location/andys-room"

# Resolve a manifest
px resolve px://toystory/character/woody

# Query a specific field
px resolve px://toystory/character/woody#properties.toy_type
# → human

# Resolve a subtree
px resolve px://toystory/character/woody properties
```

### Version Control

```bash
# View commit history
px history px://toystory/character/woody

# Create branches
px branch toystory canon

# Sync with remote
px sync toystory

# Push to remote
px push toystory
```

### Output Formats

```bash
px resolve px://toystory/character/woody -f json
px resolve px://toystory/character/woody -f yaml
```
