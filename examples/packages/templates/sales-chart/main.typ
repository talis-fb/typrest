// Packages must be literal and fully versioned (REQ-TPL-060/061).
#import "@preview/cetz:0.4.2": canvas
#import "@preview/cetz-plot:0.1.3": chart

#let data = json(bytes(sys.inputs.data))
#let suffix = if data.unit == "" { "" } else { " " + data.unit }

#set page(paper: "a4", margin: 2cm)
#set text(font: "Libertinus Serif", size: 11pt)

#text(size: 22pt, weight: "bold")[#data.title]
#v(0pt)
#text(fill: luma(100))[#data.period]
#v(1cm)

#let rows = data.series.map(s => (s.label, s.value))

// Warm compilation runs with an empty array for `series`, and charts cannot be drawn
// without data, so the chart is only drawn when there are rows.
#if rows.len() > 0 [
#align(center)[
  #canvas({
    chart.barchart(
      size: (12, 7),
      label-key: 0,
      value-key: 1,
      x-label: data.unit,
      bar-style: _ => (fill: rgb("#2f6fb0"), stroke: none),
      rows,
    )
  })
]
]

#v(1cm)
#table(
  columns: (1fr, auto),
  align: (left, right),
  stroke: (x: none, y: 0.5pt + luma(200)),
  table.header[*Series*][*Value*],
  ..data.series.map(s => (s.label, [#s.value#suffix])).flatten(),
  table.cell(fill: luma(240))[*Total*],
  table.cell(fill: luma(240))[*#data.series.map(s => s.value).sum(default: 0)#suffix*],
)
