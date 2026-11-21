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
| 2026-11-21T15:28:19Z | walkthrough | 01, 02, 03 @ 35ed5ff..HEAD | claude-opus-5-5, auto | 0 new, 6 resolved, 0 still open | auto walkthrough, not a review pass |
| 2026-11-21T15:31:11Z | check | @ a5775dd | scripts/check | — | green, the loop's check of the round 1 fixes |
| 2026-11-21T15:31:11Z | code-review | 01, 02, 03 @ 35ed5ff..a5775dd | claude-sonnet-5-5 (dispatched; code by claude-opus-5-5, orchestrated by claude-opus-5-5) | 3 new: 3 standards, 0 spec, 0 security | auto round 2, the fix commits, commit filter `(#10)`; all six fixes held. Same vendor family, so weaker independence |

## Findings

| # | Status | Axis | Affects | Description | Comments |
|---|---|---|---|---|---|
| C1 | resolved | standards | 02, lambda/core/tests/worked_example.rs:90 | Possible Duplicated Code: the Rust test re-implements the `#proof-of-work` → `<details>` extraction with string searches that `cdk/tests/test_website_pages.py` already does by parsing, and `find("<details>")` would take the wrong block if another `<details>` came first in the section. | 2026-11-21: found. 2026-11-21 (auto walkthrough): fixed: the Rust test now slices to the `#proof-of-work` section and asserts it holds exactly one `<details>`, so the slice can't take another block; the cross-language duplicate stays, since the Rust test is what ties the pages to the verifier's values. `cargo test --test worked_example` passes. |
| C2 | resolved | standards | 02, 03, cdk/generate_website.py:45 | Possible Data Clumps: `api_host`, `site_host` and `contact` travel together through `generate()` and its `.replace()` chain, so each new placeholder touches the signature, `main()` and the chain; one placeholder-to-value map would also feed the leftover check. | 2026-11-21: found. 2026-11-21 (auto walkthrough): fixed: `generate()` builds one placeholder-to-value map and fills every text file from it, replacing the `.replace()` chain; its keyword arguments stay, so callers are unchanged. `tests/test_generate_website.py` and `tests/test_website_pages.py` pass. |
| C3 | resolved | standards | 03, cdk/website_stack.py:60 | Possible Shotgun Surgery: the catalog path lives as `API_CATALOG_PATH` in `website_stack.py` and again as the bare `"api-catalog"` in `generate_website.py`'s `_TEXT_NAMES`, untied, so a rename needs edits in several places. | 2026-11-21: found. 2026-11-21 (auto walkthrough): fixed: a test in `cdk/tests/test_website_stack.py` ties `API_CATALOG_PATH` to the committed `website/.well-known/api-catalog` and to `generate_website._TEXT_NAMES`, so a rename that misses one fails; the two modules stay independent. The test passes. |
| C4 | resolved | standards | 03, cdk/website_stack.py:64 | Possible Primitive Obsession: `api_catalog_function_code` builds the CloudFront Function's JavaScript with an f-string of doubled braces; a template constant with two substitutions would read more clearly (style only, the values are quoted by `json.dumps`). | 2026-11-21: found. 2026-11-21 (auto walkthrough): fixed: the function's JavaScript is now a `string.Template` constant, `_API_CATALOG_FUNCTION`, with `$content_type` and `$link` filled as JSON strings, instead of an f-string with doubled braces. `tests/test_website_stack.py`, which runs the function in node, passes. |
| C5 | resolved | spec | 02, website/agents/index.html:28 | The agent join steps leave out what spec.md "How to join" asks: one birth per IP every few hours, or `409` `too-soon`, and that a lost birth answer burns the code and means a new code, a new proof and waiting out `too-soon`. | 2026-11-21: found. 2026-11-21 (auto walkthrough): fixed: the agent version's birth step now says one birth per IP address every few hours, or `409 too-soon` with a `retry_after`, and the keep-it-safe step says a lost answer loses the code too: a new code, a new proof and waiting out `too-soon`; a new test in `cdk/tests/test_website_pages.py` checks both and `409 try-later`. The page tests pass. |
| C6 | resolved | spec | 02, website/agents/index.html:28 | The agent join steps get a genesis code before reading the constitution, while spec.md "How to join" (after R21) reads the constitution and its Terms first, then gets a code. | 2026-11-21: found. 2026-11-21 (auto walkthrough): fixed: the agent join steps now read the constitution and its Terms first, then the API description, and only then get a genesis code, "from this route or from the owner's posts", as spec.md "How to join" lists them; the order test in `cdk/tests/test_website_pages.py` follows and checks the owner's posts. The page tests pass. |
| C7 | open | standards | 03, cdk/tests/test_website_stack.py:253 | C3's test only checks that the catalog's name is in the private `generate_website._TEXT_NAMES`, not that `generate()` fills the catalog file, against `CODING_STANDARDS.md` "checks meaning, not just text: run the pattern, not only compare it". | 2026-11-21: found. |
| C8 | open | standards | 02, cdk/tests/test_website_pages.py:346 | Possible test gap: C6's change dropped the `curl .../v1/openapi.yaml` step from the join-order test, so nothing checks that the agent page still gives it, or that it comes before the genesis code. | 2026-11-21: found. |
| C9 | open | standards | 02, 03, cdk/generate_website.py:60 | Possible Duplicated Code: the placeholder syntax is spelled twice, as `"{{" + name + "}}"` in the fill loop and as the `_PLACEHOLDER` regex, so the two could drift apart. | 2026-11-21: found. |
