#let data = json(bytes(sys.inputs.data))

// Fonts come from templates/certificate/fonts/ (REQ-TPL-040), not from the OS.
#let display = "Cinzel"
#let script = "Pinyon Script"

// Each style picks colors and an ornament; the enum in schema.yml lists the keys.
#let styles = (
  classic: (
    paper: rgb("#fdf8ec"), ink: rgb("#2b2118"), accent: rgb("#8a6a14"),
    ornament: "❖",
  ),
  modern: (
    paper: rgb("#f4f7fb"), ink: rgb("#14213d"), accent: rgb("#1f6feb"),
    ornament: "◆",
  ),
)
#let s = styles.at(data.style)

#set document(title: "Certificate " + data.certificate_id, author: data.issuer.name)
#set page(paper: "a4", flipped: true, margin: 0pt, fill: s.paper)
#set text(font: display, fill: s.ink, size: 12pt)

// Double border drawn with shapes; the modern style uses a single thick frame.
#place(top + left, dx: 14mm, dy: 14mm,
  rect(width: 100% - 28mm, height: 100% - 28mm, stroke: 2.5pt + s.accent))
#if data.style == "classic" {
  place(top + left, dx: 19mm, dy: 19mm,
    rect(width: 100% - 38mm, height: 100% - 38mm, stroke: 0.75pt + s.accent))
  for (a, b) in ((top + left, (1, 1)), (top + right, (-1, 1)), (bottom + left, (1, -1)), (bottom + right, (-1, -1))) {
    place(a, dx: b.at(0) * 24mm, dy: b.at(1) * 24mm,
      circle(radius: 3mm, fill: s.accent, stroke: none))
  }
} else {
  place(top + left, dx: 14mm, dy: 14mm,
    rect(width: 100% - 28mm, height: 14mm, fill: s.accent, stroke: none))
}

#align(center + top)[
  #set par(spacing: 0pt)
  #v(if data.style == "modern" { 34mm } else { 28mm })
  #text(size: 34pt, tracking: 4pt, fill: s.accent)[CERTIFICATE]
  #v(5mm)
  #text(size: 13pt, tracking: 5pt)[OF COMPLETION]
  #v(8mm)
  #text(fill: s.accent, size: 16pt)[#s.ornament]
  #v(10mm)
  #text(size: 12pt, tracking: 1pt)[This certifies that]
  #v(9mm)
  #text(font: script, size: 46pt)[#data.recipient]
  #v(3mm)
  #line(length: 110mm, stroke: 0.75pt + s.accent)
  #v(8mm)
  #text(size: 12pt, tracking: 1pt)[has successfully completed]
  #v(6mm)
  #text(size: 22pt, weight: "bold")[#data.course]
  #v(6mm)
  #text(size: 12pt, tracking: 1pt)[on #data.date]
]

// Seal from a bundled SVG, bottom center.
#place(bottom + center, dy: -22mm, image("assets/seal.svg", width: 24mm))

// Signature block, bottom right.
#place(bottom + right, dx: -38mm, dy: -26mm)[
  #align(center)[
    #text(font: script, size: 22pt)[#data.issuer.name]
    #v(-3mm)
    #line(length: 55mm, stroke: 0.6pt + s.ink)
    #v(-1mm)
    #text(size: 9pt, tracking: 1pt)[#data.issuer.name]
    #if data.issuer.title != "" {
      linebreak()
      text(size: 8pt, fill: s.ink.lighten(30%))[#data.issuer.title]
    }
  ]
]

// Certificate ID, bottom left.
#place(bottom + left, dx: 38mm, dy: -30mm)[
  #text(size: 8pt, tracking: 1pt, fill: s.ink.lighten(30%))[CERTIFICATE ID]
  #linebreak()
  #text(size: 10pt, tracking: 1pt)[#data.certificate_id]
]
