// Payload-driven localization without any runtime library. A locale is a dictionary entry;
// the template asks for labels with `t(locale, key)`, which falls back to English.

#let en = (
  title: "Account statement",
  holder: "Account holder",
  account: "Account number",
  period: "Statement period",
  issued: "Issued on",
  opening: "Opening balance",
  closing: "Closing balance",
  date: "Date",
  description: "Description",
  amount: "Amount",
  balance: "Balance",
  credits: "Total credits",
  debits: "Total debits",
  empty: "No transactions in this period.",
  page: "Page",
  of: "of",
)

#let locales = (
  en: (
    lang: "en", region: "US", thousands: ",", decimal: ".", symbol-after: false, date-format: "numeric-mdy",
    months: ("January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"),
    labels: en,
  ),
  "pt-BR": (
    lang: "pt", region: "BR", thousands: ".", decimal: ",", symbol-after: false, date-format: "numeric-dmy",
    months: ("janeiro", "fevereiro", "março", "abril", "maio", "junho", "julho", "agosto", "setembro", "outubro", "novembro", "dezembro"),
    labels: (
      title: "Extrato da conta",
      holder: "Titular",
      account: "Número da conta",
      period: "Período do extrato",
      issued: "Emitido em",
      opening: "Saldo inicial",
      closing: "Saldo final",
      date: "Data",
      description: "Descrição",
      amount: "Valor",
      balance: "Saldo",
      credits: "Total de créditos",
      debits: "Total de débitos",
      empty: "Nenhuma transação neste período.",
      page: "Página",
      of: "de",
    ),
  ),
  es: (
    lang: "es", region: "ES", thousands: ".", decimal: ",", symbol-after: true, date-format: "numeric-dmy",
    months: ("enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto", "septiembre", "octubre", "noviembre", "diciembre"),
    labels: (
      title: "Extracto de cuenta",
      holder: "Titular",
      account: "Número de cuenta",
      period: "Periodo del extracto",
      issued: "Emitido el",
      opening: "Saldo inicial",
      closing: "Saldo final",
      date: "Fecha",
      description: "Descripción",
      amount: "Importe",
      balance: "Saldo",
      credits: "Total de abonos",
      debits: "Total de cargos",
      empty: "No hay movimientos en este periodo.",
      page: "Página",
      of: "de",
    ),
  ),
  de: (
    lang: "de", region: "DE", thousands: ".", decimal: ",", symbol-after: true, date-format: "numeric-dmy-dot",
    months: ("Januar", "Februar", "März", "April", "Mai", "Juni", "Juli", "August", "September", "Oktober", "November", "Dezember"),
    labels: (
      title: "Kontoauszug",
      holder: "Kontoinhaber",
      account: "Kontonummer",
      period: "Auszugszeitraum",
      issued: "Erstellt am",
      opening: "Anfangssaldo",
      closing: "Endsaldo",
      date: "Datum",
      description: "Beschreibung",
      amount: "Betrag",
      balance: "Saldo",
      credits: "Summe Gutschriften",
      debits: "Summe Belastungen",
      empty: "Keine Buchungen in diesem Zeitraum.",
      page: "Seite",
      of: "von",
    ),
  ),
)

#let symbols = (USD: "$", EUR: "€", BRL: "R$", GBP: "£")

/// Settings for `locale`, falling back to English for anything unknown.
#let locale-of(locale) = locales.at(locale, default: locales.en)

/// Label lookup: the locale's entry, then English, then the key itself.
#let t(locale, key) = locale-of(locale).labels.at(key, default: en.at(key, default: key))

/// Formats `amount` with the locale's separators and exactly `digits` decimals.
#let number(locale, amount, digits: 2) = {
  let style = locale-of(locale)
  let scaled = int(calc.round(calc.abs(amount) * calc.pow(10, digits)))
  let whole = str(calc.quo(scaled, calc.pow(10, digits)))
  let fraction = str(calc.rem(scaled, calc.pow(10, digits)))
  while fraction.len() < digits { fraction = "0" + fraction }
  let groups = ()
  while whole.len() > 3 {
    groups.insert(0, whole.slice(-3))
    whole = whole.slice(0, -3)
  }
  groups.insert(0, whole)
  let sign = if amount < 0 { "\u{2212}" } else { "" }
  sign + groups.join(style.thousands) + style.decimal + fraction
}

/// Formats a money amount: symbol before or after the number depending on the locale.
#let money(locale, amount, currency) = {
  let style = locale-of(locale)
  let symbol = symbols.at(currency, default: currency)
  let body = number(locale, calc.abs(amount))
  let sign = if amount < 0 { "\u{2212}" } else { "" }
  if style.symbol-after { sign + body + "\u{a0}" + symbol } else { sign + symbol + "\u{a0}" + body }
}

#let iso-parts(iso) = {
  if iso.matches(regex("^\d{4}-\d{2}-\d{2}$")).len() != 1 { return none }
  iso.split("-").map(int)
}

#let pad(n) = if n < 10 { "0" + str(n) } else { str(n) }

/// Short numeric date in the locale's order. Non-ISO input is returned unchanged.
#let short-date(locale, iso) = {
  let p = iso-parts(iso)
  if p == none { return iso }
  let (y, m, d) = p
  let style = locale-of(locale)
  if style.date-format == "numeric-mdy" { pad(m) + "/" + pad(d) + "/" + str(y) }
  else if style.date-format == "numeric-dmy-dot" { pad(d) + "." + pad(m) + "." + str(y) }
  else { pad(d) + "/" + pad(m) + "/" + str(y) }
}

/// Long date with the locale's month names and word order.
#let long-date(locale, iso) = {
  let p = iso-parts(iso)
  if p == none { return iso }
  let (y, m, d) = p
  let style = locale-of(locale)
  let month = style.months.at(m - 1)
  if locale == "pt-BR" or locale == "es" { str(d) + " de " + month + " de " + str(y) }
  else if locale == "de" { str(d) + ". " + month + " " + str(y) }
  else { month + " " + str(d) + ", " + str(y) }
}
