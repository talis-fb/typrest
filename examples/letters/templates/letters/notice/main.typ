#import "/_shared/letterhead.typ": *

#let data = json(bytes(sys.inputs.data))
#let tone = if data.level == "warning" { warn } else { accent }

#show: letter-page

#letterhead(data.sender, color: tone)

#recipient-block(data.recipient)
#h(1fr)
#text(fill: muted)[#data.date]

#v(20pt)
#block(
  width: 100%,
  fill: tone.transparentize(90%),
  stroke: (left: 4pt + tone),
  inset: 14pt,
)[
  #text(size: 9pt, weight: "bold", fill: tone, tracking: 1pt)[#upper(data.level)]
  #v(2pt)
  #text(size: 17pt, weight: "bold")[#data.headline]
  #if data.effective_date != "" [
    #v(2pt)
    #text(fill: muted)[Effective #data.effective_date]
  ]
]

#v(12pt)
#data.message

#if data.actions.len() > 0 [
  #v(8pt)
  #strong[What you need to do]
  #for a in data.actions [
    - #a
  ]
]

#if data.contact_note != "" [
  #v(14pt)
  #line(length: 100%, stroke: 0.5pt + rule)
  #text(size: 9.5pt, fill: muted)[#data.contact_note]
]
