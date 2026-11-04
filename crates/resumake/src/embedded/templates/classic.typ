// Resumake Canonical Modular Typst Engine
// Consolidated single-file layout template: tokens, primitives, blocks, and telemetry.

// ---------------------------------------------------------------------------
// 1. Data and Theme Resolution
// ---------------------------------------------------------------------------
#let content-path = sys.inputs.at("content", default: "/content.yaml")
#let data = yaml(content-path)
#let meta = data.at("meta", default: (:))
#let THEME = if meta != none and "theme" in meta { meta.theme } else { (:) }

// ---------------------------------------------------------------------------
// 2. Design Tokens and Typography Scale
// ---------------------------------------------------------------------------
#let PHI = 1.618

#let calc-scale(base, theme) = {
  let body = if "body_size" in theme {
    eval(str(theme.body_size))
  } else if "font_size" in theme {
    eval(str(theme.font_size))
  } else {
    base
  }
  let org = if "org_size" in theme {
    eval(str(theme.org_size))
  } else {
    body * (12.0 / 11.5)
  }
  let sec = if "section_size" in theme {
    eval(str(theme.section_size))
  } else {
    body * (13.0 / 11.5)
  }
  let name = if "name_size" in theme {
    eval(str(theme.name_size))
  } else {
    body * (25.0 / 11.5)
  }
  (body: body, org: org, sec: sec, name: name)
}

#let scale = calc-scale(11pt, THEME)
#let BODY = scale.body
#let ORG_SIZE = scale.org
#let SEC = scale.sec
#let NAME = scale.name
#let RULE = if "rule_thickness" in THEME {
  eval(str(THEME.rule_thickness))
} else { 0.5pt }

#let DENSITY = if "density" in THEME { THEME.density } else { "comfortable" }

#let LEADING = if "leading" in THEME {
  eval(str(THEME.leading))
} else if DENSITY == "compact" {
  0.46em
} else if DENSITY == "spacious" {
  0.58em
} else {
  0.52em
}
#let space(ratio) = LEADING * ratio

#let BULLET_GAP = space(1.0)
#let LINE_GAP = space(1.0)
#let ROLE_BELOW = space(1.0)
#let ORG_BELOW = space(1.0)
#let ROLE_ABOVE = space(1.2)
#let GROUP_GAP = space(1.5)
#let SEC_ABOVE = space(1.9)
#let RULE_GAP = space(0.4)
#let RULE_BELOW = space(1.0)
#let NAME_BELOW = space(1.35)
#let HEADER_GAP = space(0.75)

#let BULLET_INDENT = if "bullet_indent" in THEME {
  eval(str(THEME.bullet_indent))
} else if DENSITY == "compact" {
  0.35em
} else {
  0.55em
}

#let BULLET_BODY_INDENT = if "bullet_body_indent" in THEME {
  eval(str(THEME.bullet_body_indent))
} else {
  0.50em
}

#let BULLET_MARKER = if "bullet_marker" in THEME {
  let m = THEME.bullet_marker
  if m == "circle" { [○] } else if m == "square" { [▪] } else if (
    m == "dash" or m == "hyphen"
  ) { [–] } else if m == "triangle" { [‣] } else { [•] }
} else {
  [•]
}

#let NAME_WEIGHT = if "name_weight" in THEME { THEME.name_weight } else { "semibold" }
#let SECTION_WEIGHT = if "section_weight" in THEME { THEME.section_weight } else { "semibold" }
#let SECTION_CASE = if "section_case" in THEME { THEME.section_case } else { "uppercase" }
#let SECTION_STYLE = if "section_style" in THEME { THEME.section_style } else { "underline" }
#let HEADER_ALIGN = if ("header_align" in THEME and THEME.header_align == "left") { left } else { center }

#let STRETCH = if "stretch" in THEME {
  float(THEME.stretch)
} else if DENSITY == "compact" {
  0.0
} else {
  1.0
}

#let ACCENT = if "accent_color" in THEME {
  rgb(THEME.accent_color)
} else {
  rgb("#1b365d")
}

#let MUTED = if "muted_color" in THEME {
  rgb(THEME.muted_color)
} else {
  rgb("#444444")
}

#let MARGIN = if "margin" in THEME {
  eval(str(THEME.margin))
} else if DENSITY == "compact" {
  0.35in
} else if DENSITY == "spacious" {
  0.6in
} else {
  0.5in
}

#let SEP = [ · ]
#let DASH = text(" – ")

#let FONT_FALLBACKS = (
  "Libertinus Serif",
  "Linux Libertine",
  "New Computer Modern",
  "Times New Roman",
  "DejaVu Serif",
)
#let FONT = if "font_family" in THEME {
  if type(THEME.font_family) == array { THEME.font_family } else {
    (THEME.font_family,) + FONT_FALLBACKS
  }
} else {
  FONT_FALLBACKS
}

#let PAGE_DIMS = if "paper" in THEME {
  if THEME.paper == "a4" {
    (w: 210mm, h: 297mm)
  } else {
    (w: 8.5in, h: 11in)
  }
} else {
  (w: 8.5in, h: 11in)
}

// ---------------------------------------------------------------------------
// 3. Document Setup and Typography Rules
// ---------------------------------------------------------------------------
#set page(
  width: PAGE_DIMS.w,
  height: PAGE_DIMS.h,
  margin: (x: MARGIN, top: MARGIN, bottom: MARGIN),
)
#set text(
  font: FONT,
  size: BODY,
  lang: "en",
  hyphenate: false,
)
#set par(
  leading: LEADING,
  justify: false,
)

#show " - ": [ – ]
#show "'": "’"
#show regex("\"[^\"]*\""): it => "“" + it.text.slice(1, -1) + "”"

// ---------------------------------------------------------------------------
// 4. Primitives and Telemetry Probes
// ---------------------------------------------------------------------------
#let bulletinfo-counter = counter("resumake-bulletinfo")

#let guard(body, kind: "bullet") = layout(size => context {
  bulletinfo-counter.step()
  let n = bulletinfo-counter.get().first()
  let nat = measure(body)
  let fill = calc.round(nat.width / size.width * 100.0, digits: 1)
  let height = measure(par(body), width: size.width).height
  let line = measure(par[x], width: size.width).height
  let chars = repr(body).clusters()
  [#metadata((
    id: kind + "-" + str(n),
    kind: kind,
    fill: fill,
    wrapped: height > line * 1.5,
    text: chars.slice(0, calc.min(80, chars.len())).join(default: ""),
  )) <bulletinfo>]
  par(body)
})

#let flex-counter = counter("resumake-flex")
#let flex-gap(weight, base) = {
  flex-counter.update(n => n + int(weight * 100))
  v(base)
  v(weight * 1fr)
}

#let bold(body) = text(weight: "bold")[#body]
#let italic(body) = text(style: "italic")[#body]
#let bold-italic(body) = text(weight: "bold", style: "italic")[#body]
#let muted-italic(body, muted-color: MUTED) = text(style: "italic", fill: muted-color)[#body]

#let format-section-title(title) = {
  if SECTION_CASE == "uppercase" {
    upper(title)
  } else if SECTION_CASE == "small-caps" {
    smallcaps(title)
  } else {
    title
  }
}

#let section(
  title,
  sec-size: SEC,
  accent-color: ACCENT,
  rule-thick: RULE,
) = {
  flex-gap(2, SEC_ABOVE)
  let bottom-stroke = if SECTION_STYLE == "none" { none } else {
    rule-thick + accent-color
  }
  let inset-val = if SECTION_STYLE == "none" { (bottom: 0em) } else {
    (bottom: RULE_GAP)
  }
  block(
    above: SEC_ABOVE,
    below: RULE_BELOW,
    width: 100%,
    inset: inset-val,
    stroke: (bottom: bottom-stroke),
    heading(level: 2, text(
      size: sec-size,
      weight: SECTION_WEIGHT,
      tracking: 0.08em,
    )[#format-section-title(title)]),
  )
}

#let split-row(left, right, above: 0em, below: 0em, flex: 0) = {
  if flex > 0 { flex-gap(flex, above) }
  block(above: above, below: below, par[#left #h(1fr) #right])
}

#let line-item(cat, body) = {
  block(above: LINE_GAP, below: LINE_GAP)[
    #guard(kind: "line")[#bold[#cat:] #body]
  ]
}

#let bullets(items) = {
  set list(
    marker: text(size: 0.85em)[#BULLET_MARKER],
    indent: BULLET_INDENT,
    body-indent: BULLET_BODY_INDENT,
    spacing: BULLET_GAP,
  )
  block(above: 0em, below: 0em, list(..items.map(b => guard([#b]))))
}

// ---------------------------------------------------------------------------
// 5. Component Block Renderers
// ---------------------------------------------------------------------------
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
  muted-color: MUTED,
  first: false,
  stack: none,
) = split-row(
  format-role-title(title, stack: stack),
  muted-italic(dates, muted-color: muted-color),
  above: if first { 0em } else { ROLE_ABOVE },
  flex: if first { 0 } else { 0.5 },
  below: ROLE_BELOW,
)

#let render-experience(
  experience,
  org-size: ORG_SIZE,
  muted-color: MUTED,
) = {
  let first-exp = true
  let exp-list = if type(experience) == array { experience } else { (experience,) }
  for exp in exp-list {
    let org-name = if "org" in exp { exp.org } else if "company" in exp {
      exp.company
    } else { exp.organization }
    let loc = if "location" in exp { exp.location } else { "" }

    split-row(
      text(size: org-size, weight: "bold")[#org-name],
      muted-italic(loc, muted-color: muted-color),
      above: if first-exp { 0em } else { GROUP_GAP },
      flex: if first-exp { 0 } else { 1 },
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
      let r-stack = if "stack" in r { r.stack } else { none }
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

#let render-education(
  education,
  org-size: ORG_SIZE,
  muted-color: MUTED,
) = {
  let first-edu = true
  let edu-list = if type(education) == array { education } else { (education,) }
  for edu in edu-list {
    let inst = if "institution" in edu { edu.institution } else if (
      "school" in edu
    ) { edu.school } else if "university" in edu { edu.university } else { "" }
    let loc = if "location" in edu { edu.location } else { "" }
    let deg = if "degree" in edu { edu.degree } else { "" }
    let gpa-str = if "gpa" in edu and edu.gpa != "" [ #SEP GPA: #edu.gpa] else []
    let dates-str = if "dates" in edu { edu.dates } else if "date" in edu {
      edu.date
    } else { "" }

    split-row(
      text(size: org-size, weight: "bold")[#inst],
      muted-italic(loc, muted-color: muted-color),
      above: if first-edu { 0em } else { GROUP_GAP },
      flex: if first-edu { 0 } else { 1 },
      below: ORG_BELOW,
    )
    split-row(
      italic[#deg#gpa-str],
      muted-italic(dates-str, muted-color: muted-color),
      below: ROLE_BELOW,
    )

    if "honors" in edu and edu.honors != "" {
      line-item("Honors", if type(edu.honors) == array {
        edu.honors.join(", ")
      } else { edu.honors })
    }
    if "thesis" in edu {
      if type(edu.thesis) == str {
        line-item("Thesis", edu.thesis)
      } else if type(edu.thesis) == dictionary {
        let t-title = edu.thesis.at("title", default: "")
        let t-adv = if "advisor" in edu.thesis [ (Advisor: #edu.thesis.advisor)] else []
        line-item("Thesis", [#t-title#t-adv])
      }
    }
    if "coursework" in edu and edu.coursework != "" {
      line-item("Coursework", if type(edu.coursework) == array {
        edu.coursework.join(", ")
      } else { edu.coursework })
    }
    if "lines" in edu {
      for (k, v) in edu.lines.pairs() {
        line-item(k, v)
      }
    }
    if "bullets" in edu and edu.bullets.len() > 0 {
      bullets(edu.bullets)
    }
    first-edu = false
  }
}

#let render-projects(projects, muted-color: MUTED) = {
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

    let left = if stack != "" and stack != none {
      [#bold(name)#DASH#italic(stack)]
    } else {
      bold(name)
    }
    if url != none and url != "" {
      left = link(url)[#left]
    }

    split-row(
      left,
      muted-italic(d, muted-color: muted-color),
      above: if first-p { 0em } else { GROUP_GAP },
      flex: if first-p { 0 } else { 1 },
      below: ROLE_BELOW,
    )
    if "bullets" in proj and proj.bullets.len() > 0 {
      bullets(proj.bullets)
    }
    first-p = false
  }
}

#let render-skills(skills) = {
  if type(skills) == dictionary {
    for (k, v) in skills.pairs() {
      let val-str = if type(v) == array { v.join(", ") } else { v }
      line-item(k, val-str)
    }
  } else if type(skills) == array {
    for s in skills {
      if type(s) == dictionary {
        let cat = if "category" in s { s.category } else if "name" in s {
          s.name
        } else { "" }
        let items = if "items" in s {
          if type(s.items) == array { s.items.join(", ") } else { s.items }
        } else { "" }
        line-item(cat, items)
      }
    }
  }
}

#let render-split-line(items, body-size: BODY, muted-color: MUTED) = {
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
      block(above: LINE_GAP, below: LINE_GAP)[#par(text(
        size: body-size - 0.5pt,
        fill: muted-color,
      )[#it.summary])]
    }
    first-item = false
  }
}

#let render-publications(publications, body-size: BODY, muted-color: MUTED) = {
  let first-pub = true
  let pub-list = if type(publications) == array { publications } else { (publications,) }
  for pub in pub-list {
    let title = pub.title
    let venue = if "venue" in pub { pub.venue } else if "journal" in pub {
      pub.journal
    } else if "conference" in pub { pub.conference } else { "" }
    let d = if "year" in pub { str(pub.year) } else if "date" in pub {
      pub.date
    } else { "" }
    let authors = if "authors" in pub {
      if type(pub.authors) == array { pub.authors.join(", ") } else {
        pub.authors
      }
    } else { "" }

    let pub-line = [#bold(title)#DASH#italic(venue)]
    if "url" in pub and pub.url != "" {
      pub-line = link(pub.url)[#pub-line]
    } else if "doi" in pub and pub.doi != "" {
      pub-line = link(pub.doi)[#pub-line]
    }

    split-row(
      pub-line,
      muted-italic(d, muted-color: muted-color),
      above: if first-pub { 0em } else { GROUP_GAP },
      flex: if first-pub { 0 } else { 1 },
      below: ROLE_BELOW,
    )
    if authors != "" {
      block(above: LINE_GAP, below: LINE_GAP)[#par(text(
        size: body-size - 0.5pt,
        fill: muted-color,
      )[#authors])]
    }
    if "notes" in pub and pub.notes != "" {
      block(above: LINE_GAP, below: LINE_GAP)[#par(text(
        size: body-size - 0.5pt,
        style: "italic",
      )[#pub.notes])]
    }
    first-pub = false
  }
}

#let render-references(references, muted-color: MUTED) = {
  let items = if type(references) == array { references } else if (
    type(references) == dictionary and "items" in references
  ) { references.items } else { () }

  let refrow(a, b) = {
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

  let ref-item(name, role, org) = [#bold(name)#DASH#italic(role) #h(1fr) #muted-italic(org, muted-color: muted-color)]

  let i = 0
  while i < items.len() {
    if i + 1 < items.len() {
      refrow(
        ref-item(items.at(i).name, items.at(i).role, items.at(i).org),
        ref-item(items.at(i + 1).name, items.at(i + 1).role, items.at(i + 1).org),
      )
    } else {
      refrow(
        ref-item(items.at(i).name, items.at(i).role, items.at(i).org),
        [],
      )
    }
    i += 2
  }
}

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

// ---------------------------------------------------------------------------
// 6. Header and Dispatcher
// ---------------------------------------------------------------------------
#let render-header(meta) = {
  align(HEADER_ALIGN)[
    #block(below: NAME_BELOW, heading(level: 1, text(
      size: NAME,
      weight: NAME_WEIGHT,
      tracking: 0.02em,
    )[#meta.name]))
    #if "title" in meta and meta.title != "" {
      block(below: HEADER_GAP, text(
        size: BODY + 0.5pt,
        style: "italic",
        fill: MUTED,
      )[#meta.title])
    }
    #par(text(size: BODY)[
      #{
        let contact-items = ()
        if "contact" in meta {
          for item in meta.contact {
            if type(item) == str {
              contact-items.push([#item])
            } else if type(item) == dictionary {
              let label = if "label" in item { item.label } else if (
                "name" in item
              ) { item.name } else if "value" in item { item.value } else { "" }
              if "link" in item and item.link != "" {
                contact-items.push(link(item.link)[#label])
              } else if "url" in item and item.url != "" {
                contact-items.push(link(item.url)[#label])
              } else {
                contact-items.push([#label])
              }
            }
          }
        }
        contact-items.join(SEP)
      }
    ])
    #if "badge" in meta and meta.badge != "" {
      block(above: HEADER_GAP, text(size: BODY - 1pt, fill: MUTED)[#meta.badge])
    }
  ]
}

#let render(data) = {
  render-header(data.meta)

  if "sections" in data {
    for sec in data.sections {
      if "title" in sec and sec.title != "" {
        section(
          sec.title,
          sec-size: SEC,
          accent-color: ACCENT,
          rule-thick: RULE,
        )
      }

      let sec-type = if "type" in sec { sec.type } else { none }

      if sec-type == "education" or "education" in sec {
        let content = if "education" in sec { sec.education } else if (
          "items" in sec
        ) { sec.items } else { sec }
        render-education(content, org-size: ORG_SIZE, muted-color: MUTED)
      } else if sec-type == "skills" or "skills" in sec {
        let content = if "skills" in sec { sec.skills } else if "items" in sec {
          sec.items
        } else { sec }
        render-skills(content)
      } else if sec-type == "experience" or "experience" in sec {
        let content = if "experience" in sec { sec.experience } else if (
          "items" in sec
        ) { sec.items } else { sec }
        render-experience(content, org-size: ORG_SIZE, muted-color: MUTED)
      } else if sec-type == "projects" or "projects" in sec {
        let content = if "projects" in sec { sec.projects } else if (
          "items" in sec
        ) { sec.items } else { sec }
        render-projects(content, muted-color: MUTED)
      } else if sec-type == "publications" or "publications" in sec {
        let content = if "publications" in sec { sec.publications } else if (
          "items" in sec
        ) { sec.items } else { sec }
        render-publications(content, body-size: BODY, muted-color: MUTED)
      } else if (
        sec-type == "split_line"
          or sec-type == "certifications"
          or sec-type == "awards"
          or sec-type == "speaking"
          or "certifications" in sec
          or "awards" in sec
          or "speaking" in sec
      ) {
        let content = if "certifications" in sec {
          sec.certifications
        } else if "awards" in sec { sec.awards } else if "speaking" in sec {
          sec.speaking
        } else if "items" in sec { sec.items } else { sec }
        render-split-line(content, body-size: BODY, muted-color: MUTED)
      } else if (
        sec-type == "references" or sec-type == "columns" or "references" in sec
      ) {
        let content = if "references" in sec { sec.references } else if (
          "items" in sec
        ) { sec.items } else { sec }
        render-references(content, muted-color: MUTED)
      } else if sec-type == "lines" or "lines" in sec {
        let content = if "lines" in sec { sec.lines } else if "items" in sec {
          sec.items
        } else { sec }
        render-lines(content)
      } else if sec-type == "bullets" or "bullets" in sec {
        let content = if "bullets" in sec { sec.bullets } else if (
          "items" in sec
        ) { sec.items } else { sec }
        bullets(content)
      } else if sec-type == "text" or "text" in sec {
        let content = if "text" in sec { sec.text } else { "" }
        render-text(content)
      }
    }
  }
}

// ---------------------------------------------------------------------------
// 7. Page Probe Telemetry and Elastic Flex-Fit Layout
// ---------------------------------------------------------------------------
#let page-probe(y) = context [
  #metadata((
    pages: counter(page).final().first(),
    y: if y == auto { here().position().y.pt() } else { y.pt() },
    margin: MARGIN.pt(),
    page_w: PAGE_DIMS.w.pt(),
    page_h: PAGE_DIMS.h.pt(),
  )) <pageinfo>
]

#let fit-page(body) = layout(size => context {
  let natural = measure(block(width: size.width, body)).height
  let weight = flex-counter.final().first() / 100
  let slack = calc.min(
    size.height - natural,
    weight * STRETCH * measure(v(LEADING)).height,
  )
  if natural > size.height or slack <= 0pt {
    body
    page-probe(auto)
  } else {
    block(height: natural + slack, width: 100%, body)
    page-probe(here().position().y + natural)
  }
})

#fit-page(render(data))
