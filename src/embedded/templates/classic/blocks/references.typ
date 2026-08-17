// References block component (2-column balanced grid)

#import "../tokens.typ": *
#import "../primitives.typ": *

#let refrow(a, b) = {
  block(above: LINE_GAP, below: LINE_GAP)[
    #guard(kind: "line")[
      #grid(
        columns: (1fr, 1fr),
        column-gutter: 1.2em,
        a, b,
      )
    ]
  ]
}

#let ref-item(name, role, org, muted-color: rgb("#444444")) = [#bold(
    name,
  )#DASH#italic(role) #h(1fr) #muted-italic(org, muted-color: muted-color)]

#let render-references(references, muted-color: rgb("#444444")) = {
  let items = if type(references) == array { references } else if (
    type(references) == dictionary and "items" in references
  ) { references.items } else { () }
  let i = 0
  while i < items.len() {
    if i + 1 < items.len() {
      refrow(
        ref-item(
          items.at(i).name,
          items.at(i).role,
          items.at(i).org,
          muted-color: muted-color,
        ),
        ref-item(
          items.at(i + 1).name,
          items.at(i + 1).role,
          items.at(i + 1).org,
          muted-color: muted-color,
        ),
      )
    } else {
      refrow(
        ref-item(
          items.at(i).name,
          items.at(i).role,
          items.at(i).org,
          muted-color: muted-color,
        ),
        [],
      )
    }
    i += 2
  }
}
