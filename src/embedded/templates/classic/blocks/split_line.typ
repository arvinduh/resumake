// Split Line component (certifications, awards, honors, speaking engagements)

#import "../tokens.typ": *
#import "../primitives.typ": *

#let render-split-line(
  items,
  body-size: 11.5pt,
  muted-color: rgb("#444444"),
) = {
  let first-item = true
  let item-list = if type(items) == array { items } else { (items,) }
  for it in item-list {
    let name = it.name
    let sub = if "issuer" in it { it.issuer } else if "organization" in it {
      it.organization
    } else if "event" in it { it.event } else { "" }
    let d = if "date" in it { it.date } else if "dates" in it {
      it.dates
    } else if "year" in it { str(it.year) } else { "" }

    let left = if sub != "" { [#bold(name)#DASH#italic(sub)] } else {
      bold(name)
    }
    if "url" in it and it.url != "" {
      left = link(it.url)[#left]
    } else if "link" in it and it.link != "" {
      left = link(it.link)[#left]
    }

    split-row(
      left,
      muted-italic(d, muted-color: muted-color),
      above: if first-item { 0em } else { LINE_GAP },
      below: LINE_GAP,
    )
    if "summary" in it and it.summary != "" {
      block(above: 0.10em, below: 0.25em)[#text(
        size: body-size - 0.5pt,
        fill: muted-color,
      )[#it.summary]]
    }
    first-item = false
  }
}
