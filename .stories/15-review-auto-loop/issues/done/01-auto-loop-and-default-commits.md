# Add the auto loop and commit by default

`spec.md`, all sections.

## Scope

- `.claude/skills/review-story/SKILL.md` and `.claude/skills/code-review/SKILL.md`: an "Auto mode" section with
  the loop, and a commit step after the record file is written. In review-story, amend `Scope` and the
  `Reviewer independence` note for auto mode; in code-review, step 4's author detection covers fix commits in
  rounds 2+.
- `.claude/skills/address-story-review-findings/SKILL.md` and
  `.claude/skills/address-code-review-findings/SKILL.md`: commit per treated finding in every mode, `nocommit`,
  `auto commit` removed, the "important findings" rule, dirty-file findings listed as needing the owner, and the
  failing-check rule (check per code-changing finding, revert own paths and leave open in `auto`; replaces
  address-code-review's "`scripts/check` before the last commit"). In `Finishing` and the Auto-mode finish, say
  that inside the loop the pass hands back to the orchestrator instead of recommending a fresh review or naming
  the range one must cover; run standalone, that text stays.
- `docs/agents/commit-policy.md`: the exception to its opening paragraph for these skills' own commits.
- `docs/agents/issue-tracker.md`: under "Review file", the `auto` bullet, "Only `/review-story` writes
  findings…", "by default only on a per-finding go-ahead" and the "Reviewer independence" bullet's "Human-authored
  stories are exempt" (not in the auto loop); and the "Code review file" section.
- `docs/adr/0006-review-skills-commit-by-default.md`.

## Done when

- Outside `.stories/`, no file mentions `auto commit`, and none says these skills ask before committing.
- Each review skill documents `auto [X]` with default 3, `auto`/`nocommit` as keywords and invalid X asked up
  front, the loop shape, the stop conditions (progress means a change outside the record file), the up-front
  questions (branch check, foreign-story ruling, missing inputs, offer to commit uncommitted story documents with
  its subject, `auto nocommit` refused, code-review's baseline `scripts/check`), the recorded ranges and HEADs, and the final report with its additions.
- Both address skills document per-finding commits in every mode, `nocommit`, the important-findings rule, the
  failing-check rule, and the in-loop hand-back in `Finishing`.
- `commit-policy.md` carries the exception, and ADR 0006 exists and is linked from the spec.

## Comments

- 2026-11-05: implemented, uncommitted. ADR 0006, the commit-policy exception, both review skills (step 5/8
  commit, "Auto mode") and both address skills (per-finding commits, `nocommit`, important findings, failing
  checks, in-loop hand-back), and issue-tracker.md. Two choices beyond the spec: a code-review loop whose
  `scripts/check` fails beyond the baseline stops after the next review; a standalone code-review walkthrough runs
  `scripts/check` once at the end. Needs a `/code-review` before closing (`.claude/skills/` is outside `.stories/`
  and `docs/`).
- 2026-11-05: closed. Code-reviewed by `/code-review 334b3f3 auto 3` (three rounds, C1–C15, last row
  `5390f1d..841306e`); C4, C5, C7 waived by the owner, C9–C11 settled in the spec.
