#let data = json(bytes(sys.inputs.data))

#set page(margin: 22mm)
#set text(size: 11pt)

= Welcome, #data.name

#data.message

#if data.company != "" [

Sent by #data.company.
]
