#import "/_shared/letterhead.typ": *

#let data = json(bytes(sys.inputs.data))

#show: letter-page
#set text(size: 11.5pt)

#letterhead(data.sender)

#recipient-block(data.recipient)
#h(1fr)
#text(fill: muted)[#data.date]

#v(24pt)
#text(size: 24pt, weight: "bold", fill: accent)[Welcome, #data.first_name!]
#v(4pt)
#text(fill: muted)[Your #data.product account is ready.]

#v(14pt)
#data.intro

#if data.next_steps.len() > 0 [
  #v(10pt)
  #text(weight: "bold")[Your first steps]
  #v(4pt)
  #for (i, step) in data.next_steps.enumerate() {
    grid(
      columns: (22pt, 1fr),
      row-gutter: 4pt,
      box(fill: accent, radius: 50%, width: 16pt, height: 16pt, align(center + horizon, text(fill: white, size: 9pt, weight: "bold")[#(i + 1)])),
      [*#step.title* #if step.detail != "" [#linebreak()#text(fill: muted)[#step.detail]]],
    )
    v(6pt)
  }
]

#v(18pt)
Warm regards, \
#data.team_name
