// Letterhead and address helpers shared by every letter.
#import "/_shared/theme.typ": *

/// Sender banner: company name, address lines and contact e-mail.
#let letterhead(sender, color: accent) = {
  grid(
    columns: (1fr, auto),
    align: (left + horizon, right + horizon),
    text(size: 17pt, weight: "bold", fill: color)[#sender.name],
    align(right, text(size: 8.5pt, fill: muted)[
      #sender.address.join(linebreak())
      #if sender.email != "" [#linebreak()#sender.email]
    ]),
  )
  v(4pt)
  line(length: 100%, stroke: 1.2pt + color)
  v(14pt)
}

/// Recipient block: name followed by address lines.
#let recipient-block(recipient) = {
  strong(recipient.name)
  for line in recipient.address {
    linebreak()
    line
  }
}

/// Page setup and base text style for a letter.
#let letter-page(body) = {
  set page(paper: "a4", margin: (x: 25mm, top: 20mm, bottom: 22mm))
  set text(font: body-font, size: 11pt, fill: ink)
  set par(justify: true, leading: 0.7em)
  body
}
