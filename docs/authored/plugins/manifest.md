# Plugin manifests

Publish the manifest schema with `px schema plugin`. A package contains `plugin.yaml`, platform entrypoints, and any declared skill directories. Plugin IDs use reverse-DNS components. Namespaces are unique and cannot collide with core command namespaces; tool names are bounded and validated before installation.

```yaml
id: example.hello
namespace: hello
version: 0.1.0
px: ">=0.10, <0.11"
license: PolyForm-Shield-1.0.0
kind: executable
entrypoint:
  aarch64-apple-darwin: bin/hello
capabilities: [px-read]
tools:
  - name: greet
    description: Return a greeting.
    input_schema:
      type: object
      properties: {name: {type: string}}
      required: [name]
    annotations: {readOnlyHint: true}
    timeout_ms: 5000
skills: []
hooks: []
commerce: {model: free}
```

`uv-python` uses a Python entrypoint inside a project with a locked dependency graph. `px-write` and builtin package kinds are reserved: plugins delegate mutations to core commands. Hook declarations describe harness-neutral events; v1 prints hook instructions and does not edit harness settings.

Commerce models are `free`, `one_time` with product and major version, and `usage` with meter names. Payments and hosting are deferred; paid packages require valid offline tokens against administratively pinned keys.
