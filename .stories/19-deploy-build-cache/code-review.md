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
| 2026-11-20T19:35:00+00:00 | walkthrough | 6ca30ab | claude-opus-5-5 (auto) | 0 new, 2 resolved, 0 still open | auto walkthrough, not a review pass |
| 2026-11-20T19:50:00+00:00 | check | @ afa2e4a | scripts/check | — | green, the loop's check of the round 1 fixes |
| 2026-11-20T19:50:00+00:00 | code-review | 01, 02, 03 @ a4bda63..afa2e4a | claude-sonnet-5-5 (dispatched; code by claude-opus-5-5, fixes by the orchestrator claude-opus-5-5) | 1 new: 1 standards, 0 spec, 0 security | auto round 2, fix commits only; same-family reviewers |
| 2026-11-20T19:55:00+00:00 | walkthrough | 72ef9aa | claude-opus-5-5 (auto) | 0 new, 1 resolved, 0 still open | auto walkthrough, not a review pass |
| 2026-11-20T20:10:00+00:00 | check | @ 2b1f222 | scripts/check | — | green, the loop's check of the round 2 fix |
| 2026-11-20T20:10:00+00:00 | code-review | 01, 02, 03 @ afa2e4a..2b1f222 | claude-sonnet-5-5 (dispatched; code by claude-opus-5-5, fixes by the orchestrator claude-opus-5-5) | 1 new: 0 standards, 1 spec, 0 security | auto round 3, fix commits only; the standards reviewer raised the same point as a weak judgement call, recorded once |
| 2026-11-20T20:15:00+00:00 | walkthrough | 289e8c3 | claude-opus-5-5 (auto) | 0 new, 1 resolved, 0 still open | auto walkthrough, not a review pass |
| 2026-11-20T20:30:00+00:00 | check | @ 5be67ff | scripts/check | — | green, the loop's check before round 4 |
| 2026-11-20T20:30:00+00:00 | code-review | 01, 02, 03 @ 2b1f222..5be67ff | claude-sonnet-5-5 (dispatched; code by claude-opus-5-5, fixes by the orchestrator claude-opus-5-5) | 2 new: 1 standards, 1 spec, 0 security | auto round 4, the last; the loop stops at its limit of 3 address passes with C5, C6 open |

## Findings

| # | Status | Axis | Affects | Description | Comments |
|---|---|---|---|---|---|
| C1 | resolved | standards | 02, lambda/world/federation/build.rs | Possible Mysterious Name: the `(name, parse)` tuples and the `(|text: &str| files::parse_world(text).map(drop)) as fn(&str) -> Result<(), String>` cast exist only to coerce the first closure's type; two calls to a small `check(name, parse)` helper would read more plainly. | 2026-11-20: found. 2026-11-20 (auto walkthrough): fixed: `build.rs` calls `check(&federation, name, parse)` twice, with `parse` a plain `fn(&str) -> Result<T, String>`, so the tuples and the cast are gone; `cargo build`/`test --test directory`/`fmt --check`/`clippy -D warnings` on `sfab-federation` pass, and a broken `world.yaml` still fails the build. |
| C2 | resolved | spec | 01, docs/aws-facts.md | The CodeBuild S3 caching-permissions row was set to `tested` in 6b14d6c, but issue 01's Done when says to set it to `tested` only after the owner's second build shows no `Unable to download cache`; the build log proves only that `s3:ListBucket` was missing, not that the full list restores the cache. | 2026-11-20: found. 2026-11-20 (auto walkthrough): fixed: the row's status is now `docs, tested in part` (the top rules allow a mixed status, as `docs, forum`), and its source says `s3:ListBucket` was seen missing, not yet seen to be enough, and becomes `tested` once a build restores the cache, per issue 01. |
| C3 | resolved | standards | 01, docs/aws-facts.md | `docs/aws-facts.md`'s CodeBuild S3 caching-permissions row has status `docs, tested in part`, but `tested in part` isn't one of the file's four statuses (`tested`, `docs`, `forum`, `memory`; a mix of defined ones, as `docs, forum`, is the only precedent): use `docs` and keep the caveat in the Source cell. | 2026-11-20: found. 2026-11-20 (auto walkthrough): fixed: status `docs`; the Source cell still says `s3:ListBucket` was seen missing in build `fd2f07ad`'s log and becomes `tested` once a build restores the cache. |
| C4 | resolved | spec | 01, docs/aws-facts.md | The CodeBuild S3 caching-permissions row is now plain `docs`, but issue 01 says to cite the build log when the docs don't name `s3:ListBucket`, and the failure without it was seen on a real build (`fd2f07ad`), which the file's rules call `tested`: `docs, tested` (a mix, as `docs, forum`) fits the evidence, with the Source cell's caveat that the full list isn't yet seen to be enough. | 2026-11-20: found. 2026-11-20 (auto walkthrough): fixed: status `docs, tested`, and the Source cell says the `s3:ListBucket` failure is the `tested` part and the full list becomes `tested` once a build restores the cache. |
| C5 | resolved | standards | 01, docs/aws-facts.md | Possible confusing wording: the CodeBuild S3 caching-permissions row's Status says `docs, tested` while its Source still says "set `tested` once a build restores the cache", and one row carries two facts (the permission list from the docs, and the `s3:ListBucket` failure seen on a build); two rows, `docs` and `tested`, would read plainly. | 2026-11-20: found. 2026-11-20 (walkthrough): fixed: the row is split in two, the permission list (`docs`, `tested` once a build restores the cache, per issue 01) and the `s3:ListBucket` failure seen in build `fd2f07ad` (`tested`) (owner's option 1). |
| C6 | resolved | spec | 01, docs/aws-facts.md | Issue 01 says to set the row to `tested` only after the owner's second build shows no `Unable to download cache`, but it reads `docs, tested` before that build; C2 (too early), C3 (undefined status) and C4 (plain `docs` understates the log evidence) moved it back and forth, so the status needs the owner's call. | 2026-11-20: found. 2026-11-20 (walkthrough): fixed by C5's split: the permission-list row is `docs` until the owner's second build, as issue 01 says, and only the observed failure is `tested` (owner's option 1 on C5). |
