---
generated: "true"
generator: px-docgen
version: 0.9.10
source: clap
---


# px sync
Fetch remote manifests and push local commits


## Synopsis
```bash
px sync [OPTIONS] <REPOSITORY>
```


## Arguments

| Name | Description | Required |
|---|---|---|
| repository | Repository name | Yes |


## Flags

| Flag | Description |
|---|---|
|     --force | Force push even if branch already exists on remote (bypass fast-forward check) |
|     --skip-missing | Skip files with missing content addresses (e.g., stale .DS\_Store from historical commits) |
| -h, --help | Print help |


## Source
`crates/px-cli/src/main.rs` — `sync` command

