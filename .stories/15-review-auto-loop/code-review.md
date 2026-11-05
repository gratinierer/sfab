# 15-review-auto-loop: code review

Code reviews of this story's issues. The journal lists every pass: `code-review` runs of `/code-review`, and
`walkthrough` passes of `/address-code-review-findings`. The findings table lists what each run found and what became
of it. Rows are never deleted or renumbered: a fixed finding flips to `resolved`.

Statuses: `open` / `resolved` / `wontfix` / `superseded`.
Axes: `standards` / `spec`.
Conventions: `docs/agents/issue-tracker.md`, section "Code review file".

## Review journal

| Date | Kind | Reviewed | By | Findings | Notes |
|---|---|---|---|---|---|
| 2026-11-05T16:00:00+00:00 | code-review | 01 @ 334b3f3..a4a1aee | claude-sonnet-5-5 (dispatched; code by claude-opus-5-5) | 13 new: 7 standards, 6 spec | auto loop round 1 of up to 4 |

## Findings

| # | Status | Axis | Affects | Description | Comments |
|---|---|---|---|---|---|
| C1 | resolved | standards | 01, .claude/skills/address-story-review-findings/SKILL.md:153, address-code-review-findings/SKILL.md:126 | Auto mode's "ask-and-wait rules (decision, change, per-finding commit prompt)" names a commit prompt the change removed. | 2026-11-05: found. 2026-11-05: resolved (auto): dropped "per-finding commit prompt" from both Auto-mode parentheticals. |
| C2 | open | standards | 01, .claude/skills/code-review/SKILL.md:105 | Step 7's "create it from the template below" now points past the inserted "Auto mode" section; the template sits under "Final report". | 2026-11-05: found. |
| C3 | open | standards | 01, .claude/skills/code-review/SKILL.md:19 | Possible Speculative Generality: the `refs/heads/auto` edge case sits in always-read step 1 rather than "Auto mode > Arguments". | 2026-11-05: found. |
| C4 | open | standards | 01, four skills, commit-policy.md, issue-tracker.md | Possible Duplicated Code / Shotgun Surgery: the branch and story-documents check, up-front list, final report, important-findings, failing-check and hand-back text are near-verbatim in several files, so a rule change edits ~six places. | 2026-11-05: found. |
| C5 | open | standards | 01, address-*/SKILL.md "Committing" | Possible Divergent Change: "Committing" mixes commit mechanics, branch gating and story-document hygiene. | 2026-11-05: found. |
| C6 | open | standards | 01, .claude/skills/code-review/SKILL.md:147 | Possible Speculative Generality: listing `auto -1`/`auto foo` and the "run without check gating" path adds branches the spec barely needs. | 2026-11-05: found. |
| C7 | open | standards | 01, four skills | Possible Mysterious Name: "progress" is defined in four places; define it once and point to it. | 2026-11-05: found. |
| C8 | open | spec | 01, .claude/skills/review-story/SKILL.md:211 | review-story's "Up front" list lacks the spec's "an unresolvable story" item, which code-review has. | 2026-11-05: found. |
| C9 | open | spec | 01, .claude/skills/code-review/SKILL.md:172 | code-review stops clean only when no finding *against the reviewed issues* is open; spec says "a review leaves no open rows". | 2026-11-05: found. |
| C10 | open | spec | 01, .claude/skills/code-review/SKILL.md:179, address-code-review-findings/SKILL.md:80 | Beyond the spec: the stop after a `scripts/check` failure and the standalone end-of-walkthrough `scripts/check` (noted in 01's Comments, not owner-approved). | 2026-11-05: found. |
| C11 | open | spec | 01, address-*/SKILL.md "Committing" | Beyond the spec: standalone address passes offer to commit uncommitted story documents and carry on as `nocommit` on decline; spec scoped the offer to the loop's up-front stage. | 2026-11-05: found. |
| C12 | open | spec | 01, .claude/skills/address-code-review-findings/SKILL.md:96 | The dirty-file leave-open rule applies to any file, not code files only as R10 narrowed it; address-story says "code file". | 2026-11-05: found. |
| C13 | open | spec | 01, .claude/skills/review-story/SKILL.md:80 | "the user is in the loop on every one" stays unqualified right before the auto-mode exception, so the note reads self-contradictory. | 2026-11-05: found. |
