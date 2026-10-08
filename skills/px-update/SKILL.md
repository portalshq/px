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

## Provenance Checklist

Before any generation:

- Resolve the project and entity from their explicit branches, including existing reference representations.
- Check whether each input file is already tracked in PX by its BLAKE3 hash. If absent, add it as a reference representation on the active revision branch with `px_add`; use a stable key such as `reference_front`. Hash the actual bytes with `px_content_hash`, and verify the committed representation. Do not rely on a filename, a temporary URL, or a SHA-256 digest as identity.
- Keep iterations on the same entity URI. A new output or prompt is a new revision, not a new entity. Create distinct entities only for requested identity variants.

With each generated asset, record:

- Exact prompt text, including negative prompts; save prompt text as a UTF-8 representation such as `character_sheet_prompt`, with its BLAKE3 hash.
- Provider and exact model identifier returned by the generation tool. Record an unknown identifier as unavailable; never invent a model/version.
- All supplied generation parameters (seed, size, aspect ratio, guidance, steps, references, and tool-specific settings). Do not guess hidden defaults. Identify non-default values under `model_parameters`.
- Every input's PX URI, revision, representation key, and BLAKE3 file hash, including project style references. Obtain the immutable Lore commit with `px_head(repository, branch)` when pinning an input; a PX mutation summary ID is not a Lore commit selector.
- UTC timestamp, generation job ID when available, and per-asset `date_created` and `date_updated`. Preserve `date_created` for the stable representation key; advance `date_updated` for each new output.

Store structured per-asset details under `metadata.generation.<representation_key>` when they do not fit the supported `provenance` fields. Keep provider/model, `prompt_text`, `prompt_hash`, `parameters`, `model_parameters`, `inputs`, timestamps, and `job_id` there. Save the exact prompt as an addressable representation as well. A prompt hash alone cannot recover the prompt. Use `px_set` with an explicitly prefixed top-level key and a JSON-encoded value, for example `values: ["metadata.generation.character_sheet", "{...}"]`. Multiple key/value pairs can be saved together in that array; each JSON value remains one string. Bare keys are narrative properties. `px_set` accepts `metadata`, `provenance`, `references`, and `representations` prefixes and auto-commits/pushes on the current branch. Verify the active branch first. For direct manifest-file edits, use one `px_commit`, then `px_push`.

After saving, resolve the explicit revision branch (or its pinned commit, never both selectors together) and verify the output representation, prompt representation/text, provenance fields, and input/output BLAKE3 hashes against the files actually used. Remote provenance hydration may be unsupported: verify the manifest's metadata and representation hashes with ordinary branch resolution; inspect committed prompt bytes through the available file tools. Do not claim provenance is verified if a required field or artifact is inaccessible.


## When to Apply

Reference these guidelines when:

- Making changes to entity properties in a workflow
- Generating new assets as part of a workflow
- Storing assets back into the entity manifest
- Refining, regenerating, selecting, rejecting, endorsing, or promoting an entity whose PX URI was established earlier in the task
For creating or first resolving entities, use `px-resolve`. For repository-level init/pull/branch, use `px-repo`. For questions about `px` CLI syntax from humans, use `ask-px` (read-only; never execute CLI commands).

## MCP Server (mandatory for agents)

Agents MUST issue all PX operations exclusively through the `px-mcp-server` MCP tools (`px_<command>`). The `px` CLI is NOT available for agentic use — never shell out, never follow `px ...` shell examples.

The standard PX installer bundles the native `px-mcp-server` binary with `px`. If the MCP server is missing or broken, rerun the standard PX installer from a host shell.

The MCP server is not a daemon; agent clients start it on demand over stdio (see `docs/authored/mcp/install.md` for client configuration), and it proxies tool calls to the host PX installation.

## Available MCP Tools

Every PX command is available as an MCP tool with a `px_` prefix and dashes/spaces converted to underscores. For example:

- `px_resolve` — resolve a PX URI to its manifest or a subtree
- `px_create` — create a new entity manifest
- `px_resolve` with `path` — query a subtree from a manifest
- `px_set` — set a property on an entity manifest
- `px_add` — add a file representation to an entity manifest
- `px_commit` — commit changes to a repository

Per-tool parameters are documented under `docs/generated/mcp/<tool>.md` (generated from the same command definitions as the MCP server itself). Tool arguments use MCP field names (for example `branch`, `commit`, `format`, `include_blobs`); pass the branch explicitly on every call that accepts one — do not rely on whichever branch happens to be checked out. The CLI reference (`docs/generated/cli.md`, `docs/generated/commands/`) and shell examples elsewhere in these docs are for humans in host shells only and MUST NOT be used as agent instructions.


## Repository Context (`repository.yaml`)

`repository.yaml` is the repository's manifest and the sole source of truth for project-wide context. It owns durable global visual or narrative style, reusable asset conventions, global exclusions, and canonical references through its `properties`, `representations`, and `references`.

Before creating an entity or generating content, resolve the repository.yaml from the same target branch as the entity work (via `px_resolve` on the repository URI). Read its `properties`, `representations`, and `references`, and resolve only the referenced resources relevant to the requested work. Treat repository context as the project-wide baseline; entity-specific identity, behavior, and representation data apply as refinements. Keep entity manifests focused on identity and entity-specific facts; do not add a repository-level summary or reference for every entity.

If project and entity instructions truly conflict, pause and ask the user for direction rather than choosing one. If the repository.yaml cannot be read, warn the user and ask how to proceed; never silently generate without project context.

Update `repository.yaml` only when the user explicitly defines or approves a project-wide property or reference. If entity work reveals a potentially reusable global fact, present it as a proposed repository update and wait for user approval before writing it. Preserve existing global context when adding an approved change. Never promote inferred project-wide facts into `repository.yaml` as part of an entity update.

Unless the user explicitly requests a different provider or storage location, call `px_init` with no provider argument and preserve the configured provider and default PX directory. Never infer a local provider from an example. Never choose an isolated storage location merely to isolate a repository. Pass a provider only when the user explicitly requests a provider change, and a storage location only when the user explicitly names one.

**No tagging.** Lore VCS has no native tag support — branches are the only mechanism for human-readable names on a revision point. Do not append tags to URIs.

Checking the current workspace is not required: PX usually stores all repositories in a centralized directory unless configured otherwise.


## Target Branch

The **target branch** is whichever branch the entity's accepted work is meant to land on. It is **not always `main`** — resolve it per task, in this order:

1. If the user named a branch for this work (e.g., "we're doing this on the `classic` branch"), that branch is the target.
2. Otherwise, the branch the entity was created on or first resolved from is the target.
3. Otherwise, resolve the repository's configured default branch (and the provider/global default if needed). Do not assume it is `main`.

Establish the target branch at creation/first-resolve time and carry it forward for the rest of the task. Every promotion promotes to the **resolved target branch**, never a hardcoded `main`. When reporting or asking about promotion, name the target branch explicitly (e.g., "promote to `classic`") rather than saying "main" generically.


## Active Entity Continuity

Once a PX entity URI is established in a task, carry forward:

- the active URI, repository, entity type, and entity ID
- the active revision branch and the **target branch** (see `target-branch.md`)
- stable representation keys such as `character_sheet`, `face_sheet`, `portrait`, `model_sheet`, or `reference_image`
- user-approved identity constraints and negative constraints

Later turns that keep refining the same entity are continuity work. They must trigger `px-update` even when the user does not mention PX again and does not say "save" or "commit" or repeat the URI. Carry forward stable representation keys and identity constraints so attributes do not get dropped between turns.


## Revision Branches

Use a revision branch for ordinary iteration:

```text
revision-<entity-type>-<entity-id>
```

Example: `revision-character-atlas`.

Commit every accepted user-visible iteration to the revision branch in the same turn. An accepted iteration means a concrete candidate the agent generated or revised in response to the user's current refinement request, unless the user explicitly said not to save it, asked only to brainstorm, or the generation failed. Multiple tiny corrections in one turn may be grouped into one accepted-revision commit; do not wait across many turns to batch revisions.

Rejected or superseded versions should remain traceable in branch history. Do not delete historical assets or replace history to make a rejected variant disappear.

If the user selects an older variant, restore that content as a new commit at the tip of the revision branch rather than rewriting history. If a reference such as "#2" is ambiguous or stale, resolve the mapping from the visible candidate set or ask a brief clarification before persisting the selection.


## Promotion to the Target Branch

Saving a generated candidate on a revision branch does not accept it as canonical. **Promotion requires explicit user acceptance of the version and destination.** A direct request to promote, merge, make canonical, or use this version going forward counts; so does a clear yes to a promotion question. Do not ask again when acceptance is already established. Silence, general thanks, or changing tasks is not acceptance.

After verifying a saved revision, ask briefly whether to promote to the named target branch or keep iterating. At a milestone, one question can cover all outstanding revisions. Continue independent work while a promotion decision is pending. If a downstream task needs an unaccepted revision, resolve which version the user intends before promoting it; do not auto-promote merely because it is a dependency.

## Executing a Promotion

1. Resolve the accepted revision branch and record its commit and representation hashes.
2. Switch to the resolved target branch and check for unrelated dirty content. If applying the revision would overwrite it, report the conflict and preserve the user's work.
3. Apply the accepted representations, properties, references, and provenance as a new target-branch commit. Do not rewrite history or replace unrelated entities.
4. Commit with a message naming the source branch and commit, then push the target branch. Auto-committing tools do not require an extra commit.
5. Resolve the target branch and verify the accepted representation, prompt, reference, and provenance hashes match. Report success only after verification; if push failed, report the local commit and recovery command.

## Reporting

Saved: `PX: <URI> saved on <revision-branch> at <commit>; <key> verified. Promote to <target-branch>, or keep iterating?`

Promoted: `PX: <URI> promoted to <target-branch> at <commit> from <revision-branch> <source-commit>; hashes verified.`

Failed: `PX persistence failed: <URI> generated but not committed` (or `committed locally; push failed`, according to the actual state).


## Update Pipeline

1. Resolve the target branch and project context from `repository.yaml`.
2. If the revision branch is absent, switch to the target first and create the revision branch there. Switch to the revision branch. Publish a newly created branch with `px_push` before resolving it remotely. Verify the current branch before mutating; every tool that supports `branch` receives it explicitly.
3. Resolve the same entity URI from the active revision branch via `px_resolve` and gather relevant properties, representations, references, and exclusions.
4. Follow the Provenance Checklist: add untracked inputs as reference representations before generation; verify their BLAKE3 hashes.
5. Generate or edit the content using that context, preserving the entity URI and stable representation keys.
6. Persist output, exact prompts, input hashes, and generation metadata in the same turn. `px_add` and `px_set` auto-commit and push. For structured manifest changes, call `px_commit` once, then `px_push` for that branch; commit alone does not publish.
7. Resolve the explicit revision branch and verify the saved assets, prompt text, provenance, and BLAKE3 hashes. If saving or pushing failed, report the actual state and recovery action; do not promote or claim remote persistence.
8. Report the saved URI, branch, and revision. Promote only with explicit acceptance (see Promotion to the Target Branch above); name the resolved target branch.

## Data Placement

- `properties` — narrative facts and durable identity constraints.
- `representations` — current addressable assets, under stable semantic keys (e.g., `character_sheet` across all of Atlas's character-sheet revisions).
- Commit messages — revision notes (`revision_summary`). Do not create an append-only `revision_log` property; PX/Lore history is the revision log.
- `metadata` — extension data that isn't narrative canon (e.g., per-representation provenance the manifest format can't otherwise express).

## Branch and Version Truth

Branch heads and commit history are VCS state — do not store or repair them inside entity manifests. Treat any branch-head data a PX tool exposes as derived status only.

Do not use commit `parent` fields as validation truth unless the current PX/Lore version documents them as reliable. Count commit IDs/messages and verify branch-specific resolves instead.

Some PX tools auto-commit (`px_add`, `px_set`). Do not call `px_commit` afterward unless you intentionally made additional uncommitted changes.

Note: `px_add`, `px_set`, `px_create`, and `px_unset` stage, commit, and push automatically. `px_resolve` reads the configured server by default, so successful mutations are immediately visible to other clients. If a push fails, the commit remains local and PX reports the recovery command. For local inspection of uncommitted changes, use available file/status tools. Agents must use MCP for PX operations; the local CLI mode described in `ask-px` is for humans.

## Character Creation and Character Sheets

Treat a request to create a character, character sheet, reference sheet, or other character visual as a persistence workflow. Do not leave generated assets only in the conversation or on a local filesystem.

1. Create or resolve the character entity on its target branch (use `px-resolve`), then create/switch to its revision branch before adding generated assets.
2. Add every accepted generated asset with a stable semantic representation key via `px_add`. Use `character_sheet` for a complete three-view reference sheet and `portrait` for a portrait.
3. Commit the representation and any properties that describe the accepted character in the same turn (`px_add` and `px_set` may commit automatically; otherwise run one `px_commit` after updating the manifest and push the revision branch).
4. Resolve the entity from the branch that received the commit and confirm its manifest contains the expected representation key, URI, format, and BLAKE3 hash.

Keep the manifest current whenever a character is created or a character sheet is generated. A file is not complete until it is represented in the entity manifest and committed to PX.

## Entity Variants

When the user asks to create a variant of an existing entity, create a new entity. Never replace the base entity's canonical identity or overwrite its representations to impersonate a variant.

1. Resolve the base entity and its target branch.
2. Create a distinct entity ID that describes the variant via `px_create`, for example `claire-cole-summer-dress` for `px://25th-chapter/character/claire-cole-summer-dress`.
3. Persist the variant's applicable properties and representations via `px_set` / `px_add`; these tools auto-commit and push. Commit separately only for additional direct file edits.
4. Update the base entity's `properties.variants` value to the complete, deduplicated list of PX URIs for every known variant, including the new URI. Preserve existing entries; do not record local paths or bare IDs.
5. Resolve both entities after the auto-committed base-manifest update to verify the variant and the base entity's `variants` property.

For example, the base entity `px://25th-chapter/character/claire-cole` keeps:

```yaml
properties:
  variants:
    - px://25th-chapter/character/claire-cole-summer-dress
    - px://25th-chapter/character/claire-cole-riot-gear
```

Use the same target branch for the base update and variant unless the user explicitly asks for a different branch. Report both committed entity URIs and their revisions.



# px_resolve
Resolve a PX URI to its manifest or a subtree


## Parameters

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| branch | string | No |  | Resolve at a specific branch |
| commit | string | No |  | Resolve at a specific commit hash |
| format | string | No | yaml | Output format: yaml, json |
| include\_blobs | boolean | No | false | Hydrate known readable provenance artifacts such as prompts and run records |
| path | string | No |  | Optional manifest subtree selector. URI fragments take precedence |
| provenance | boolean | No | false | Include condensed per-file provenance for the manifest and direct representations |
| uri | string | Yes |  | PX URI. e.g., "px://toystory/character/woody" |




# px_add
Add a file representation to an entity manifest


## Parameters

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| author | string | No | px | Author identifier |
| file | string | Yes |  | File path to the asset |
| format | string | Yes |  | Asset format. e.g., "png", "glb" |
| key | string | Yes |  | Representation key. e.g., "reference\_image" |
| message | string | No |  | Commit message |
| replace | boolean | No | false | Replace an existing representation only when its content differs |
| uri | string | Yes |  | PX URI |




# px_set
Set one or more properties on an entity manifest


## Parameters

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| author | string | No | px | Author identifier |
| message | string | No |  | Commit message |
| uri | string | Yes |  | PX URI |
| values | string or string[] | Yes |  | Repeating key/value pairs. Keys support dot-notation |




# px_commit
Commit all repository changes, or only one entity when given its URI


## Parameters

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| author | string | No | px | Author identifier |
| message | string | Yes |  | Commit message |
| target | string | Yes |  | Repository name or PX entity URI |




# px_content_hash
Compute the BLAKE3 content hash of a file


## Parameters

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| file | string | Yes |  | Path to the file to hash |




# px_switch
Switch to a branch


## Parameters

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| name | string | Yes |  | Branch name to switch to |
| repository | string | Yes |  | Repository name |




# px_branch
Create or list branches


## Parameters

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| name | string | No |  | Branch name to create. Omit to list local branches |
| repository | string | Yes |  | Repository name |




# px_status
Show system status, or working-tree status for one repository


## Parameters

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| repository | string | No |  | Repository name |




# px_head
Show the current HEAD commit hash


## Parameters

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| branch | string | No |  | Read the head of a specific branch |
| repository | string | Yes |  | Repository name |




# px_push
Push the current branch to its configured upstream remote


## Parameters

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| branch | string | No |  | Branch to push (default: current branch) |
| remote | string | No | origin | Remote name (default: tracking branch's remote, or "origin") |
| repository | string | Yes |  | Repository name |




# px_presign
Create a time-limited public URL for a committed representation


## Parameters

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| branch | string | No |  | Resolve at a specific branch |
| commit | string | No |  | Resolve at a specific commit hash |
| download | string | No |  | Download the representation after creating its presigned URL. Optionally set its destination |
| http\_url | string | No |  | Explicit Lore HTTP origin, such as http://127.0.0.1:41339 |
| output | string | No |  | Destination for --download. Defaults to the entity asset directory |
| representation | string | Yes |  | Representation name (manifest key), e.g. item. Its URI is relative to the entity's asset directory |
| token\_env | string | No |  | Environment variable containing a repository-scoped bearer token |
| ttl\_seconds | string | No |  | Requested lifetime in seconds; Lore enforces its configured bounds |
| uri | string | Yes |  | Entity ID, e.g. 25th-chapter/character/nathan-gunn. The px:// prefix is optional; fragments are not supported |
