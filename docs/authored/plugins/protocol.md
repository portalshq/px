# PX Plugin Protocol v1

The CLI launches one process per tool call:

```text
<entrypoint> --px-plugin-protocol 1 call <tool>
```

The process reads one JSON object from standard input:

```json
{"input":{},"context":{"base_dir":"/home/user/.px","repository":"demo","branch":"story","session":"optional-session","px_binary":"/absolute/path/to/px"}}
```

`repository`, `branch`, `session`, and `px_binary` are optional. The CLI supplies its own absolute executable path as `px_binary` so a plugin can delegate a mutation to a core command. Additional context fields may be added without changing the protocol version. Paid hosts may also supply an `entitlement` object after host verification.

Standard output contains exactly one response:

```json
{"output":{"result":"value"}}
```

or:

```json
{"error":{"code":"invalid_request","message":"explanation"}}
```

Diagnostics belong on standard error. The host validates tool inputs against the manifest, checks compatibility and enablement before execution, and applies the manifest deadline with bounded output and kill-on-drop. `uv-python` packages run through `uv run --locked --project <directory> python <entrypoint>` with the same envelope.

Host-completed model tasks are returned in the plugin's output as `status: llm_task`, a session ID, a single-use task ID, messages, and a response schema. Complete them with the host model and call the plugin's submit tool. This protocol does not require API keys. An invalid result leaves the task pending; a successful submission advances the persisted session atomically.
