#let data = json(bytes(sys.inputs.data))

#set page(margin: 18mm)
#set text(size: 10pt)

= Invoice #data.invoice_number

*Date:* #data.date \
*Due:* #data.due_date

== Bill To

#data.customer.name \
#data.customer.email

== Items

#for item in data.items [
- #item.description: #item.quantity x #item.unit_price = #item.total
]

== Total

*Subtotal:* #data.subtotal \
*Tax:* #data.tax \
*Total:* #data.total
