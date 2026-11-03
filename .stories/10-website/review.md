# 10-website: review

Review record for this story. The journal lists every pass over it — reviews
and walkthroughs, distinguished by `Kind`; the
findings table lists what each pass found and what became of it. Rows are
never deleted — a fixed finding flips to `resolved`, it does not disappear.

Statuses: `open` / `resolved` / `wontfix` / `superseded`.
Axes: `factual` / `consistency` / `completeness` / `convention` / `staleness`.
Conventions: `docs/agents/issue-tracker.md`, section "Review file".

## Review journal

| Date | Kind | Reviewed | By | Findings | Notes |
|---|---|---|---|---|---|
| 2026-11-03T19:42:45+00:00 | review | 65bc6e7 | claude-fable-5-1 (dispatched; stories authored by claude-opus-5-5) | 1 new, 0 resolved, 1 still open | only the one point found while reviewing stories 2 and 4; not a full review |
| 2026-11-03T19:48:38+00:00 | walkthrough | working tree | claude-opus-5-5, auto commit | 0 new, 1 resolved, 0 still open | auto walkthrough, not a review pass |
| 2026-11-03T19:59:23+00:00 | review | 253641c | claude-fable-5-1 (dispatched; stories authored by claude-opus-5-5) | 0 new, 0 resolved, 0 still open | closure of R1 checked while reviewing stories 2 and 4; not a full review |

## Findings

| # | Status | Axis | Affects | Description | Comments |
|---|---|---|---|---|---|
| R1 | resolved | staleness | spec.md "Must include" | "It makes fake identities cost a little" undersells proof of work since ADR 0003, which puts a proof on every write. | 2026-11-03: found. 2026-11-03 (auto walkthrough): the explanation now says every write costs a proof, per ADR 0003. |
