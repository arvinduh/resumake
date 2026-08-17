// Resumake Primitives: Wrap guard, split-row, section dividers, and typography primitives.

#import "tokens.typ": *

// Wrap guard + fill-ratio telemetry
#let guard(body, kind: "bullet") = layout(size => context {
  let nat = measure(body)
  let fill = calc.round(nat.width / size.width * 100.0, digits: 1)
  let t = repr(body)
  [#metadata((
    kind: kind,
    fill: fill,
    text: t.slice(0, calc.min(80, t.len())),
  )) <bulletinfo>]
  body
})

// Text style primitives
#let bold(body) = text(weight: "bold")[#body]
#let italic(body) = text(style: "italic")[#body]
#let bold-italic(body) = text(weight: "bold", style: "italic")[#body]
#let muted-italic(body, muted-color: rgb("#444444")) = text(style: "italic", fill: muted-color)[#body]

// Structural layout helpers
#let section(title, sec-size: 13pt, accent-color: rgb("#2a2a2a"), rule-thick: 0.5pt) = {
  block(above: SEC_ABOVE, below: RULE_BELOW)[
    #text(size: sec-size, weight: "semibold", tracking: 0.08em)[#upper(title)]
    #v(-1.00em)
    #line(length: 100%, stroke: rule-thick + accent-color)
  ]
}

// Generic "label ...... flush-right meta" row.
#let split-row(left, right, above: 0em, below: 0em) = {
  block(above: above, below: below)[#left #h(1fr) #right]
}

#let line-item(cat, body) = {
  block(above: LINE_GAP, below: LINE_GAP)[
    #guard(kind: "line")[#bold[#cat:] #body]
  ]
}

#let bullets(items) = {
  set list(marker: text(size: 0.85em)[•], indent: 0.30em, body-indent: 0.40em, spacing: BULLET_GAP)
  block(above: 0em, below: 0em, list(..items.map(b => guard([#b]))))
}
