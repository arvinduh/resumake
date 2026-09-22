// Resumake Primitives: Wrap guard, split-row, section dividers, and typography primitives.

#import "tokens.typ": *

// Wrap guard + fill-ratio telemetry
#let bulletinfo-counter = counter("resumake-bulletinfo")

#let guard(body, kind: "bullet") = layout(size => context {
  bulletinfo-counter.step()
  let n = bulletinfo-counter.get().first()
  let nat = measure(body)
  let fill = calc.round(nat.width / size.width * 100.0, digits: 1)
  let t = repr(body)
  [#metadata((
    id: kind + "-" + str(n),
    kind: kind,
    fill: fill,
    text: t.slice(0, calc.min(80, t.len())),
  )) <bulletinfo>]
  body
})

// Flexible spacing. Each call reserves `weight` shares of whatever height
// the page has left once the content is laid out at its natural spacing
// (see `fit-page` in main.typ); in an auto-height container it is zero.
// The counter records the total weight so `fit-page` can cap the stretch
// per share rather than per document.
//
// Hard `v` spacing suppresses the weak `above`/`below` spacing of the
// blocks around it, so the call also emits the gap's natural size `base`.
#let flex-counter = counter("resumake-flex")

#let flex-gap(weight, base) = {
  flex-counter.update(n => n + int(weight * 100))
  v(base)
  v(weight * 1fr)
}

// Text style primitives
#let bold(body) = text(weight: "bold")[#body]
#let italic(body) = text(style: "italic")[#body]
#let bold-italic(body) = text(weight: "bold", style: "italic")[#body]
#let muted-italic(body, muted-color: rgb("#444444")) = text(
  style: "italic",
  fill: muted-color,
)[#body]

// Structural layout helpers
#let section(
  title,
  sec-size: 13pt,
  accent-color: rgb("#2a2a2a"),
  rule-thick: 0.5pt,
) = {
  flex-gap(2, SEC_ABOVE)
  // The rule is the block's own bottom border rather than a separate line
  // pulled up with negative spacing, so its distance from the title stays
  // fixed whatever the font's line metrics or the paragraph leading are.
  block(
    above: SEC_ABOVE,
    below: RULE_BELOW,
    width: 100%,
    inset: (bottom: RULE_GAP),
    stroke: (bottom: rule-thick + accent-color),
    text(size: sec-size, weight: "semibold", tracking: 0.08em)[#upper(title)],
  )
}

// Generic "label ...... flush-right meta" row.
#let split-row(left, right, above: 0em, below: 0em, flex: 0) = {
  if flex > 0 { flex-gap(flex, above) }
  block(above: above, below: below)[#left #h(1fr) #right]
}

#let line-item(cat, body) = {
  block(above: LINE_GAP, below: LINE_GAP)[
    #guard(kind: "line")[#bold[#cat:] #body]
  ]
}

#let bullets(items) = {
  set list(
    marker: text(size: 0.85em)[•],
    indent: 0.30em,
    body-indent: 0.40em,
    spacing: BULLET_GAP,
  )
  block(above: 0em, below: 0em, list(..items.map(b => guard([#b]))))
}
