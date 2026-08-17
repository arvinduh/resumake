// Education block component

#import "../tokens.typ": *
#import "../primitives.typ": *

#let render-single-education(edu, org-size: 12pt, muted-color: rgb("#444444"), first: false) = {
  let inst = if "institution" in edu { edu.institution } else if "school" in edu { edu.school } else if "university" in edu { edu.university } else { "" }
  let loc = if "location" in edu { edu.location } else { "" }
  let deg = if "degree" in edu { edu.degree } else { "" }
  let gpa-str = if "gpa" in edu and edu.gpa != "" [ #SEP GPA: #edu.gpa] else []
  let dates-str = if "dates" in edu { edu.dates } else if "date" in edu { edu.date } else { "" }

  split-row(text(size: org-size, weight: "bold")[#inst], muted-italic(loc, muted-color: muted-color), above: if first { 0em } else { GROUP_GAP }, below: ORG_BELOW)
  split-row(italic[#deg#gpa-str], muted-italic(dates-str, muted-color: muted-color), below: ROLE_BELOW)

  if "honors" in edu and edu.honors != "" {
    line-item("Honors", if type(edu.honors) == array { edu.honors.join(", ") } else { edu.honors })
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
    line-item("Coursework", if type(edu.coursework) == array { edu.coursework.join(", ") } else { edu.coursework })
  }
  if "lines" in edu {
    for (k, v) in edu.lines.pairs() {
      line-item(k, v)
    }
  }
}

#let render-education(education, org-size: 12pt, muted-color: rgb("#444444")) = {
  if type(education) == array {
    let first = true
    for edu in education {
      render-single-education(edu, org-size: org-size, muted-color: muted-color, first: first)
      first = false
    }
  } else if type(education) == dictionary {
    render-single-education(education, org-size: org-size, muted-color: muted-color, first: true)
  }
}
