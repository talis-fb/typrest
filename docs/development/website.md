# Website development guide

- **Status:** Current
- **Scope:** The public site in `website/`: landing pages, the examples gallery, and the docs at
  `/docs`, served at `https://typrest.talison.dev`. The brand rules it follows are in
  [`brand/README.md`](../../brand/README.md).

## Overview

`website/` is a static [Astro](https://astro.build/) project. The landing pages (`/`,
`/examples/`, and the 404 page) are adapted from the AstroWind template and live in
`website/src/pages/`. [Starlight](https://starlight.astro.build/) serves everything under `/docs`
from `website/src/content/docs/docs/`. Both use daisyUI 5 with the brand kit's `light` and `dark`
themes.

| Path | Holds |
|---|---|
| `website/src/config.ts` | Site URL, repository URL, analytics token, and the `SITE_INDEXABLE` flag |
| `website/astro.config.mjs` | Astro and Starlight configuration, sidebar, and head tags |
| `website/src/pages/` | Landing pages; `robots.txt` is generated from `SITE_INDEXABLE` |
| `website/src/content/docs/docs/` | Hand-written docs pages (`index.mdx`, `getting-started.mdx`) and the generated folders |
| `website/src/lib/examples.ts` | Builds the examples gallery from `examples/README.md` |
| `website/scripts/` | `sync-docs.mjs`, `sync-brand.mjs`, and `capture-webui.mjs` |
| `website/public/examples/` | Rendered example PDFs, committed |
| `website/src/assets/` | Example previews, their `manifest.json`, and the Web UI screenshot, committed |

Parts of the layouts, widgets, and `landing.css` are adapted from AstroWind; its MIT license is in
`website/ASTROWIND-LICENSE`.

## Running the site

The site needs Node.js 22.23.4 or newer and no Rust toolchain.

```bash
cd website
npm ci
npm run dev       # http://localhost:4321
npm run check     # astro check
npm run build     # dist/, with link validation and the search index
npm run preview   # serves dist/
```

`dev`, `check`, and `build` first run `npm run sync`, which runs the two sync scripts below. The
build fails when a link in the docs is broken (`starlight-links-validator`), so a change under
`docs/` that breaks a link also fails the CI `website` job.

Two build warnings are expected: astro-icon finds no local `src/icons/` folder (the site uses
Tabler icons only), and Starlight's optional `i18n` collection is empty.

## Docs sync

`website/scripts/sync-docs.mjs` publishes `docs/specs/*.md` and `docs/architecture/*.md`, except
the specification template and index files. It never edits `docs/`; the canonical text stays
there.

- The H1 becomes the frontmatter `title`, the `Scope` line becomes `description`, and the
  Status/Scope list is dropped from the page body.
- Links between published pages become site routes, such as `/docs/specs/http-api/#errors`.
- Links to anything else in the repository (ADRs, plans, guides, the product vision, source files)
  become GitHub URLs on the `main` branch.
- `REQ-*` IDs and all other text stay as written.

Output goes to `website/src/content/docs/docs/specs/` and `.../architecture/`, which are
gitignored. To publish another folder, add it to `SECTIONS` in the script and to the sidebar in
`astro.config.mjs`. Plans and ADRs are deliberately not published.

## Brand sync

`website/scripts/sync-brand.mjs` copies the favicons, touch and manifest icons, and the social
card from `brand/` into `website/public/`. The copies are gitignored; `brand/` is the only source.
The logos and stylesheets are imported from `brand/` directly. `vite.server.fs.allow` includes
`../brand` so these imports work in `astro dev`.

## How the site applies the brand

The rules themselves are in [`brand/README.md`](../../brand/README.md). The site implements them
this way.

### Stylesheets

| File | Loaded by | Contents |
|---|---|---|
| `website/src/styles/fonts.css` | Every page, first | Fontsource Fraunces (the `opsz` entry, with optical size and weight axes), IBM Plex Sans 400, 400 italic, 500, 600, and IBM Plex Mono 400 |
| `website/src/styles/global.css` | Every page | Tailwind theme and utilities, `@astrojs/starlight-tailwind`, daisyUI, `brand/daisyui/typrest-themes.css`, and the `dark` variant |
| `website/src/styles/landing.css` | Landing pages | Tailwind Preflight, then `global.css`, then AstroWind's utilities and the `type-dot` utility |
| `brand/starlight/typrest.css` | Docs pages, last | Every Starlight color, the display font on headings, and the sidebar type-dot |

Starlight's `customCss` loads `fonts.css`, `global.css`, and `typrest.css` in that order.
`typrest.css` is unlayered, so its colors win over the layered `starlight-tailwind` defaults.

`global.css` follows the `@astrojs/starlight-tailwind` layout: it has no Preflight, which would
reset Starlight's lists and headings, and declares `@layer base, starlight, theme, components,
utilities`. It also excludes `src/content/` from Tailwind's class detection. Without that, words in
the docs such as "card" or "collapse" would generate daisyUI components that also match Starlight
markup. To check for leaks, build once with `global.css` removed from `customCss` and compare docs
pages: they should render identically.

AstroWind's `--aw-*` variables, set in `website/src/components/CustomStyles.astro`, point at
daisyUI tokens and the brand fonts. AstroWind's own `btn` utilities are removed, and the landing
pages use daisyUI buttons: `btn-primary` (ink) for the main action and `btn-outline` or
`btn-ghost` for others.

### Dark mode

Every page uses one mechanism: `data-theme="light"` or `"dark"` on `<html>`, which both daisyUI
and Starlight read. The choice is stored under Starlight's `localStorage` key, `starlight-theme`,
as `light`, `dark`, or an empty string for auto.

- On docs pages, Starlight's own theme provider and select handle it.
- On landing pages, `website/src/components/common/ThemeScript.astro` sets `data-theme` inline in
  `<head>` before first paint, falling back to `prefers-color-scheme`. The header toggle
  (`ToggleTheme.astro`) cycles auto, light, and dark, and writes the same key.

A choice made on `/` therefore carries over to `/docs` and back.

### Shared head tags

`website/src/lib/head.ts` lists the icon, manifest, `theme-color`, and social card tags. The
landing layout renders them, and Starlight receives the same list through its `head` option.
`website/src/components/Robots.astro` and `Analytics.astro` are rendered by both the landing
layout and the Starlight `Head` override (`website/src/components/starlight/Head.astro`).

## Example previews and the Web UI screenshot

The examples gallery reads its list and descriptions from the table in
[`examples/README.md`](../../examples/README.md), which stays the only index of examples. The
previews come from `scripts/render-website-examples.sh`, run by hand:

```bash
scripts/render-website-examples.sh
```

For each example project with a `templates/**/payload.json`, the script starts `typrest serve`,
waits on `/_healthz`, posts each payload, and writes the PDF to `website/public/examples/`, a
first-page PNG to `website/src/assets/examples/`, and `manifest.json`. When Chrome or Chromium is
installed, it also serves the invoice example and captures `website/src/assets/webui.png` with
`website/scripts/capture-webui.mjs`. It needs `curl`, `pdftoppm` and `pdfinfo` (poppler), `python3`,
and `node`, and builds `target/release/typrest` if missing. `packages` and `ticket` download Typst
packages on their first run; `clients` has no templates and is skipped.

The output is committed so site builds need no Rust toolchain. Previews can therefore drift from
the templates: rerun the script and commit the result when a template, payload, or the Web UI
changes. The screenshot shows the Web UI's current styling, so retake it after the Web UI adopts
the brand themes.

## Deployment

The site is deployed to Cloudflare Workers static assets. `website/wrangler.jsonc` serves `dist/`
with the custom domain `typrest.talison.dev` and Astro's `404.html` for unknown paths.

`.github/workflows/website-deploy.yml` builds and runs `wrangler deploy` on a push to `main` that
touches `website/`, `brand/`, `docs/`, or `examples/README.md`, and on manual dispatch. It needs
the `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` repository secrets; the token must be able
to deploy Workers and manage the `talison.dev` custom domain.

To check the configuration without deploying:

```bash
cd website
npm run build
npx wrangler deploy --dry-run
```

The CI `website` job (`.github/workflows/ci.yml`) runs `npm ci`, `npm run check`, and `npm run build`
on every pull request. `.dockerignore` excludes `website/` and `brand/`, so the site never enters
the container build context.

## Search engines

`SITE_INDEXABLE` in `website/src/config.ts` is `false` until the repository is public. While it is
false, every page carries `<meta name="robots" content="noindex, nofollow">` and `robots.txt`
disallows everything. Set it to `true` and redeploy at launch; `robots.txt` then allows crawling
and points to the sitemap.

## Analytics

The site uses Cloudflare Web Analytics. `website/src/components/Analytics.astro` renders the
beacon only in production builds and only when `ANALYTICS_TOKEN` is set in
`website/src/config.ts`. The token is public by design. In the Cloudflare dashboard, the hostname
is set to manual snippet installation, so automatic injection does not count visits twice.

### Limitation: no click tracking

Cloudflare Web Analytics reports visits, page paths, referrers, and countries, but it does not
support custom events. Internal navigation appears only as page views, and clicks on outbound links
such as GitHub are invisible. Ad blockers also block the beacon, which undercounts a developer
audience; Cloudflare's edge request metrics are not blocked and serve as a rough cross-check.
Because analytics lives in one component, moving to a tool with events, such as Umami, changes
only `Analytics.astro` and `config.ts`.

## Why it is built this way

| Choice | Reason |
|---|---|
| Astro with AstroWind for the landing pages and Starlight for the docs | Ready-made sections, plus docs with search, with little custom CSS |
| daisyUI 5 with the brand themes on every page | The brand kit's themes target daisyUI, and the Web UI uses it too, so the site and the Web UI share one set of tokens |
| `brand/` at the repository root as the only copy of the brand kit | One source for the site, the Web UI, and Typst templates |
| One dark-mode mechanism (`data-theme` and the `starlight-theme` key) | The brand themes key on `data-theme`, and a choice made on `/` must carry over to `/docs` |
| Self-hosted fonts through Fontsource | Brand typefaces under the OFL, with no third-party font requests |
| Cloudflare Workers static assets | `talison.dev` DNS is already on Cloudflare, static requests are free, and Cloudflare recommends Workers over Pages for new projects. GitHub Pages on GitHub Free needs a public repository |
| Deploy from GitHub Actions with wrangler | Lives in the repository beside `ci.yml`. Cloudflare's documentation did not confirm Workers Builds for private repositories |
| Cloudflare Web Analytics | Free and built into the host |
| Docs generated from `docs/` | One source of truth: the site never holds a second copy of a specification |
| Pre-rendered example PDFs and a Web UI screenshot, no live demo | A static site has no server to render with, and the Web UI needs a running Typrest |

No ADR records these choices. A static site is portable, so it does not meet the costly-to-reverse
test in the [ADR policy](../adr/README.md), although issue #7 suggested one.
