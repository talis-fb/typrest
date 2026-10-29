#import "@preview/tiaoma:0.3.0": qrcode

#let data = json(bytes(sys.inputs.data))

#let ink = rgb("#1f2a44")
#let accent = rgb("#c2410c")
#let muted = rgb("#6b7280")

#let seat = if data.holder.seat == "" { "General admission" } else { data.holder.seat }
// The QR encoder rejects empty input, which also happens during the warm-up compile
// (schema-seeded payload). Draw an empty box instead of failing.
#let qr(content) = if content == "" { rect(width: 30mm, height: 30mm, stroke: 0.5pt + muted) } else { qrcode(content, width: 30mm) }
#let price = if data.price == 0 { "Free" } else { "$" + str(calc.floor(data.price)) + "." + { let c = str(calc.round((data.price - calc.floor(data.price)) * 100)); if c.len() < 2 { "0" + c } else { c } } }

#set page(width: 210mm, height: 90mm, margin: 0pt)
#set text(font: "Libertinus Serif", size: 10pt, fill: ink)

#let label(body) = text(size: 7pt, fill: muted, tracking: 1pt, upper(body))

#grid(
  columns: (150mm, 60mm),
  // Main body
  block(width: 150mm, height: 90mm, inset: (x: 12mm, y: 10mm), fill: rgb("#fdf8ee"))[
    #text(size: 8pt, fill: accent, tracking: 2pt, weight: "bold")[ADMIT ONE]
    #v(2mm)
    #text(size: 24pt, weight: "bold")[#data.event.name]
    #v(5mm)
    #grid(
      columns: (1fr, 1fr),
      row-gutter: 5mm,
      [#label[Date]\ #data.event.date],
      [#label[Venue]\ #data.event.venue],
      [#label[Holder]\ #data.holder.name],
      [#label[Seat]\ #seat],
    )
    #place(bottom + left)[#label[Ticket]  #data.ticket_id #h(8mm) #label[Price]  #price]
  ],
  // Stub
  block(width: 60mm, height: 90mm, inset: 8mm, fill: ink)[
    #set text(fill: white)
    #align(center)[
      #block(fill: white, inset: 2mm, radius: 1.5mm)[#qr(data.qr_payload)]
      #v(3mm)
      #text(size: 8pt, tracking: 1pt)[#data.ticket_id]
      #v(2mm)
      #text(size: 9pt, weight: "bold")[#seat]
    ]
  ],
)

// Perforation line between main body and stub
#place(top + left, dx: 147mm, line(angle: 90deg, length: 90mm, stroke: (paint: ink, thickness: 0.8pt, dash: "dotted")))
