# 10-website: code review

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
| 2026-11-21T15:26:02Z | check | @ 6ae7677 | scripts/check | — | green, the baseline from `/implement-story 10` |
| 2026-11-21T15:26:02Z | code-review | 01, 02, 03 @ 6ae7677..35ed5ff | claude-sonnet-5-5 (dispatched; code by claude-opus-5-5) | 6 new: 4 standards, 2 spec, 0 security | auto round 1, caller mode from `/implement-story 10`, commit filter `(#10)`. Same vendor family, so weaker independence |

## Findings

| # | Status | Axis | Affects | Description | Comments |
|---|---|---|---|---|---|
| C1 | open | standards | 02, lambda/core/tests/worked_example.rs:90 | Possible Duplicated Code: the Rust test re-implements the `#proof-of-work` → `<details>` extraction with string searches that `cdk/tests/test_website_pages.py` already does by parsing, and `find("<details>")` would take the wrong block if another `<details>` came first in the section. | 2026-11-21: found. |
| C2 | open | standards | 02, 03, cdk/generate_website.py:45 | Possible Data Clumps: `api_host`, `site_host` and `contact` travel together through `generate()` and its `.replace()` chain, so each new placeholder touches the signature, `main()` and the chain; one placeholder-to-value map would also feed the leftover check. | 2026-11-21: found. |
| C3 | open | standards | 03, cdk/website_stack.py:60 | Possible Shotgun Surgery: the catalog path lives as `API_CATALOG_PATH` in `website_stack.py` and again as the bare `"api-catalog"` in `generate_website.py`'s `_TEXT_NAMES`, untied, so a rename needs edits in several places. | 2026-11-21: found. |
| C4 | open | standards | 03, cdk/website_stack.py:64 | Possible Primitive Obsession: `api_catalog_function_code` builds the CloudFront Function's JavaScript with an f-string of doubled braces; a template constant with two substitutions would read more clearly (style only, the values are quoted by `json.dumps`). | 2026-11-21: found. |
| C5 | open | spec | 02, website/agents/index.html:28 | The agent join steps leave out what spec.md "How to join" asks: one birth per IP every few hours, or `409` `too-soon`, and that a lost birth answer burns the code and means a new code, a new proof and waiting out `too-soon`. | 2026-11-21: found. |
| C6 | open | spec | 02, website/agents/index.html:28 | The agent join steps get a genesis code before reading the constitution, while spec.md "How to join" (after R21) reads the constitution and its Terms first, then gets a code. | 2026-11-21: found. |
