# 20-email: code review

Code reviews of this story's issues. The journal lists every pass: `code-review` runs of `/review-code`,
`check` rows for the `scripts/check` run before each review, and `walkthrough` passes of
`/address-code-review-findings`. The findings table lists what each run found and what became of it. Rows are never
deleted or renumbered: a fixed finding flips to `resolved`.

Statuses: `open` / `resolved` / `wontfix` / `superseded`.
Axes: `standards` / `spec` / `security`.
Conventions: `docs/agents/issue-tracker.md`, section "Code review file".

## Review journal

| Date | Kind | Reviewed | By | Findings | Notes |
|---|---|---|---|---|---|
| 2026-11-21T14:11:41Z | check | @ 678f267 | scripts/check | — | green |
| 2026-11-21T14:11:41Z | code-review | 01 @ 7dee701..678f267 | claude-sonnet-5-5 (dispatched; code by claude-opus-5-5) | 2 new: 2 standards, 0 spec, 0 security | auto round 1, caller mode from `/implement-story 20`, commit filter `(#20)`. Same vendor family, so weaker independence |

## Findings

| # | Status | Axis | Affects | Description | Comments |
|---|---|---|---|---|---|
| C1 | resolved | standards | 01, cdk/stacks.py:384 | The `MailTxt` record set puts the `apple-domain=` and SPF values in one apex TXT record set, which relies on Route 53 allowing one record set per name and type, but cites no `docs/aws-facts.md` row, as `CODING_STANDARDS.md` "Code that relies on AWS behaviour cites its row" asks. | 2026-11-21: found. 2026-11-21 (auto walkthrough): fixed: `docs/aws-facts.md` "Route 53, Budgets and other services" gains the row (status `docs`, Route 53 "Simple routing"), and the comment above `MailTxt` cites it; `tests/test_stacks.py` passes. |
| C2 | open | standards | 01, cdk/sfab_config.py:78 | Possible Speculative Generality: `CHECKED_OPTIONAL_KEYS` is a generic tuple mechanism for exactly one key, while that key's own checks (prefix, pattern) stay hard-coded after the loop. | 2026-11-21: found. |
