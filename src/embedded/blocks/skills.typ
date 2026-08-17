// Skills block component

#import "../tokens.typ": *
#import "../primitives.typ": *

#let render-skills(skills) = {
  if type(skills) == dictionary {
    for (k, v) in skills.pairs() {
      let val-str = if type(v) == array { v.join(", ") } else { v }
      line-item(k, val-str)
    }
  } else if type(skills) == array {
    for s in skills {
      if type(s) == dictionary {
        let cat = if "category" in s { s.category } else if "name" in s { s.name } else { "" }
        let items = if "items" in s {
          if type(s.items) == array { s.items.join(", ") } else { s.items }
        } else { "" }
        line-item(cat, items)
      }
    }
  }
}
