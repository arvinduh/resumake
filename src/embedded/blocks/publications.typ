// Publications block component

#import "../tokens.typ": *
#import "../primitives.typ": *

#let render-publications(publications, body-size: 11.5pt, muted-color: rgb("#444444")) = {
  let first-pub = true
  let pub-list = if type(publications) == array { publications } else { (publications,) }
  for pub in pub-list {
    let title = pub.title
    let venue = if "venue" in pub { pub.venue } else if "journal" in pub { pub.journal } else if "conference" in pub { pub.conference } else { "" }
    let d = if "year" in pub { str(pub.year) } else if "date" in pub { pub.date } else { "" }
    let authors = if "authors" in pub {
      if type(pub.authors) == array { pub.authors.join(", ") } else { pub.authors }
    } else { "" }

    let pub-line = [#bold(title)#DASH#italic(venue)]
    if "url" in pub and pub.url != "" {
      pub-line = link(pub.url)[#pub-line]
    } else if "doi" in pub and pub.doi != "" {
      pub-line = link(pub.doi)[#pub-line]
    }

    split-row(pub-line, muted-italic(d, muted-color: muted-color), above: if first-pub { 0em } else { GROUP_GAP }, below: ROLE_BELOW)
    if authors != "" {
      block(above: 0.15em, below: 0.25em)[#text(size: body-size - 0.5pt, fill: muted-color)[#authors]]
    }
    if "notes" in pub and pub.notes != "" {
      block(above: 0.10em, below: 0.25em)[#text(size: body-size - 0.5pt, style: "italic")[#pub.notes]]
    }
    first-pub = false
  }
}
