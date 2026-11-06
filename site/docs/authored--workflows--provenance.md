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
