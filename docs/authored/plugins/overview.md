# PX plugins

PX plugins provide additional tools and harness skills. Use `px plugin` to install, list, inspect, enable, disable, and diagnose plugins; use `px schema plugin` to inspect the manifest schema. `px license` manages offline entitlements. `px record` records generated outputs and provenance.

The MCP gateway discovers enabled plugin tools at startup. Restart it after changing plugins. If discovery fails, core tools remain available. See [protocol](protocol.md), [manifests](manifest.md), [harnesses](harnesses.md), and [commerce](commerce.md).
