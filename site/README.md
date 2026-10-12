# PX standalone site

This is the static PX landing page. It includes the image-rich Eye Candy sections,
the Portals footer, and the Saga WebGL canvas. Preview it with `npx serve site`.

The public canonical URL is always `https://portals.works/px`.

## Deployment architecture

- **Content source:** This repo's `site/` directory (not cloud/frontend)
- **Primary origin:** GitHub Pages at `https://portalshq.github.io/px`
- **Public URL:** `https://portals.works/px` (proxied through cloud/frontend for the canonical domain)
- **cloud/frontend role:** Reverse proxy only (rewrites `/px/:path*` to GitHub Pages)

## Content source

All PX landing page content is authored directly in this repo's `site/` directory:

- `index.html` - Main landing page HTML (manually edited)
- `styles.css` - Compiled CSS (see scripts/update-site-content.mjs)
- `script.js` - Clipboard behavior for code blocks
- `saga-webgl.js`, `models/scene.glb` - WebGL 3D assets
- `eye-candy/` - Image references (served from https://portals.works)
- `fonts/*.woff2` - PX font files
- `favicon.svg` - PX favicon
- `.nojekyll` - GitHub Pages marker

**Note:** The cloud/frontend app does NOT contain PX landing page source. It only proxies `/px` to GitHub Pages.

The workflow refreshes the site's technical code blocks at build time on every
commit to `main`. The install command comes from the `Installation Script`
section of `README.md`; CLI and SDK examples come from
`docs/generated/commands/`. The update script only substitutes demo-domain
values into those canonical snippets; command syntax stays owned by the
documentation.
`README.md` is checked as the public documentation mirror for the MCP summary;
a drift between the authored MCP overview and README fails the Pages build.
The published HTML is static after deployment; it does not fetch snippets at
runtime.

Two dead references were intentionally dropped: `styles.card` and
`styles.sagaBannerFrame` have no CSS-module rules, so omitting them is
pixel-identical.

## Deploy

`.github/workflows/pages.yml` refreshes generated CLI content, validates the static
site, and uploads `site/` to GitHub Pages on every push to `main`. Run
`node scripts/update-site-content.mjs` and `node scripts/validate-site.mjs`
locally before pushing.
