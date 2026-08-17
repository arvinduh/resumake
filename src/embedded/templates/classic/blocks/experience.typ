// Experience block component (supports multi-role promotion ladders and single roles)

#import "../tokens.typ": *
#import "../primitives.typ": *

#let format-role-title(title, stack: none) = {
  let base = if title.contains(" - ") {
    let parts = title.split(" - ")
    let prefix = parts.at(0)
    let suffix = parts.slice(1).join(" - ")
    italic[#bold[#prefix]#DASH#suffix]
  } else {
    bold-italic(title)
  }
  if stack != none and stack != "" {
    [#base #DASH #italic(stack)]
  } else {
    base
  }
}

#let role(
  title,
  dates,
  muted-color: rgb("#444444"),
  first: false,
  stack: none,
) = split-row(
  format-role-title(title, stack: stack),
  muted-italic(dates, muted-color: muted-color),
  above: if first { 0em } else { ROLE_ABOVE },
  below: ROLE_BELOW,
)

#let render-experience(
  experience,
  org-size: 12pt,
  muted-color: rgb("#444444"),
) = {
  let first-exp = true
  let exp-list = if type(experience) == array { experience } else {
    (experience,)
  }
  for exp in exp-list {
    let org-name = if "org" in exp { exp.org } else if "company" in exp {
      exp.company
    } else { exp.organization }
    let loc = if "location" in exp { exp.location } else { "" }

    split-row(
      text(size: org-size, weight: "bold")[#org-name],
      muted-italic(loc, muted-color: muted-color),
      above: if first-exp { 0em } else { GROUP_GAP },
      below: ORG_BELOW,
    )

    let roles-list = if "roles" in exp { exp.roles } else { (exp,) }
    let first-r = true
    for r in roles-list {
      let r-title = if "title" in r { r.title } else if "role" in r {
        r.role
      } else if "position" in r { r.position } else { "" }
      let r-dates = if "dates" in r { r.dates } else if "date" in r {
        r.date
      } else { "" }
      let r-stack = if "stack" in r { r.stack } else if "technologies" in r {
        r.technologies
      } else { none }

      role(
        r-title,
        r-dates,
        muted-color: muted-color,
        first: first-r,
        stack: r-stack,
      )
      if "bullets" in r and r.bullets.len() > 0 {
        bullets(r.bullets)
      }
      first-r = false
    }
    first-exp = false
  }
}
