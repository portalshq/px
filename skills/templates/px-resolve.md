---
name: px-resolve
description: Create PX entities, resolve PX URIs, and query entity context via px-mcp-server (px_create, px_resolve), establishing active entity continuity so later refinements automatically persist through px-update. The px CLI is not available for agentic use.
---

# PX Resolve

Use this skill to create entities, resolve PX URIs, and gather entity context for creative workflows.

## When to Apply

Reference these guidelines when:

- Creating new entities (e.g., characters, locations, items, events)
- Resolving PX URIs into manifests
- Querying subtree data for creative workflows

For revising entity content and persisting iterations, use `px-update`. For repository-level init/pull/branch, use `px-repo`. For questions about `px` CLI syntax from humans, use `ask-px` (read-only; never execute CLI commands).

## Source of Truth and Approved Changes

The PX repository is the ground truth workspace for the task. Before using existing assets, references, or entities, agents **MUST resolve their current state through PX** on the appropriate branch. Do not use assets saved on the filesystem as an independent source of truth. A PX-resolved asset may be materialized temporarily on the filesystem when a tool requires a local file.

Draft revisions may be committed and pushed to the revision branch as described in `px-update`. Before promoting changes to the target branch, confirm the version and destination with the user; explicit acceptance already given in the conversation counts. Always push approved changes to the target branch, verify the result through PX, and report any push failure accurately.

{{include docs/authored/mcp/overview.md}}

{{include docs/authored/primitives.md}}

{{include docs/authored/workflows/repository-stewardship.md}}

{{include docs/authored/workflows/target-branch.md}}

{{include docs/authored/workflows/continuity.md}}

{{include docs/authored/workflows/resolve-workflow.md}}

{{include docs/generated/mcp/px_create.md}}

{{include docs/generated/mcp/px_resolve.md}}
