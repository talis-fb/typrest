// Shared design tokens and heading styles. Import with `#import "/_shared/theme.typ": *`.

#let ink = rgb("#1d2733")
#let muted = rgb("#667385")
#let accent = rgb("#1d4ed8")
#let accent-soft = rgb("#e8eefc")
#let rule-color = rgb("#d4dae3")
#let stripe = rgb("#f5f7fa")

// Numbered headings with an accent number and a thin rule under level 1.
#let apply-headings(body) = {
  set heading(numbering: "1.1")
  show heading.where(level: 1): it => {
    v(1.2em, weak: true)
    block(width: 100%, below: 0.9em)[
      #text(size: 18pt, weight: "bold", fill: ink)[
        #if it.numbering != none [#text(fill: accent)[#counter(heading).display(it.numbering)] #h(0.4em)]
        #it.body
      ]
      #v(-0.3em)
      #line(length: 100%, stroke: 0.8pt + accent)
    ]
  }
  show heading.where(level: 2): it => {
    v(0.8em, weak: true)
    text(size: 13pt, weight: "bold", fill: accent)[#it.body]
    v(0.3em, weak: true)
  }
  body
}
