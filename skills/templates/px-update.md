---
name: px-update
description: Persist changes to existing PX entities via px-mcp-server (px_add, px_set, px_commit), including narrative property updates and every user-visible creative iteration, and manage promotion of accepted revisions to the current working branch. The px CLI is not available for agentic use. Use whenever a user refines, regenerates, selects, rejects, endorses, or promotes an entity whose PX URI was established earlier in the task, even if the user does not mention PX again.
---

# PX Update

Use this skill whenever existing PX entity content changes, especially during iterative creative work.

## Quick Reference: Create → Revise → Save → Verify → Promote

1. Establish the existing entity URI and target branch from task context; never assume the target is `main`. For resolve/presign, supply **either** `branch` for the moving branch head **or** `commit` for a pinned revision; never send both.
2. Resolve project and entity context. If `revision-<entity-type>-<entity-id>` does not exist, switch to the target branch first and create it there; switch to the revision branch and publish it with `px_push(repository, branch)` before remote resolution. If it already exists, switch to it. Verify `px_status(repository)` reports the intended `current_branch` before any mutation.
3. Track all reference inputs and record their BLAKE3 hashes before generation. Follow the Provenance Checklist below.
4. Generate, add representations and provenance to the same entity, then commit and push the revision in this turn. Tools that auto-commit/push already satisfy this step; do not commit twice.
5. Pin the saved revision with `px_head(repository, branch)` and resolve using that returned `commit` alone. Mutation summaries may show PX metadata IDs; do not treat those as Lore commit selectors. Verify representations, prompt text, metadata, and hashes. A push failure means the revision is local only: report the recovery command and retry only when the underlying issue is resolved.
6. Promote the verified revision only after explicit user acceptance, to the resolved target branch. Verify the target's hashes match the accepted revision. Otherwise keep iterating on the revision branch.

{{include docs/authored/workflows/provenance.md}}

## When to Apply

Reference these guidelines when:

- Making changes to entity properties in a workflow
- Generating new assets as part of a workflow
- Storing assets back into the entity manifest
- Refining, regenerating, selecting, rejecting, endorsing, or promoting an entity whose PX URI was established earlier in the task
For creating or first resolving entities, use `px-resolve`. For repository-level init/pull/branch, use `px-repo`. For questions about `px` CLI syntax from humans, use `ask-px` (read-only; never execute CLI commands).

{{include docs/authored/mcp/overview.md}}

{{include docs/authored/workflows/repository-stewardship.md}}

{{include docs/authored/workflows/target-branch.md}}

{{include docs/authored/workflows/continuity.md}}

{{include docs/authored/workflows/revision-branches.md}}

{{include docs/authored/workflows/promotion.md}}

{{include docs/authored/workflows/update-workflow.md}}

{{include docs/generated/mcp/px_resolve.md}}

{{include docs/generated/mcp/px_add.md}}

{{include docs/generated/mcp/px_set.md}}

{{include docs/generated/mcp/px_commit.md}}

{{include docs/generated/mcp/px_content_hash.md}}

{{include docs/generated/mcp/px_switch.md}}

{{include docs/generated/mcp/px_branch.md}}

{{include docs/generated/mcp/px_status.md}}

{{include docs/generated/mcp/px_head.md}}

{{include docs/generated/mcp/px_push.md}}

{{include docs/generated/mcp/px_presign.md}}
