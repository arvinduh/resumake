# resumake orchestration process

This is the _process_ — how work on this repo gets planned, dispatched,
reviewed, and merged. It replaces any loose or git-ignored plan files. State
(what's ready, blocked, in review) lives entirely in GitHub issue `status:*`
labels, not in this file — this file only changes when the _process itself_
changes.

## How to use this file

One file, one path, no per-tool discovery convention to keep in sync:
**`.agents/orchestrate.md`.** If your harness auto-discovers skills from some
other directory, it won't find this automatically — that trade-off is
deliberate. Point any agent at it directly ("read `.agents/orchestrate.md`
before starting") or rely on `AGENTS.md`'s own pointer, since `AGENTS.md` itself
_is_ read automatically by both Antigravity and Claude Code.

- **Read this before dispatching or picking up any resumake work.** If you're an
  agent that just landed on this repo cold, this file plus `AGENTS.md` is
  everything you need — don't re-derive the process from git history.
- **Finding current work:** `gh issue list --label status:ready` is the _only_
  source of truth. Query labels directly instead of trusting any cached summary,
  tracking issue, or external document.

## 1. Worktree isolation

Every worker subagent operates in its own isolated git worktree
(`git worktree add`), never the shared primary working directory. Workers never
concurrently mutate another agent's active branch. Verify a worktree is actually
being used, don't assume. This rule also protects
**orchestrator-vs-orchestrator** collisions: never run two live orchestrator
sessions against the same physical local clone — worktrees all share one
`.git/`, and two processes doing concurrent `git worktree add`/push against it
can hit `.git/index.lock` contention. Separate clones, not just separate
worktrees, if you're running more than one orchestrator at once.

## 1.5. Multiple concurrent orchestrators — the dispatch race

The `status:*` label design (§11) fixes the race on the _tracking view_. **It
does not fix the race on _dispatch_ itself.** Two orchestrators can both query
`status:ready`, both see the same unclaimed issue, and both start work on it
before either writes `status:in-progress` — a plain label write has no
compare-and-swap.

**Claim, then verify, before doing any real work:**

1. Flip the issue to `status:in-progress` **and** self-assign:
   `gh issue edit <N> --add-label status:in-progress --add-assignee @me` (and
   remove `status:ready`).
2. Immediately read the issue back (`gh issue view <N> --json assignees`). If
   you're not the sole assignee, someone else's write landed first — abort,
   don't proceed, pick a different issue.
3. Only after that readback succeeds: create the worktree and dispatch.

This shrinks the race window to milliseconds and makes a collision loud (visible
on readback) instead of silent. The branch-name convention (`feat/issue-N-...`
or `fix/issue-N-...`) is a cheap backstop on top of this: if the
claim-and-verify step somehow still double-dispatches, the second worker's
`git push -u` on an already-existing remote branch fails loudly rather than
silently duplicating work.

**Review/merge ownership follows the same pattern:** whoever self-assigns a PR
for QA review owns merging it. An orchestrator that sees a PR already
`status:in-review` with a different assignee skips it — it does not also review
or merge.

## 2. Commits & presubmit

- Conventional Commits: `<type>(<scope>): <description> (Fixes #<issue>)`.
- Super well-scoped, granular commits: every commit represents exactly one
  logical change. PRs can be larger, but should be composed of multiple
  distinct, atomic commits rather than monolithic lumps.
- Architectural conventions:
  - Use modern `mod_name.rs` (and `mod_name/`) layout, never `mod.rs`.
  - Subsystem boundaries: `src/commands/` for CLI dispatch, `src/engine/` for
    in-process Typst compilation, `src/utils/` for shared cross-cutting
    plumbing, and domain models in `src/models.rs`, `src/schema.rs`,
    `src/telemetry.rs`.
  - Colocated domain errors: each subsystem defines its own domain errors,
    aggregated into `ResumakeError` in `src/error.rs` with `Result<T>` alias and
    classification helpers (`is_engine()`, `is_schema()`,
    `is_layout_overflow()`, etc.).
  - Use absolute imports (`crate::...`) across all internal modules.
  - Zero internal re-exporting (`pub use crate::...`); only `src/lib.rs` exports
    the public API.
  - Control public visibility cleanly at the module level in `src/lib.rs`.
  - Release boundaries: `rsmk release` is strictly an end-user command for
    résumé repos containing `content.yaml`. Releases of `resumake` itself are
    cut by pushing a `v*` tag directly to `main` (driven by `cargo-dist`).
- Progressive 2-tier quality gate:
  - **Tier 1 (Local pre-commit hook)**: `.githooks/pre-commit` (activated via
    `git config core.hooksPath .githooks`). Runs `fml fmt --staged` and
    `fml lint` (or `cargo fmt --check` / `cargo clippy`) plus fast unit tests
    (`cargo test --lib --quiet`) before every commit.
  - **Tier 2 (Parallel CI checks)**: `.github/workflows/ci.yml` and `schema.yml`
    run format checks, clippy, multi-platform test suites, security audit, and
    schema drift verification on every PR.
- Before every commit: standard presubmit command suite:
  `cargo test --lib -q && cargo clippy --all-targets -- -D warnings`, dogfooded
  with the freshly built binary (`cargo run -q -- build` /
  `cargo run -q -- check`), never a stale global `resumake` or `rsmk`.

## 3. CI / branch-protection changes need the orchestrator, not a worker

A worker in an isolated worktree cannot see repository branch-protection
settings. Any PR renaming a CI job, changing which workflow produces a required
check, or changing trigger conditions must be validated against current required
status checks by the orchestrator before merge — a rename that looks fine in
isolation can silently block all future merges. Changing branch protection
itself requires the user's explicit sign-off, always — an "ask first" item per
`AGENTS.md`.

## 4. Maker-checker QA gate (required for non-trivial changes)

- A separate QA reviewer subagent audits the worker's finished diff _before_
  merge, for anything touching core engine logic, models, schema generation,
  telemetry rules, or introducing new CLI behavior. Pure one-liners and typo
  fixes don't need this ceremony.
- The reviewer didn't write the code and reviews skeptically: edge cases,
  telemetry regressions, schema backward compatibility, and whether the diff
  actually satisfies the issue's acceptance criteria.
- **Debate, don't rubber-stamp** — worker and reviewer go back and forth on
  concrete objections until they converge on the best solution, not just an
  acceptable one.
- **Scope triage:** If a discovered item still fits the issue as filed, fold it
  in. If it implies new capability, a different part of the codebase, or a
  design decision the issue didn't ask for, don't scope-creep the current PR —
  file it as its own issue (`gh issue create`, topical label + `status:*` per
  §11, `Blocked-by:` if applicable, and a `Spun off from #N` line).
- **Audit/survey-shaped issues fan out by nature:** An issue whose job is
  reading across the codebase (e.g. docs backfill, telemetry audit) will
  routinely turn up more than one PR's worth of findings — that is the audit
  doing its job. Create targeted spinoff issues for orthogonal follow-ups.

## 4.5. When the orchestrator merges, and how it handles conflicts

**The orchestrator merges a PR once, in order:**

1. Required status checks are green (monitor reactively via
   `gh pr checks <PR_NUM> --watch` or `gh run watch` — never poll in ad-hoc
   loops).
2. All review conversations are resolved.
3. Either: the change was trivial enough to skip §4's ceremony entirely, or §4's
   QA debate concluded with the reviewer's written sign-off as a PR comment.

Merge method matches this repo's existing convention:
`gh pr merge --squash --delete-branch`.

**Merge conflicts, resolved by the orchestrator:**

1. In the PR's own worktree: `git fetch origin main && git merge origin/main`.
2. Resolve conflicts. **Classify the resolution before pushing it:**
   - _Textual/adjacent_ (both diffs touched nearby lines, no semantic overlap) —
     resolve directly.
   - _Semantic_ (both diffs changed the same behavior, shared models, or one
     branch's assumptions no longer hold) — do **not** silently pick one side.
     Treat the resolution itself as new implementation subject to the full §4
     gate again.
3. Re-run the full presubmit
   (`cargo test --lib -q && cargo clippy --all-targets -- -D warnings`,
   `cargo test --all-targets`) on the resolved state before pushing.
4. Push, wait for CI to go green again, then merge per the steps above.

## 4.6. Post-merge cleanup — leave nothing behind, every time

A merge is not done until the local footprint it created is gone too:

1. **Remove the worktree**: `git worktree remove <path>` (`--force` if it has
   ignorable build residue).
2. **Delete the local branch**: `git branch -d <branch>`.
3. **`git worktree prune`** periodically to catch anything removed out-of-band.
4. **Target directory growth**: Set a shared `CARGO_TARGET_DIR` (env var or
   `[build] target-dir` in `.cargo/config.toml`) so every worktree shares one
   incremental build cache instead of growing independent multi-GB caches.
5. **Dead code is a merge-gate check, not a follow-up sweep:** If a diff makes
   something unreachable (e.g. retiring old template code or inline helpers),
   the old code must be deleted in the same PR, not left behind.
6. **Local scratch tied to a closed issue** is deleted once that issue is
   closed.

## 5. Core domain principles

1. **Strict 1-Page Layout Geometry & Telemetry contract:** Resumake's core value
   proposition is zero layout regressions and deterministic 1-page guarantees.
   All embedded templates (`src/embedded/templates/`) must emit `<pageinfo>`
   metadata (pages, y, margin, page_w, page_h) and route bullets through
   `<bulletinfo>`. Telemetry calculations in `src/telemetry.rs` must never be
   bypassed.
2. **Modular Presentation vs Data Separation:** `ResumeDocument`
   (`src/models.rs`) carries structured career data; Typst templates define
   visual styling. Never couple schema fields to template-specific formatting
   tricks.
3. **Deterministic Typst compilation & Schema Stability:** Typst compiles
   pixel-perfect PDFs with identical metrics locally and in CI. `src/models.rs`
   is canonical for JSON Schema generation; avoid schema drift.

## 6. Maximize parallelism, respect real conflict risk

Dispatch every currently-unblocked issue simultaneously, each in its own
worktree — sequential dispatch of independent issues wastes wall-clock time for
no safety benefit. The one hard constraint: never run two workers concurrently
against branches whose diffs are likely to overlap (e.g. two tasks heavily
refactoring `src/engine.rs` or `src/cli.rs`). When only one issue is unblocked,
wait for it rather than jumping the dependency order.

## 7. Role-based routing, not model/vendor-based

Assume any agent is fully capable of any role, including dispatching its own
subagents. Route by **role**, not by which model happens to be running:

- **Orchestrator / QA-reviewer role** — dispatching, reviewing diffs
  skeptically, debating design (§4), scope-triage. Requires high reasoning
  depth.
- **Implementer / worker role** — following an already-scoped issue, writing
  code, running presubmit, opening a PR. Medium effort is normally sufficient.
- **On Antigravity or other harnesses:** Configure subagent roles appropriately
  (e.g. `Role: "QA Reviewer"`, `Model: "pro"` or `"inherit"` for
  reviewer/orchestrator; `Role: "Implementer"`, `Model: "flash"` or `"inherit"`
  for scoped worker tasks).

## 8. Default to orchestrator role

Whichever agent is prompted defaults to **dispatching/reviewing**, not executing
directly — even for a single non-parallelizable task. Before doing
implementation work in the current turn, ask: could this be handed to a worker
subagent in its own worktree instead? Default yes.

Direct execution is the deliberate exception, and only for these two cases:

1. Pure bookkeeping with no code-review surface (labeling issues, checking CI,
   merging an already-reviewed PR).
2. Resolving a merge conflict between two already-reviewed branches.

Anything beyond these two, however small, still gets the §4 QA gate even when
the orchestrator wrote it directly.

## 9. Design-phase stop rule

An issue whose scope requires an architectural decision or user UX sign-off gets
`status:design-phase` + a `Needs-user-design: yes` line in its body. Any agent
about to write code against such an issue stops and tells the user to align on
architecture first. It does not implement, and does not silently reinterpret the
issue to make it implementable without that conversation.

## 10. Applied-feature checkpoint

An issue introducing new user-facing CLI surface (commands, flags, terminal
output formats) requires presenting the concrete proposal — example invocation,
example output — to the user for confirmation before finalizing. Never
build-then-reveal.

## 11. Issue conventions & status labels

Every issue has:

- ≥1 topical label (`architecture`, `dx`, `documentation`, `rust`, `ci`,
  `schema`, `telemetry`, `typst`, `templates`).
- Exactly one `status:*` label:
  - `status:ready`: Unblocked, scoped, ready for immediate dispatch.
  - `status:blocked`: Blocked by another issue. Body contains `Blocked-by: #N`.
  - `status:design-phase`: Requires architecture/UX alignment with user before
    implementation (`Needs-user-design: yes`).
  - `status:in-progress`: Currently being worked on by a claimed worker.
  - `status:in-review`: Code complete, PR open, undergoing presubmit and §4 QA
    gate.

### Why state lives only on per-issue labels, not a shared document

GitHub's issue API has no optimistic concurrency (no ETag/If-Match) — two agents
editing the same shared document around the same time creates a silent
lost-update race. `status:*` labels on individual issues are narrow,
low-contention writes: two sessions touching different issues cannot collide,
and there is no aggregate snapshot to fall out of sync.
