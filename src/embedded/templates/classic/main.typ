// Resumake Canonical Modular Typst Engine

#import "tokens.typ": *
#import "primitives.typ": *
#import "blocks/education.typ": render-education
#import "blocks/experience.typ": render-experience
#import "blocks/projects.typ": render-projects
#import "blocks/skills.typ": render-skills
#import "blocks/publications.typ": render-publications
#import "blocks/split_line.typ": render-split-line
#import "blocks/references.typ": render-references
#import "blocks/lines.typ": render-lines, render-text

// Dynamically read content path from CLI input or default to root /content.yaml
#let content-path = sys.inputs.at("content", default: "/content.yaml")
#let data = yaml(content-path)

// Theme extraction from YAML meta.theme or fallback defaults
#let theme = if "theme" in data.meta { data.meta.theme } else { (:) }

// Golden Ratio Typography Scale
#let scale = calc-scale(11.5pt, theme)
#let BODY = scale.body
#let ORG_SIZE = scale.org
#let SEC = scale.sec
#let NAME = scale.name
#let RULE = if "rule_thickness" in theme {
  eval(str(theme.rule_thickness))
} else { 0.5pt }

// Font configuration with universal fallbacks
#let FONT = if "font_family" in theme {
  if type(theme.font_family) == array { theme.font_family } else {
    (theme.font_family, "Linux Libertine", "Times New Roman", "DejaVu Serif")
  }
} else {
  ("Linux Libertine", "Times New Roman", "DejaVu Serif")
}

#let PAPER = if "paper_size" in theme { theme.paper_size } else { "us-letter" }
#let MARGIN = if "margin" in theme { eval(str(theme.margin)) } else { 0.5in }

// Physical dimensions of the supported paper presets, mirrored here because
// Typst does not expose a preset's resolved size back to script context.
#let PAPER_DIMENSIONS = (
  "us-letter": (w: 8.5in, h: 11in),
  "a4": (w: 210mm, h: 297mm),
)
#let PAGE_DIMS = PAPER_DIMENSIONS.at(
  PAPER,
  default: PAPER_DIMENSIONS.at("us-letter"),
)

#let INK = if "ink_color" in theme { rgb(theme.ink_color) } else {
  rgb("#111111")
}
#let ACCENT = if "accent_color" in theme { rgb(theme.accent_color) } else {
  rgb("#2a2a2a")
}
#let MUTED = if "muted_color" in theme { rgb(theme.muted_color) } else {
  rgb("#444444")
}

#set document(
  title: data.meta.name + " - Resume",
  author: data.meta.name,
  keywords: if "keywords" in data.meta { data.meta.keywords } else { () },
  date: auto,
)
#set page(paper: PAPER, margin: MARGIN)
#set text(font: FONT, size: BODY, fill: INK)
#set par(justify: false, leading: 0.65em)
#show link: it => text(fill: INK)[#it]

#let render-header(meta) = {
  align(center)[
    #text(size: NAME, weight: "semibold", tracking: 0.02em)[#meta.name]
    #if "title" in meta and meta.title != "" [
      #v(-0.55em)
      #text(size: BODY + 0.5pt, style: "italic", fill: MUTED)[#meta.title]
    ]
    #v(-0.60em)
    #text(size: BODY)[
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
    ]
    #if "badge" in meta and meta.badge != "" [
      #v(-0.40em)
      #text(size: BODY - 1pt, fill: MUTED)[#meta.badge]
    ]
  ]
}

// Main Render Dispatcher
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

  // Page layout telemetry probe for the Resumake CLI
  context [
    #metadata((
      pages: counter(page).final().first(),
      y: here().position().y.pt(),
      margin: MARGIN.pt(),
      page_w: PAGE_DIMS.w.pt(),
      page_h: PAGE_DIMS.h.pt(),
    )) <pageinfo>
  ]
}

#render(data)
