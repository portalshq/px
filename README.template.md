# {{crate_name}} — PX protocol

{{include docs/authored/introduction.md}}

---

{{include docs/authored/installation.md}}

---

{{include docs/authored/primitives.md}}

---

{{include docs/authored/primitives-examples.md}}

---

## Representation downloads and presigned URLs

Create a temporary download URL with
`px presign 25th-chapter/character/nathan-gunn item`.
See the [px presign reference](docs/generated/commands/presign.md) for entity
and representation arguments, file lookup, configuration, and SDK examples.

---

{{include docs/authored/mcp/overview.md}}

{{include docs/authored/mcp/install.md}}

---

## Plugins and optimizer

The free `px.optimizer` plugin assembles pinned project context, runs a host-completed critic, and compiles generation prompts through a DSPy state runtime. The head agent sends the exact prompt to the generation model, then records outputs with `px record`.

Use `px plugin list`, `info`, `enable`, `disable`, and `doctor` to manage installed packages. See the [plugin overview](docs/authored/plugins/overview.md). Optimizer distribution is being migrated to a separately versioned release.

## Training

The development-side training plugin is maintained separately from PX. Training requires a reviewed dataset, an approved spending limit, and the plugin’s own development environment.

---

{{include docs/generated/cli.md}}

---

{{include docs/authored/http-api.md}}

---

{{include docs/generated/command-index.md}}

---

{{include docs/authored/design-principles.md}}

---

## Status

This is a v0 prototype. APIs and formats may change.

## License

[PolyForm Shield 1.0.0](LICENSE) (`{{license}}`). Previously published MIT releases remain MIT.
