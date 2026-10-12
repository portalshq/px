---
generated: "true"
generator: px-docgen
version: 0.9.2
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
|     --skip-missing | Skip files with missing content addresses (e.g., stale .DS\_Store from historical commits) |
| -h, --help | Print help |


## Source
`crates/px-cli/src/main.rs` — `sync` command

