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

// Spacing design tokens
#let SEC_ABOVE = 1.00em
#let RULE_BELOW = 0.42em
#let GROUP_GAP = 0.80em
#let ORG_BELOW = 0.44em
#let ROLE_ABOVE = 0.46em
#let ROLE_BELOW = 0.34em
#let BULLET_GAP = 0.34em
#let LINE_GAP = 0.30em

// Symbols & Separators
#let SEP = [ · ]
#let DASH = text(" - ")
