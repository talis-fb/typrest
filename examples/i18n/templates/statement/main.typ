#import "/_shared/i18n.typ": *

#let data = json(bytes(sys.inputs.data))
#let loc = data.locale
#let cur = data.currency
#let style = locale-of(loc)
#let L(key) = t(loc, key)
#let M(amount) = money(loc, amount, cur)

#let ink = rgb("#1f2933")
#let muted = rgb("#6b7785")
#let accent = rgb("#1d4e89")
#let rule = rgb("#d5dce3")
#let positive = rgb("#17613a")

// Running balance per transaction, in payload order.
#let rows = {
  let balance = data.account.opening_balance
  let out = ()
  for tx in data.transactions {
    balance += tx.amount
    out.push((..tx, balance: balance))
  }
  out
}
#let closing = data.account.opening_balance + data.transactions.map(tx => tx.amount).sum(default: 0)
#let credits = data.transactions.filter(tx => tx.amount > 0).map(tx => tx.amount).sum(default: 0)
#let debits = data.transactions.filter(tx => tx.amount < 0).map(tx => tx.amount).sum(default: 0)

#set text(font: "Libertinus Serif", size: 10pt, fill: ink, lang: style.lang, region: style.region)
#set page(
  paper: "a4",
  margin: (x: 18mm, top: 20mm, bottom: 22mm),
  footer: context text(size: 8.5pt, fill: muted)[
    #data.bank #h(1fr) #L("page") #counter(page).display() #L("of") #counter(page).final().first()
  ],
)

#let label(body) = text(size: 8pt, fill: muted, tracking: 0.06em, upper(body))
#let money-cell(amount) = align(right, text(fill: if amount < 0 { ink } else { positive }, M(amount)))

#grid(
  columns: (1fr, auto),
  align: (left + horizon, right + horizon),
  text(size: 17pt, weight: "bold", fill: accent)[#data.bank],
  text(size: 13pt, fill: muted)[#L("title")],
)
#v(2mm)
#line(length: 100%, stroke: 1.2pt + accent)
#v(5mm)

#grid(
  columns: (1fr, 1fr),
  row-gutter: 5mm,
  column-gutter: 8mm,
  [#label(L("holder")) \ #text(weight: "bold", size: 11.5pt)[#data.account.holder] \ #data.account.address.join(linebreak())],
  [
    #label(L("account")) \ #data.account.number \
    #v(2mm)
    #label(L("period")) \ #short-date(loc, data.period_start) – #short-date(loc, data.period_end) \
    #v(2mm)
    #label(L("issued")) \ #long-date(loc, data.statement_date)
  ],
)
#v(6mm)

#block(fill: rgb("#eef3f9"), inset: 10pt, radius: 3pt, width: 100%)[
  #grid(
    columns: (1fr, 1fr, 1fr, 1fr),
    [#label(L("opening")) \ #text(weight: "bold")[#M(data.account.opening_balance)]],
    [#label(L("credits")) \ #text(weight: "bold", fill: positive)[#M(credits)]],
    [#label(L("debits")) \ #text(weight: "bold")[#M(debits)]],
    [#label(L("closing")) \ #text(weight: "bold", fill: accent)[#M(closing)]],
  )
]
#v(6mm)

#if rows.len() == 0 [
  #text(fill: muted, style: "italic")[#L("empty")]
] else [
  #table(
    columns: (24mm, 1fr, 32mm, 32mm),
    stroke: none,
    inset: (x: 6pt, y: 6pt),
    fill: (_, y) => if y > 0 and calc.odd(y) { rgb("#f6f8fa") } else { none },
    table.header(
      table.cell(stroke: (bottom: 1pt + accent))[#label(L("date"))],
      table.cell(stroke: (bottom: 1pt + accent))[#label(L("description"))],
      table.cell(stroke: (bottom: 1pt + accent), align: right)[#label(L("amount"))],
      table.cell(stroke: (bottom: 1pt + accent), align: right)[#label(L("balance"))],
    ),
    ..rows.map(r => (
      short-date(loc, r.date),
      r.description,
      money-cell(r.amount),
      align(right, M(r.balance)),
    )).flatten(),
  )
]
