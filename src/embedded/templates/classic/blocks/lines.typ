// Lines and freeform text component

#import "../tokens.typ": *
#import "../primitives.typ": *

#let render-lines(lines) = {
  if type(lines) == dictionary {
    for (k, v) in lines.pairs() {
      line-item(k, v)
    }
  } else if type(lines) == array {
    for item in lines {
      if type(item) == dictionary {
        for (k, v) in item.pairs() {
          line-item(k, v)
        }
      }
    }
  }
}

#let render-text(txt) = {
  block(above: LINE_GAP, below: LINE_GAP)[
    #guard(kind: "line")[#txt]
  ]
}
