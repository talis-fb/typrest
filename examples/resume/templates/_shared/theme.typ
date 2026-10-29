// Shared design tokens. Import with `#import "/_shared/theme.typ": *`.

#let ink = rgb("#1f2933")
#let muted = rgb("#6b7785")
#let rule = rgb("#d5dce3")

// Maps the `accent` enum in schema.yml to colors. Add a key here and to the enum to add a theme.
#let accents = (
  blue: (strong: rgb("#1d4ed8"), soft: rgb("#e0e9fc")),
  green: (strong: rgb("#15803d"), soft: rgb("#dcf2e3")),
  slate: (strong: rgb("#334155"), soft: rgb("#e2e8f0")),
)
