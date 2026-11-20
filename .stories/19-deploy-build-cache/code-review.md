# 19-deploy-build-cache: code review

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
| 2026-11-20T19:20:00+00:00 | check | @ a4bda63 | scripts/check | — | green (407 passed), the baseline before the run, from /implement-story 19 |
| 2026-11-20T19:20:00+00:00 | code-review | 01, 02, 03 @ 71b7c034e0..a4bda63 | claude-sonnet-5-5 (dispatched; code by claude-opus-5-5) | 2 new: 1 standards, 1 spec, 0 security | auto round 1, caller mode (implement-story), commits filtered by (#19); same-family reviewers, weaker independence |

## Findings

| # | Status | Axis | Affects | Description | Comments |
|---|---|---|---|---|---|
| C1 | resolved | standards | 02, lambda/world/federation/build.rs | Possible Mysterious Name: the `(name, parse)` tuples and the `(|text: &str| files::parse_world(text).map(drop)) as fn(&str) -> Result<(), String>` cast exist only to coerce the first closure's type; two calls to a small `check(name, parse)` helper would read more plainly. | 2026-11-20: found. 2026-11-20 (auto walkthrough): fixed: `build.rs` calls `check(&federation, name, parse)` twice, with `parse` a plain `fn(&str) -> Result<T, String>`, so the tuples and the cast are gone; `cargo build`/`test --test directory`/`fmt --check`/`clippy -D warnings` on `sfab-federation` pass, and a broken `world.yaml` still fails the build. |
| C2 | resolved | spec | 01, docs/aws-facts.md | The CodeBuild S3 caching-permissions row was set to `tested` in 6b14d6c, but issue 01's Done when says to set it to `tested` only after the owner's second build shows no `Unable to download cache`; the build log proves only that `s3:ListBucket` was missing, not that the full list restores the cache. | 2026-11-20: found. 2026-11-20 (auto walkthrough): fixed: the row's status is now `docs, tested in part` (the top rules allow a mixed status, as `docs, forum`), and its source says `s3:ListBucket` was seen missing, not yet seen to be enough, and becomes `tested` once a build restores the cache, per issue 01. |
