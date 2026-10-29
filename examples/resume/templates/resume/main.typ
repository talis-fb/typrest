#import "/_shared/theme.typ": *

#let data = json(bytes(sys.inputs.data))
#let b = data.basics
#let theme = accents.at(data.accent)

#set page(paper: "a4", margin: (x: 14mm, y: 14mm))
#set text(font: "Libertinus Serif", size: 10pt, fill: ink)
#set par(justify: false, leading: 0.6em)
#set block(spacing: 0.9em)
#set list(spacing: 0.5em)

#let section(title, body) = block(breakable: false, below: 1.1em)[
  #text(size: 9pt, weight: "bold", tracking: 0.12em, fill: theme.strong)[#upper(title)]
  #v(-0.3em)
  #line(length: 100%, stroke: 0.6pt + rule)
  #body
]

#let pill(label) = box(
  fill: theme.soft, radius: 8pt, inset: (x: 6pt, y: 3pt), outset: (y: 1pt),
  text(size: 8.5pt, fill: theme.strong)[#label],
)

// Header
#text(size: 24pt, weight: "bold", fill: theme.strong)[#b.name]
#v(-0.6em)
#text(size: 12pt, fill: muted)[#b.headline]
#v(0.2em)

#grid(
  columns: (31%, 1fr),
  column-gutter: 8mm,
  // Sidebar
  [
    #section("Contact")[
      #b.email \
      #if b.location != "" [#b.location \ ]
      #for l in b.links [
        #link(l.url)[#l.label] \
      ]
    ]
    #if data.skills.len() > 0 [
      #section("Skills")[
        #for g in data.skills [
          #text(weight: "bold", size: 9pt)[#g.group] \
          #for item in g.items [#pill(item) ] 
          #v(0.4em)
        ]
      ]
    ]
    #if data.education.len() > 0 [
      #section("Education")[
        #for e in data.education [
          *#e.degree* \
          #e.school \
          #if e.year != "" [#text(fill: muted)[#e.year]]
          #v(0.3em)
        ]
      ]
    ]
  ],
  // Main column
  [
    #if data.summary != "" [
      #section("Summary")[#data.summary]
    ]
    #if data.experience.len() > 0 [
      #section("Experience")[
        #for job in data.experience [
          #block(below: 0.9em)[
            #grid(
              columns: (1fr, auto),
              [*#job.role* · #job.company],
              text(fill: muted, size: 9pt)[#job.start – #job.end],
            )
            #if job.highlights.len() > 0 [
              #v(-0.3em)
              #for h in job.highlights [- #h]
            ]
            #v(0.4em)
          ]
        ]
      ]
    ]
  ],
)
