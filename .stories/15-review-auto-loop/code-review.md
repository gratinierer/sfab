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
| 2026-11-05T16:30:00+00:00 | walkthrough | 01 @ e214bc6..e5685bc | claude-opus-5-5, auto (in loop) | 0 new, 6 resolved, 1 superseded, 6 still open | auto walkthrough, not a review pass |
| 2026-11-05T17:00:00+00:00 | code-review | 01 @ a4a1aee..5390f1d | claude-fable-5-1 (dispatched; code by claude-opus-5-5, fixes by the orchestrator claude-opus-5-5) | 2 new: 2 standards, 0 spec | auto loop round 2; spec fixes C1-C13 verified sound |

## Findings

| # | Status | Axis | Affects | Description | Comments |
|---|---|---|---|---|---|
| C1 | resolved | standards | 01, .claude/skills/address-story-review-findings/SKILL.md:153, address-code-review-findings/SKILL.md:126 | Auto mode's "ask-and-wait rules (decision, change, per-finding commit prompt)" names a commit prompt the change removed. | 2026-11-05: found. 2026-11-05: resolved (auto): dropped "per-finding commit prompt" from both Auto-mode parentheticals. |
| C2 | resolved | standards | 01, .claude/skills/code-review/SKILL.md:105 | Step 7's "create it from the template below" now points past the inserted "Auto mode" section; the template sits under "Final report". | 2026-11-05: found. 2026-11-05: resolved (auto): step 7 points to a new "Record template" heading over the template. |
| C3 | resolved | standards | 01, .claude/skills/code-review/SKILL.md:19 | Possible Speculative Generality: the `refs/heads/auto` edge case sits in always-read step 1 rather than "Auto mode > Arguments". | 2026-11-05: found. 2026-11-05: resolved (auto): moved the `refs/heads/auto` edge case to Auto mode > Arguments. |
| C4 | open | standards | 01, four skills, commit-policy.md, issue-tracker.md | Possible Duplicated Code / Shotgun Surgery: the branch and story-documents check, up-front list, final report, important-findings, failing-check and hand-back text are near-verbatim in several files, so a rule change edits ~six places. | 2026-11-05: found. 2026-11-05: left open (auto): needs the owner. Deduplicating means a shared doc the skills point to, which trades against ADR 0005's self-contained frozen skills; a design decision, and a waiver candidate. 2026-11-05: pass 2, unchanged, still needs the owner. |
| C5 | open | standards | 01, address-*/SKILL.md "Committing" | Possible Divergent Change: "Committing" mixes commit mechanics, branch gating and story-document hygiene. | 2026-11-05: found. 2026-11-05: left open (auto): judgement-call smell I would waive (the three concerns all gate one commit step); only the owner can waive. 2026-11-05: pass 2, unchanged, still needs the owner. |
| C6 | superseded | standards | 01, .claude/skills/code-review/SKILL.md:147 | Possible Speculative Generality: listing `auto -1`/`auto foo` and the "run without check gating" path adds branches the spec barely needs. | 2026-11-05: found. 2026-11-05: superseded: the owner's recorded decisions R12 (invalid X asked up front) and R16 (run without check gating) in spec.md "Decided later" ask for both. |
| C7 | open | standards | 01, four skills | Possible Mysterious Name: "progress" is defined in four places; define it once and point to it. | 2026-11-05: found. 2026-11-05: left open (auto): defining "progress" once means a shared doc, the same design trade-off as C4. 2026-11-05: pass 2, unchanged, still needs the owner. |
| C8 | resolved | spec | 01, .claude/skills/review-story/SKILL.md:211 | review-story's "Up front" list lacks the spec's "an unresolvable story" item, which code-review has. | 2026-11-05: found. 2026-11-05: resolved (auto): review-story "Up front" now asks about an unresolvable story. |
| C9 | open | spec | 01, .claude/skills/code-review/SKILL.md:172 | code-review stops clean only when no finding *against the reviewed issues* is open; spec says "a review leaves no open rows". | 2026-11-05: found. 2026-11-05: left open (auto): the implementation's narrower rule fits a loop that only addresses the reviewed issues, but it departs from spec "a review leaves no open rows"; the owner picks spec or skill. Recommendation: amend the spec to "no open finding against the reviewed issues". 2026-11-05: pass 2, unchanged, still needs the owner. |
| C10 | open | spec | 01, .claude/skills/code-review/SKILL.md:179, address-code-review-findings/SKILL.md:80 | Beyond the spec: the stop after a `scripts/check` failure and the standalone end-of-walkthrough `scripts/check` (noted in 01's Comments, not owner-approved). | 2026-11-05: found. 2026-11-05: left open (auto): behaviour beyond the spec that the owner hasn't approved, a design decision. 2026-11-05: pass 2, unchanged, still needs the owner. |
| C11 | open | spec | 01, address-*/SKILL.md "Committing" | Beyond the spec: standalone address passes offer to commit uncommitted story documents and carry on as `nocommit` on decline; spec scoped the offer to the loop's up-front stage. | 2026-11-05: found. 2026-11-05: left open (auto): behaviour beyond the spec that the owner hasn't approved; also decides how C12 reads for story documents. |
| C12 | resolved | spec | 01, .claude/skills/address-code-review-findings/SKILL.md:96 | The dirty-file leave-open rule applies to any file, not code files only as R10 narrowed it; address-story says "code file". | 2026-11-05: found. 2026-11-05: resolved (auto): address-code's dirty-file rule now says "a code file", matching R10 and address-story. |
| C13 | resolved | spec | 01, .claude/skills/review-story/SKILL.md:80 | "the user is in the loop on every one" stays unqualified right before the auto-mode exception, so the note reads self-contradictory. | 2026-11-05: found. 2026-11-05: resolved (auto): the note now says the address skill needs no guard "when run interactively", before the auto-mode exception. |
| C14 | resolved | standards | 01, .claude/skills/code-review/SKILL.md:109 | Step 7 says the template is "at the end of this file", but `## Why two axes` follows `## Record template`. | 2026-11-05: found. 2026-11-05: resolved (auto): step 7 now says "the template under \"Record template\"", without "at the end". |
| C15 | resolved | standards | 01, .claude/skills/address-story-review-findings/SKILL.md:153, address-code-review-findings/SKILL.md:126 | Possible noise: the C1 edit left the Auto-mode paragraphs unreflowed, with a short orphan line. | 2026-11-05: found. 2026-11-05: resolved (auto): reflowed both Auto-mode paragraphs. |
