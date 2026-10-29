#import "/_shared/letterhead.typ": *

#let data = json(bytes(sys.inputs.data))

#show: letter-page

#letterhead(data.sender)

#grid(
  columns: (1fr, auto),
  recipient-block(data.recipient),
  align(right, text(fill: muted)[#data.date #if data.reference != "" [#linebreak()Ref: #data.reference]]),
)

#v(22pt)
#text(weight: "bold")[Subject: #data.subject]
#v(10pt)

#data.salutation

#for p in data.paragraphs [
  #p

]

#v(6pt)
#data.closing
#v(34pt)
#data.signatory \
#text(fill: muted, size: 9.5pt)[#data.sender.name]
