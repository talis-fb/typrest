# Typrest brand kit

Direction "movable type": a lead type block with its nick, a T on its face, and a
brand-red square, the type-dot.

## Contents

| Path | What |
|---|---|
| `logos/typrest-symbol*.svg` | The symbol: color, inverse (for dark backgrounds), mono ink, mono paper |
| `logos/typrest-logo-horizontal*.svg` | Symbol + name, for the site header and README |
| `logos/typrest-logo-stacked*.svg` | Symbol above the name, for covers and square spaces |
| `logos/typrest-wordmark*.svg` | Name + type-dot, when the symbol is already nearby |
| `favicon/favicon.svg` | Favicon that switches to paper in dark mode |
| `favicon/favicon.ico`, `apple-touch-icon.png`, `icon-192.png`, `icon-512.png` | Raster icons |
| `social/social-card.png` | 1280×640 social preview (GitHub, Open Graph) |
| `starlight/typrest.css` | Starlight theme: colors (dark and light), fonts, headings |
| `daisyui/typrest-themes.css` | daisyUI 5 `light` and `dark` themes for the site and the Web UI |
| `typst/theme.typ` | Brand tokens and a `typrest-style` show rule for templates and examples |

All text in the SVG logos is converted to outlines, so no font is needed to display them.

## Colors

| Token | Hex | Use |
|---|---|---|
| ink-900 | `#16130F` | Text, symbol, primary buttons |
| ink-600 | `#6B655C` | Muted text on light backgrounds |
| ink-100 | `#F4EFE6` | Paper |
| ink-50 | `#F9F6F0` | Light page background |
| dark bg | `#1A1714` | Dark page background |
| press-500 | `#D9381E` | Brand red: symbol, type-dot, decoration. Not for body text on paper (4.0:1) |
| press-600 | `#B82C16` | Links and accent text in light mode (5.7:1) |
| press-300 | `#E7765A` | Links and accent text in dark mode (6.1:1) |
| error | `#B3243B` | Errors. Kept apart from brand red on purpose |

## Type

- Fraunces (opsz 48, weight 560, SOFT 0, WONK 0 in the logo): display and headings
- IBM Plex Sans: body
- IBM Plex Mono: code

All three are under the SIL Open Font License.

## Rules

- Keep clear space around the logo equal to the symbol's width ÷ 2.
- Smallest sizes: symbol 16 px, horizontal logo 96 px wide.
- Use the inverse files on backgrounds darker than ink-400.
- Primary actions are ink. Brand red is for links, highlights and the type-dot.
- Do not recolor the type-dot, stretch the symbol, or set the name in another typeface.
- Do not use Typst's teal or the Typst logo next to the Typrest mark in a way that suggests the project is official Typst.

## Who uses what

| Consumer | Files |
|---|---|
| Website (`website/`) | `logos/typrest-logo-horizontal*.svg`, `daisyui/typrest-themes.css`, `starlight/typrest.css`, and, copied into `website/public/` at build time by `website/scripts/sync-brand.mjs`, `favicon/favicon.svg`, `favicon.ico`, `apple-touch-icon.png`, `icon-192.png`, `icon-512.png`, and `social/social-card.png` |
| Web UI (`webui/`) | Planned: `daisyui/typrest-themes.css` and the brand fonts |
| Typst templates | `typst/theme.typ`, for Typrest-branded documents. The examples keep their own designs |
| GitHub | `social/social-card.png` as the repository's social preview |

Files not listed (the stacked logos, wordmarks, symbols, `icon-32.png`, `icon-512.svg`,
`social-card.svg`) are sources and spares for future uses.

## Starlight setup

The website's setup, in `website/astro.config.mjs`. Paths are relative to `website/`; how the
site applies the brand is in [`docs/development/website.md`](../docs/development/website.md).

```js
// astro.config.mjs
starlight({
  title: 'Typrest',
  logo: {
    light: '../brand/logos/typrest-logo-horizontal.svg',
    dark: '../brand/logos/typrest-logo-horizontal-inverse.svg',
    replacesTitle: true,
  },
  favicon: '/favicon.svg', // copied from brand/favicon/ by scripts/sync-brand.mjs
  customCss: [
    './src/styles/fonts.css',             // Fraunces (opsz), IBM Plex Sans 400/400i/500/600, Plex Mono 400
    './src/styles/global.css',            // Tailwind + daisyUI + ../brand/daisyui/typrest-themes.css
    '../brand/starlight/typrest.css',     // Starlight colors; must load last
  ],
})
```
