// Prints the payload exactly as the template receives it, after validation and defaults.
#let data = json(bytes(sys.inputs.data))

#set page(paper: "a4", margin: 18mm)
#set text(font: "Libertinus Serif", size: 10pt)

#let show-value(v) = {
  if type(v) == str { raw("\"" + v + "\"") } else { raw(repr(v)) }
}

#let rows(d, prefix: "") = {
  let out = ()
  for (k, v) in d {
    let path = prefix + k
    if type(v) == dictionary and v.len() > 0 {
      out += rows(v, prefix: path + ".")
    } else {
      out.push((path, show-value(v)))
    }
  }
  out
}

= Schema v1 showcase

The table below is the payload as Typst sees it, after validation and defaults.
Compare it with the request body you sent.

#table(
  columns: (auto, 1fr),
  stroke: 0.5pt + luma(180),
  fill: (_, y) => if y == 0 { luma(235) },
  inset: 6pt,
  [*Path*], [*Value*],
  ..for (path, val) in rows(data) { (raw(path), val) },
)
