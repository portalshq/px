---
generated: "true"
generator: px-docgen
version: 0.9.5
source: mcp
---


# px_sync
Fetch remote manifests and push local commits


## Parameters

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| force | boolean | No | false | Force push even if branch already exists on remote (bypass fast-forward check) |
| repository | string | Yes |  | Repository name |
| skip\_missing | boolean | No | false | Skip files with missing content addresses (e.g., stale .DS\_Store from historical commits) |

