<!-- markdownlint-disable MD041 -->

## Summary

<!-- High-level summary of what this PR changes and why. -->

Fixes #<!-- issue number -->

## Changes

<!-- Bulleted list of specific changes made in this diff. -->

-

## Verification

<!-- Commands run and results observed (unit tests, clippy, dogfooding) -->

- [ ] `cargo test --lib -q`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test --all-targets`
- [ ] Staged pre-commit hook passed (`.githooks/pre-commit` /
      `fml fmt --staged`)

## Maker-Checker QA Review

<!-- For non-trivial changes, record the independent QA review findings. -->

- **QA Reviewer**: <!-- Subagent role / name -->
- **Review Findings**:
  <!-- Summary of edge case analysis, telemetry checks, or debate points -->
- **Sign-off**: <!-- Approved / Ready to merge -->

## Checklist

- [ ] Conventional Commit format followed
      (`type(scope): description (Fixes #N)`).
- [ ] Telemetry and 1-page layout contracts preserved.
- [ ] No extraneous dead code or untracked scratch files left behind.
