#import "/_shared/theme.typ": *

#let data = json(bytes(sys.inputs.data))
#let cur = data.currency
#let fmt(amount) = money(amount, cur)

// --- Calculations -----------------------------------------------------------

#let global-discount = if "discount" in data { data.discount } else { (label: "Discount", percent: 0) }
#let global-factor = 1 - global-discount.percent / 100

#let lines = data.items.map(item => {
  let gross = item.quantity * item.unit_price
  let net = gross * (1 - item.discount_percent / 100)
  (..item, gross: gross, net: net, tax: net * global-factor * item.tax_rate / 100)
})

#let subtotal = lines.map(l => l.net).sum(default: 0)
#let discount-amount = subtotal * global-discount.percent / 100
#let tax-total = lines.map(l => l.tax).sum(default: 0)
#let total = subtotal - discount-amount + tax-total

// Tax collected per distinct rate, sorted by rate.
#let tax-by-rate = {
  let groups = (:)
  for l in lines.filter(l => l.tax_rate > 0) {
    let key = str(l.tax_rate)
    let prev = groups.at(key, default: (rate: l.tax_rate, amount: 0))
    groups.insert(key, (rate: l.tax_rate, amount: prev.amount + l.tax))
  }
  groups.values().sorted(key: g => g.rate)
}

#let status-label = (draft: "Draft", sent: "Due", paid: "Paid", overdue: "Overdue")
#let status-color = (draft: muted, sent: accent, paid: rgb("#15803d"), overdue: rgb("#b91c1c"))

// --- Page -------------------------------------------------------------------

#set document(title: "Invoice " + data.number, author: data.seller.name)
#set text(font: "Libertinus Serif", size: 10pt, fill: ink)
#set par(leading: 0.55em)
#set page(
  paper: "a4",
  margin: (x: 18mm, top: 16mm, bottom: 22mm),
  footer: context {
    set text(size: 8pt, fill: muted)
    line(length: 100%, stroke: 0.5pt + rule)
    v(-2pt)
    grid(
      columns: (1fr, auto),
      [#data.seller.name · #data.seller.email #if data.seller.website != "" [· #data.seller.website]],
      [Invoice #data.number · Page #counter(page).display() of #counter(page).final().first()],
    )
  },
  background: if data.status == "draft" or data.status == "paid" {
    rotate(-28deg, text(
      size: 120pt,
      weight: "bold",
      fill: status-color.at(data.status).transparentize(90%),
      upper(data.status),
    ))
  },
)

// --- Header -----------------------------------------------------------------

#grid(
  columns: (1fr, auto),
  align: (left + horizon, right + horizon),
  grid(
    columns: (auto, 1fr),
    column-gutter: 10pt,
    align: horizon,
    image("assets/logo.svg", width: 34pt),
    stack(
      spacing: 4pt,
      text(size: 17pt, weight: "bold")[#data.seller.name],
      if data.seller.tagline != "" { text(size: 9pt, fill: muted, style: "italic")[#data.seller.tagline] },
    ),
  ),
  stack(
    spacing: 6pt,
    align(right, text(size: 26pt, weight: "bold", fill: accent, tracking: 2pt)[INVOICE]),
    align(right, box(
      fill: status-color.at(data.status).transparentize(85%),
      inset: (x: 7pt, y: 3pt),
      radius: 3pt,
      text(size: 8pt, weight: "bold", fill: status-color.at(data.status), tracking: 0.8pt, upper(status-label.at(data.status))),
    )),
  ),
)

#v(8pt)
#line(length: 100%, stroke: 1.5pt + accent)
#v(10pt)

// --- Meta + parties ---------------------------------------------------------

#grid(
  columns: (1fr,) * 4,
  column-gutter: 10pt,
  field("Invoice no.")[*#data.number*],
  field("Issue date")[#long-date(data.issue_date)],
  field("Due date")[*#long-date(data.due_date)*],
  field("Amount due")[#text(fill: accent, weight: "bold", size: 12pt)[#if data.status == "paid" { fmt(0) } else { fmt(total) }]],
)

#v(12pt)

#let party(label, p) = grid.cell(
  inset: 10pt,
  fill: stripe,
  stroke: 0.5pt + rule,
  field(label)[
    #text(weight: "bold", size: 11pt)[#p.name] \
    #if "contact" in p and p.contact != "" [#p.contact \ ]
    #p.address.join(linebreak()) \
    #text(fill: muted)[#p.email] \
    #if p.tax_id != "" [#text(size: 8.5pt, fill: muted)[#p.tax_id]]
  ],
)

#grid(
  columns: (1fr, 1fr),
  column-gutter: 12pt,
  align: left,
  party("From", data.seller), party("Bill to", data.customer),
)

#if data.purchase_order != "" [
  #v(6pt)
  #text(size: 9pt, fill: muted)[Purchase order: ] #text(size: 9pt, weight: "bold")[#data.purchase_order]
]

#v(14pt)

// --- Line items -------------------------------------------------------------

#let head(body) = text(size: 8pt, weight: "bold", fill: white, tracking: 0.5pt, upper(body))

#table(
  columns: (1fr, auto, auto, auto, auto, auto),
  align: (left + top, right + top, right + top, right + top, right + top, right + top),
  inset: (x: 7pt, y: 6pt),
  stroke: none,
  fill: (_, y) => if y == 0 { accent } else if calc.even(y) { stripe },
  table.header(
    head[Description], head[Qty], head[Unit price], head[Disc.], head[Tax], head[Amount],
  ),
  ..for l in lines {
    (
      table.cell[
        *#l.name* \
        #if l.description != "" { text(size: 8.5pt, fill: muted)[#l.description] }
      ],
      [#l.quantity#if l.unit != "" [~#l.unit]],
      fmt(l.unit_price),
      if l.discount_percent > 0 { percent(l.discount_percent) } else { text(fill: muted)[—] },
      if l.tax_rate > 0 { percent(l.tax_rate) } else { text(fill: muted)[—] },
      table.cell[*#fmt(l.net)*],
    )
  },
  table.hline(stroke: 0.75pt + accent),
)

#v(10pt)

// --- Totals -----------------------------------------------------------------

#let total-row(label, value, ..args) = (
  text(..args)[#label], align(right, text(..args)[#value]),
)

#align(right, block(
  width: 62%,
  breakable: false,
  grid(
    columns: (1fr, auto),
    row-gutter: 7pt,
    align: (left, right),
    ..total-row("Subtotal", fmt(subtotal)),
    ..if global-discount.percent > 0 {
      total-row([#global-discount.label (#percent(global-discount.percent))], "−" + fmt(discount-amount), fill: rgb("#b45309"))
    },
    ..for g in tax-by-rate {
      total-row([Tax (#percent(g.rate))], fmt(g.amount))
    },
    grid.cell(colspan: 2, line(length: 100%, stroke: 0.5pt + rule)),
    grid.cell(
      colspan: 2,
      block(
        fill: accent-soft,
        inset: 9pt,
        radius: 4pt,
        width: 100%,
        grid(
          columns: (1fr, auto),
          align: (left + horizon, right + horizon),
          text(weight: "bold", size: 11pt)[Total #cur],
          text(weight: "bold", size: 14pt, fill: accent)[#fmt(total)],
        ),
      ),
    ),
  ),
))

#v(16pt)

// --- Payment + notes --------------------------------------------------------

#block(breakable: false, grid(
  columns: (1fr, 1fr),
  column-gutter: 16pt,
  align: left + top,
  if "payment" in data and (data.payment.method != "" or data.payment.details.len() > 0) {
    field("Payment")[
      #if data.payment.terms != "" [Terms: *#data.payment.terms* \ ]
      #if data.payment.method != "" [#data.payment.method]
      #v(2pt)
      #grid(
        columns: (auto, 1fr),
        column-gutter: 8pt,
        row-gutter: 4pt,
        ..for d in data.payment.details {
          (text(size: 9pt, fill: muted)[#d.label], text(size: 9pt)[#d.value])
        }
      )
    ]
  },
  if data.notes != "" {
    field("Notes")[#text(size: 9pt)[#data.notes]]
  },
))
