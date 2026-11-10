// Typrest brand tokens for Typst templates.
// Import with `#import "/_shared/theme.typ": *` from any template in the project.
// Fonts: add Fraunces, IBM Plex Sans and IBM Plex Mono to the project's font
// directory; the fallbacks below keep the template compiling without them.

// Ink (warm neutrals)
#let ink = (
  "50": rgb("#F9F6F0"), "100": rgb("#F4EFE6"), "200": rgb("#E6DFD2"),
  "300": rgb("#CFC6B6"), "400": rgb("#A9A193"), "500": rgb("#8A847A"),
  "600": rgb("#6B655C"), "700": rgb("#4A453E"), "800": rgb("#2C2823"),
  "900": rgb("#16130F"),
)
// Press red (brand)
#let press = (
  "50": rgb("#FCEDE9"), "100": rgb("#F8D3C9"), "200": rgb("#F0A794"),
  "300": rgb("#E7765A"), "400": rgb("#DE5638"), "500": rgb("#D9381E"),
  "600": rgb("#B82C16"), "700": rgb("#922312"), "800": rgb("#6B1A0E"),
  "900": rgb("#45110A"),
)

#let paper = ink.at("100")
#let text-color = ink.at("900")
#let muted = ink.at("600")
#let rule = ink.at("200")
#let accent = press.at("600")    // text-safe red on paper (5.4:1)
#let brand-red = press.at("500") // decorative only: rules, the type-dot

#let status = (
  success: (strong: rgb("#4F7A3A"), soft: rgb("#EAF0E2"), text: rgb("#34521F")),
  warning: (strong: rgb("#C98A1B"), soft: rgb("#F7ECD5"), text: rgb("#6E4A0B")),
  error:   (strong: rgb("#B3243B"), soft: rgb("#F6E3E7"), text: rgb("#7D1A2B")),
  info:    (strong: rgb("#2F5D8A"), soft: rgb("#E3EBF3"), text: rgb("#1F3E5E")),
)

#let font-display = ("Fraunces", "Libertinus Serif")
#let font-body = ("IBM Plex Sans", "Libertinus Serif")
#let font-mono = ("IBM Plex Mono", "DejaVu Sans Mono")

// The type-dot: a small brand-red square, used as a bullet or end mark.
#let type-dot(size: 0.4em) = box(width: size, height: size, fill: brand-red, baseline: -0.1em)

// Applies the brand to a whole document: `#show: typrest-style`
#let typrest-style(body) = {
  set text(font: font-body, size: 10.5pt, fill: text-color)
  set par(leading: 0.7em, justify: false)
  show heading: set text(font: font-display, weight: 600, fill: text-color)
  show heading.where(level: 1): set text(size: 22pt)
  show heading.where(level: 2): set text(size: 15pt)
  show raw: set text(font: font-mono, size: 0.92em)
  show link: set text(fill: accent)
  set list(marker: type-dot())
  set table(stroke: (x, y) => if y == 0 { (bottom: 0.8pt + text-color) } else { (bottom: 0.4pt + rule) })
  body
}
