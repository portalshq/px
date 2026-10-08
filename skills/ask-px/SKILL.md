---
name: ask-px
description: Explain PX architecture, document version tracking, repository remotes, and create/iterate/promote/resolve workflows. Use for conceptual PX questions and human CLI syntax questions; use px-repo, px-resolve, or px-update to perform operations via MCP.
metadata:
  author: portals
  version: "0.9.1"
---

# Ask PX

Explain the concepts relevant to the user's question, then show a concrete example. The CLI appendix is generated from PX's command definitions; consult it for exact flags rather than guessing. Execution by agents goes through `px-mcp-server` tools, following the appropriate operation skill.

## Architecture and Document Identity

A repository is a project container, identified by a name such as `toystory`. Its `repository.yaml` world manifest holds project-wide style, references, and narrative rules. An entity is a durable document inside that repository, identified by a URI such as `px://toystory/character/woody`. The URI remains the same across edits; branch and commit are separate selectors, not parts of the URI.

A manifest stores current properties, references to other entities, and representations. Representations are named assets such as `portrait`, `character_sheet`, an exact generation prompt, or a scene clip. A representation records a format, repository-relative URI, and BLAKE3 hash. Stable keys point at the latest asset in that branch; history preserves earlier versions.

Branches are named versions of the repository, not separate entity identities. A target branch carries accepted work; `revision-character-woody` carries iterations for Woody. The target is the branch chosen for the task, not necessarily `main`.

## How Version Tracking Works

PX uses Lore version control and content-addressed storage. BLAKE3 identifies file bytes: unchanged bytes have the same hash, while changed bytes receive a new hash. A manifest edit and its referenced assets are committed as a revision, and pushing publishes the branch's commits and content to its repository server.

An entity URI identifies the document; a branch selects its moving current version; a commit selects an immutable historical revision. The manifest's numeric version is not a substitute for the VCS commit identifier. Lore's storage fragment-tree addresses can differ from a representation's BLAKE3 file hash, especially for large files; keep these identifiers distinct.

Changing a stable representation key creates a new current version without erasing the older committed assets. Exact prompts, input file hashes, provider/model identifiers, parameters, timestamps, and job IDs provide generation provenance alongside revision history. A hash alone does not preserve recoverable prompt text.

## Common Workflows

- **Create:** establish the repository and target branch, resolve project context, create a durable entity URI, then create a revision branch from the target before generating assets.
- **Iterate:** resolve that same URI from its revision branch, track inputs as reference representations, generate a candidate, save assets and provenance, commit/push, and verify branch-specific resolution. Keep iterations on the existing entity.
- **Promote:** after explicit acceptance, apply the verified candidate to the resolved target branch, commit/push, and verify matching hashes. Saving or pushing a revision does not itself promote it.
- **Resolve:** use the entity URI plus an explicit branch or commit when version identity matters; query a fragment/subtree for only the relevant context. See `px-resolve` for execution details.

## Remote and Local Operations

Server-backed reads are the default; they reflect committed, pushed state. `--local` explicitly reads a checked-out working tree and is useful for inspecting uncommitted files. A successful local commit alone does not make changes visible remotely. PX mutation tools such as add/set/create normally commit and push; explicit commit workflows must also push.

An existing checkout uses `.px/config.yaml`'s `remote_url` first, then its persisted Lore server, then the global provider default. A global provider change supplies the server for new repositories and does not silently redirect established checkouts. `px remote set <repository> <url>` changes one repository; `px configure --force` explicitly migrates all. Repositories marked `remote_source: default` follow provider changes automatically, provided their remote still matches the prior default.

`px pull <repository>` hydrates that repository's manifests; `px pull px://<repository>/<type>/<id>` hydrates the selected entity and world manifest. Assets remain addressable on the server. `.pxignore` feeds Lore's native filter; PX always excludes filesystem/VCS housekeeping files such as `.DS_Store`, `.git`, and `.px`.

For workflow bootstrapping, consult `px init` or `px configure` in the current appendix and the `px-repo` skill. If a future installed version exposes `initialize-interactive`, use its own help to guide setup; this version does not implement `px initialize-interactive`, so do not suggest it as an available command.

## Appendix: Generated CLI Reference

These template includes keep the reference in sync with documentation generation. Shell examples are for humans; agents execute operations via MCP.


# PX CLI Reference
The `px` command-line interface (v0.9.1) provides tools for creating, resolving, and managing narrative resources using the PX protocol.


## Command Overview

| Command | Description |
|---|---|
| [\`px add\`](docs/generated/commands/add.md) | Add a file representation to an entity manifest |
| [\`px auth\`](docs/generated/commands/auth.md) | Manage secure authentication for the configured Lore provider |
| [\`px branch\`](docs/generated/commands/branch.md) | Create or list branches |
| [\`px commit\`](docs/generated/commands/commit.md) | Commit all repository changes, or only one entity when given its URI |
| [\`px configure\`](docs/generated/commands/configure.md) | Configure version-control backend |
| [\`px content-hash\`](docs/generated/commands/content-hash.md) | Compute the BLAKE3 content hash of a file |
| [\`px create\`](docs/generated/commands/create.md) | Create a new entity manifest |
| [\`px diff\`](docs/generated/commands/diff.md) | Show a manifest diff for an entity URI |
| [\`px doctor\`](docs/generated/commands/doctor.md) | Run diagnostics and repair |
| [\`px head\`](docs/generated/commands/head.md) | Show the current HEAD commit hash |
| [\`px history\`](docs/generated/commands/history.md) | View commit history for an entity or repository file |
| [\`px init\`](docs/generated/commands/init.md) | Initialize a repository repository and/or configure the backend provider |
| [\`px install\`](docs/generated/commands/install.md) | Install required dependencies |
| [\`px list\`](docs/generated/commands/list.md) | List repositories or entities within a repository |
| [\`px merge\`](docs/generated/commands/merge.md) | Three-way merge of JSON/YAML values |
| [\`px presign\`](docs/generated/commands/presign.md) | Create a time-limited public URL for a committed representation |
| [\`px pull\`](docs/generated/commands/pull.md) | Clone or pull PX manifests from a remote (representation files stay remote) |
| [\`px push\`](docs/generated/commands/push.md) | Push the current branch to its configured upstream remote |
| [\`px remote\`](docs/generated/commands/remote.md) | Manage remotes on a repository |
| [\`px resolve\`](docs/generated/commands/resolve.md) | Resolve a PX URI to its manifest or a subtree |
| [\`px revert\`](docs/generated/commands/revert.md) | Revert a commit by hash (undoes all changes in that commit) |
| [\`px schema\`](docs/generated/commands/schema.md) | Print a JSON Schema for manifest or commit types |
| [\`px set\`](docs/generated/commands/set.md) | Set one or more properties on an entity manifest |
| [\`px status\`](docs/generated/commands/status.md) | Show system status, or working-tree status for one repository |
| [\`px switch\`](docs/generated/commands/switch.md) | Switch to a branch |
| [\`px sync\`](docs/generated/commands/sync.md) | Fetch remote manifests and push local commits |
| [\`px unset\`](docs/generated/commands/unset.md) | Remove one or more properties or representations from an entity manifest |
| [\`px validate\`](docs/generated/commands/validate.md) | Validate a manifest against the PX schema |


## Global Options

| Flag | Description | Default |
|---|---|---|
|     --local | Resolve repository reads from an explicitly checked-out local working tree |  |
|     --remote | Resolve repository reads through the configured Lore server (the default) |  |
| -d, --base-dir | Base directory for repository repositories. Defaults to $PX\_DIR, or ~/.px if unset |  |
| -v, --verbose | Enable verbose debug logging |  |


## Output Formats
Most commands support `--format` (`-f`) with values `yaml` (default) or `json`.

When stdout is not a terminal, JSON is used automatically. Override with `$PX_OUTPUT`.


## Common Examples
```bash
# Initialize a repository
px init toystory

# Create an entity
px create character woody -u toystory -n "Woody"

# Resolve a manifest
px resolve px://toystory/character/woody

# Query a subtree
px query px://toystory/character/woody properties

# View commit history
px history px://toystory/character/woody
```




# Global Options
These options are available on all `px` commands.


| Flag | Description | Default |
|---|---|---|
|     --local | Resolve repository reads from an explicitly checked-out local working tree |  |
|     --remote | Resolve repository reads through the configured Lore server (the default) |  |
| -d, --base-dir | Base directory for repository repositories. Defaults to $PX\_DIR, or ~/.px if unset |  |
| -v, --verbose | Enable verbose debug logging |  |




# Environment Variables
The following environment variables are recognized by `px`.


| Variable | Description |
|---|---|
| PX\_OUTPUT | Override for --format |
