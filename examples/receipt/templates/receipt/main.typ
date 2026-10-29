#let data = json(bytes(sys.inputs.data))

// Two decimals with a thousands separator: 1234.5 -> "1,234.50".
#let money(n) = {
  let cents = calc.round(n * 100)
  let whole = str(calc.floor(calc.abs(cents) / 100))
  let f = calc.rem(calc.abs(cents), 100)
  let frac = if f < 10 { "0" + str(f) } else { str(f) }
  let grouped = whole.clusters().rev().chunks(3).map(c => c.rev().join()).rev().join(",")
  (if cents < 0 { "-" } else { "" }) + grouped + "." + frac
}
#let leader = box(width: 1fr, repeat[.])
#let row(left, right, bold: false) = {
  let body = [#left #leader #right]
  if bold { strong(body) } else { body }
}
#let rule = line(length: 100%, stroke: (dash: "dashed", thickness: 0.5pt))

#set page(width: 80mm, height: auto, margin: (x: 5mm, y: 6mm))
#set text(font: "DejaVu Sans Mono", size: 8pt)
#set par(leading: 0.5em, spacing: 0.6em)

// --- Calculations -----------------------------------------------------------
#let lines = data.items.map(i => (..i, total: i.quantity * i.unit_price))
#let subtotal = lines.map(l => l.total).sum(default: 0)
#let tax = subtotal * data.tax_rate / (100 + data.tax_rate)
#let total = subtotal
#let grand = total + data.tip

// --- Header -----------------------------------------------------------------
#align(center)[
  #text(size: 11pt, weight: "bold")[#upper(data.store.name)] \
  #data.store.address.join(linebreak())
  #if data.store.phone != "" [ \ Tel: #data.store.phone]
]
#rule

// --- Items ------------------------------------------------------------------
#for l in lines {
  block(breakable: false)[
    #row[#l.name][#money(l.total)] \
    #h(1em)#text(size: 7pt)[#l.quantity x #money(l.unit_price)]
    #if l.note != "" [ \ #h(1em)#text(size: 7pt, style: "italic")[#l.note]]
  ]
}
#rule

// --- Totals -----------------------------------------------------------------
#row[Subtotal][#money(subtotal)] \
#row[Tax incl. (#data.tax_rate%)][#money(tax)] \
#row[Total][#money(total)] \
#row[Tip][#money(data.tip)]
#rule
#text(size: 10pt)[#row(bold: true)[TOTAL][#money(grand)]]
#v(0.3em)
Paid by: #upper(data.payment_method)
#rule
#align(center)[#data.footer]
