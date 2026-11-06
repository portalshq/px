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
