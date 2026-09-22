// Resumake Design Tokens: Golden-ratio typography, spacing, and theme colors.

#let PHI = 1.618

// Golden-Ratio Modular Scale relative to BASE_BODY
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

// Theme overrides, read here so the spacing rhythm below can honour them.
#let THEME = {
  let data = yaml(sys.inputs.at("content", default: "/content.yaml"))
  let meta = data.at("meta", default: (:))
  if meta != none and "theme" in meta { meta.theme } else { (:) }
}

// Spacing rhythm.
//
// Typst measures leading and block spacing from one line's baseline to the
// next line's cap height, so the baseline-to-baseline pitch is roughly
// `LEADING + cap-height` (~1.1em for most serifs). Every other gap is a
// fixed multiple of LEADING, so one `theme.leading` value re-spaces the whole
// document consistently. Gaps between consecutive single-line rows are never
// below 1x, otherwise separate rows sit tighter than the wrapped lines of a
// paragraph and ascenders collide with the descenders above them.
//
// All values resolve `em` against the body text (block spacing does not see
// the enlarged name or section sizes), so they scale with `font_size` too.
#let LEADING = if "leading" in THEME { eval(str(THEME.leading)) } else {
  0.52em
}
#let space(ratio) = LEADING * ratio

#let BULLET_GAP = space(1.0) // bullet to bullet
#let LINE_GAP = space(1.0) // skill / award / freeform rows
#let ROLE_BELOW = space(1.0) // role row to its first bullet
#let ORG_BELOW = space(1.0) // organisation row to its first role
#let ROLE_ABOVE = space(1.2) // between roles inside one organisation
#let GROUP_GAP = space(1.5) // between organisations / projects / degrees
#let SEC_ABOVE = space(1.9) // above a section title
#let RULE_GAP = space(0.4) // section title to its rule
#let RULE_BELOW = space(1.0) // rule to the section's first row
#let NAME_BELOW = space(1.35) // name to title / contact row
#let HEADER_GAP = space(0.75) // between the header's secondary rows

// Bullets sit visibly inside the text column (indent) with room between
// the marker and the text (body-indent); at 0.3em the markers read as
// flush with the section's left edge.
#let BULLET_INDENT = 0.55em
#let BULLET_BODY_INDENT = 0.50em

// Vertical fill. Leftover page height is shared out across the section and
// entry gaps (weights set by `flex-gap` calls), but each unit of weight may
// grow by at most STRETCH x LEADING, so a short résumé gets gently looser
// rather than spread thin to the bottom margin. `theme.stretch: 0` turns the
// behaviour off.
#let STRETCH = if "stretch" in THEME { float(THEME.stretch) } else { 1.0 }

// Symbols & Separators
#let SEP = [ · ]
#let DASH = text(" – ")
