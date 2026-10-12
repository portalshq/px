---
generated: "true"
generator: px-docgen
version: 0.9.5
source: mcp
---


# px_pull
Clone or pull PX manifests from a remote (representation files stay remote)


## Parameters

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| skip\_missing | boolean | No | false | Skip files with missing content addresses (e.g., stale .DS\_Store from historical commits) |
| url\_or\_name | string | Yes |  | URL (clone) or repository name (pull existing) |

