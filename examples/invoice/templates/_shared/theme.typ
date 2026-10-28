// Shared design tokens and formatting helpers. Any template can import this file with
// `#import "/_shared/theme.typ": *`.

#let ink = rgb("#1f2933")
#let muted = rgb("#6b7785")
#let accent = rgb("#0f766e")
#let accent-soft = rgb("#e6f4f2")
#let rule = rgb("#d5dce3")
#let stripe = rgb("#f6f8fa")

#let currencies = (
  USD: (symbol: "$", thousands: ",", decimal: "."),
  EUR: (symbol: "€", thousands: ".", decimal: ","),
  BRL: (symbol: "R$", thousands: ".", decimal: ","),
)

/// Formats an amount using the conventions of `currency`, rounded to cents.
#let money(amount, currency) = {
  let style = currencies.at(currency)
  let cents = calc.round(amount * 100)
  let negative = cents < 0
  let cents = calc.abs(int(cents))
  let digits = str(calc.quo(cents, 100))
  let groups = ()
  while digits.len() > 3 {
    groups.insert(0, digits.slice(-3))
    digits = digits.slice(0, -3)
  }
  groups.insert(0, digits)
  let fraction = str(calc.rem(cents, 100))
  if fraction.len() < 2 { fraction = "0" + fraction }
  let sign = if negative { "-" } else { "" }
  sign + style.symbol + "\u{a0}" + groups.join(style.thousands) + style.decimal + fraction
}

/// Formats a percentage without trailing zeros, e.g. `7.5%` or `10%`.
#let percent(value) = str(calc.round(value, digits: 2)) + "%"

/// Parses an ISO `YYYY-MM-DD` string and renders it as `July 23, 2026`. Anything else is
/// returned unchanged, which also keeps the empty startup warmup payload compiling.
#let long-date(iso) = {
  if iso.matches(regex("^\d{4}-\d{2}-\d{2}$")).len() != 1 { return iso }
  let parts = iso.split("-").map(int)
  datetime(year: parts.at(0), month: parts.at(1), day: parts.at(2)).display(
    "[month repr:long] [day padding:none], [year]",
  )
}

/// A small uppercase label above a value.
#let field(label, body) = stack(
  spacing: 3pt,
  text(size: 7.5pt, weight: "bold", fill: muted, tracking: 0.6pt, upper(label)),
  body,
)
