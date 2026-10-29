#import "/_shared/theme.typ": *

#let data = json(bytes(sys.inputs.data))

#set document(title: data.title, author: data.author)
#set text(font: "Libertinus Serif", size: 11pt, fill: ink)
#set par(justify: true, leading: 0.7em)

// --- Cover page (no header/footer) ------------------------------------------

#page(margin: 2.5cm, header: none, footer: none)[
  #rect(width: 100%, height: 6pt, fill: accent)
  #v(1fr)
  #text(size: 34pt, weight: "bold")[#data.title]
  #if data.subtitle != "" [
    #v(0.4em)
    #text(size: 16pt, fill: muted)[#data.subtitle]
  ]
  #v(1.5em)
  #line(length: 30%, stroke: 1.2pt + accent)
  #v(1fr)
  #text(size: 12pt)[*#data.author*] \
  #text(fill: muted)[#data.date]
]

// --- Body pages: running header, "Page X of Y" footer -----------------------

#set page(
  paper: "a4",
  margin: (x: 2.5cm, top: 3cm, bottom: 2.5cm),
  header: context {
    set text(size: 9pt, fill: muted)
    data.title
    h(1fr)
    data.date
    v(-0.5em)
    line(length: 100%, stroke: 0.5pt + rule-color)
  },
  footer: context {
    set text(size: 9pt, fill: muted)
    line(length: 100%, stroke: 0.5pt + rule-color)
    v(-0.4em)
    data.author
    h(1fr)
    [Page #counter(page).display() of #counter(page).final().first()]
  },
)
#counter(page).update(1)

#show: apply-headings

// Table of contents; the outline is resolved automatically by Typst's multi-pass layout.
#if data.show_toc [
  #block(below: 1em)[#text(size: 18pt, weight: "bold")[Contents]]
  #outline(title: none, depth: 2, indent: 1.2em)
  #pagebreak()
]

// Executive summary
#block(fill: accent-soft, inset: 14pt, radius: 4pt, width: 100%)[
  #text(weight: "bold", fill: accent)[Executive summary] \
  #v(0.2em)
  #data.summary
]

// --- Sections ----------------------------------------------------------------

// Let long tables break across pages; `table.header` repeats on every page.
#show figure.where(kind: table): set block(breakable: true)
#show figure.where(kind: table): set figure.caption(position: top)

#for s in data.sections [
  = #s.heading

  #s.body

  #if s.bullets.len() > 0 [
    #list(..s.bullets.map(b => [#b]))
  ]

  #if "table" in s [
    #let t = s.table
    #let ncols = t.columns.len()
    #v(0.6em)
    #figure(
      table(
        columns: ncols,
        stroke: none,
        inset: (x: 8pt, y: 6pt),
        fill: (_, y) => if y == 0 { accent } else if calc.odd(y) { stripe } else { none },
        table.header(..t.columns.map(c => text(fill: white, weight: "bold")[#c])),
        ..t.rows.flatten().map(c => [#c]),
      ),
      caption: if t.caption != "" { t.caption } else { none },
      kind: table,
      supplement: none,
    )
  ]
]
