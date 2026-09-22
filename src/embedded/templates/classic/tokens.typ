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

// Spacing design tokens.
//
// Typst measures leading and block spacing from one line's baseline to the
// next line's cap height, so the baseline-to-baseline pitch is roughly
// `LEADING + cap-height` (~1.1em for most serifs). Every gap between
// consecutive single-line rows must be at least LEADING, otherwise separate
// rows sit tighter than the wrapped lines of a paragraph and ascenders
// collide with the descenders above them.
#let LEADING = 0.52em
#let BULLET_GAP = LEADING
#let LINE_GAP = LEADING
#let ROLE_BELOW = LEADING
#let ORG_BELOW = LEADING
#let ROLE_ABOVE = 0.62em
#let GROUP_GAP = 0.78em
#let SEC_ABOVE = 1.00em
#let RULE_GAP = 0.20em
#let RULE_BELOW = 0.50em

// Symbols & Separators
#let SEP = [ · ]
#let DASH = text(" - ")
