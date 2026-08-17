// Projects block component

#import "../tokens.typ": *
#import "../primitives.typ": *

#let project(
  name,
  stack,
  date,
  muted-color: rgb("#444444"),
  first: false,
  link-url: none,
) = {
  let left = if stack != "" and stack != none {
    [#bold(name)#DASH#italic(stack)]
  } else {
    bold(name)
  }
  if link-url != none and link-url != "" {
    left = link(link-url)[#left]
  }
  split-row(
    left,
    muted-italic(date, muted-color: muted-color),
    above: if first { 0em } else { GROUP_GAP },
    below: ROLE_BELOW,
  )
}

#let render-projects(projects, muted-color: rgb("#444444")) = {
  let first-p = true
  let proj-list = if type(projects) == array { projects } else { (projects,) }
  for proj in proj-list {
    let name = proj.name
    let stack = if "stack" in proj { proj.stack } else if (
      "technologies" in proj
    ) { proj.technologies } else { "" }
    let d = if "date" in proj { proj.date } else if "dates" in proj {
      proj.dates
    } else { "" }
    let url = if "link" in proj { proj.link } else if "url" in proj {
      proj.url
    } else { none }

    project(
      name,
      stack,
      d,
      muted-color: muted-color,
      first: first-p,
      link-url: url,
    )
    if "bullets" in proj and proj.bullets.len() > 0 {
      bullets(proj.bullets)
    }
    first-p = false
  }
}
