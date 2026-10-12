---
generated: "true"
generator: px-docgen
version: 0.9.4
source: mcp
---


# px_push
Push the current branch to its configured upstream remote


## Parameters

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| branch | string | No |  | Branch to push (default: current branch) |
| force | boolean | No | false | Force push even if branch already exists on remote (bypass fast-forward check) |
| remote | string | No | origin | Remote name (default: tracking branch's remote, or "origin") |
| repository | string | Yes |  | Repository name |

