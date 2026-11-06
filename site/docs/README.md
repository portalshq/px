# PX documentation demo

Preview the site from the repository root with `python3 -m http.server 8000 --directory site`, then visit <http://localhost:8000/docs/>. This is a static demo; it does not deploy anything.

The demo renders public Markdown from `docs/authored/`, `docs/examples/`, and `docs/generated/`, including the generated CLI and MCP references. The internal-only references are excluded. After changing those sources, run `python3 scripts/build-docs-demo.py` to refresh `content.js`. The generator requires Pandoc. Each page has a static `.html` URL and a matching `.md` file, so navigation and Markdown access work under `/px/docs/` without server-side routing.
