# CLI Command Reference
| Command | Description |
|---|---|
| `px init <repository>` | Initialize a new repository repository (prompts for provider on first run) |
| `px init <repository> --provider <type>` | Initialize repository + configure provider |
| `px init --provider <type>` | Configure provider only (no repository) |
| `px create <type> <id> -u <repository> -n <name>` | Create a new entity manifest |
| `px resolve <uri>` | Resolve a PX URI to a manifest or subtree |
| `px resolve <uri> [path]` | Resolve a manifest or subtree; URI fragments are preferred |
| `px set <uri> <key> <value>` | Set a property on an entity |
| `px add <uri> <key> <file> --format <fmt>` | Add a content-addressed representation |
| `px commit <repository> -m <message>` | Commit changes to the VCS |
| `px history <uri>` | View commit history for an entity |
| `px list [repository]` | List repositorys or entities |
| `px branch <repository> [name]` | Create or list branches |
| `px switch <repository> <branch>` | Switch to a branch |
| `px validate <uri>` | Validate a manifest against the PX schema |
| `px push <repository>` | Push the current branch to its upstream remote (alias: `px publish`) |
| `px pull <url-or-name>` | Clone from URL or pull an existing repository |
| `px sync <repository>` | Fetch remote manifests, then push local commits |
| `px remote add <repository> <name> <url>` | Add a remote |
| `px remote ls <repository>` | List remotes |
| `px remote rm <repository> <name>` | Remove a remote |
| `px revert <repository> -c <hash>` | Revert a commit |
| `px diff <uri>` | Diff an entity working tree against its committed manifest |
| `px merge <base> <current> <proposed>` | Three-way merge |
| `px content-hash <file>` | Compute BLAKE3 content hash |
| `px schema <name>` | Print JSON schema (manifest or commit) |
| `px doctor [--repair]` | Run diagnostics and optionally auto-repair |
| `px status` | Show system status and provider info |
| `px configure [provider]` | Inspect or change the version-control provider |
| `px head <repository>` | Show the current HEAD commit hash |

### Command Notes

**`push` and `publish`:** `push` is the canonical command; `publish` is an alias. Use `--remote-name` to select a destination remote and `--branch` to select a branch.

**`sync` vs `pull`:** `sync` pulls the current branch from the default remote. `pull` accepts either a repository name (same as `sync`) or a URL (which clones the repository).

**`init` vs `configure`:** `init` can create a repository and configure a provider. Use `configure` to inspect or change a provider without creating a repository; `choose` and `backend` remain supported as deprecated compatibility commands.

**Compatibility aliases:** `px config` aliases `px configure`; `px publish` aliases `px push`; and `px head-hash` (or `px head_hash`) aliases `px head`.

Global options: `-d/--base-dir <path>` (default `~/.px`), `-v/--verbose`, `--remote`, and `--local`.

Repository repositories are stored in `~/.px/<repository>/` by default. Use `-d` (or `--base-dir`) to point to a different directory.

---
