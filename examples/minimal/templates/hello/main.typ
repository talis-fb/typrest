#let data = json(bytes(sys.inputs.data))

#set page(paper: "a5", margin: 2cm)
#set text(font: "Libertinus Serif", size: 14pt)

#align(center + horizon)[
  #text(size: 32pt, weight: "bold")[#data.greeting,]
  #v(0.4em)
  #text(size: 40pt)[#data.name!]
]
