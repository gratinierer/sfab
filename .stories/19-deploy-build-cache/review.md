# 19-deploy-build-cache: review

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
| 2026-11-20T13:40:00+00:00 | review | 6c218cfeda | claude-sonnet-5-5 (dispatched; story authored by claude-opus-5-5) | 5 new, 5 open | full: first pass; same-family reviewer per AGENTS.local.md, weaker independence; covers issue 03, which another writer added before 6c218cf |

## Findings

| # | Status | Axis | Affects | Description | Comments |
|---|---|---|---|---|---|
| R1 | resolved | completeness | spec.md, 02 | Dropping the tests from the deploy drops gates only tests enforce, and the story lists none: `federation/worlds.yaml` is validated only by `cargo test` (`lambda/world/federation/src/directory.rs:34-36`, `.expect("... checked by its test")`), so a bad entry now deploys and panics at runtime, and `docs/federation.md:57` still says to run `scripts/check` before deploying; inventory such gates and either keep a cheap guard in `scripts/build` or name them in the spec's "Accepted risk", and fix `docs/federation.md`. | 2026-11-20: found; verified `directory.rs:34-36` and `docs/federation.md:57`. 2026-11-20 (walkthrough): fixed: issue 02 makes `scripts/build` fail on a committed data file that doesn't parse, lists the `include_str!` files, fixes `docs/federation.md:57`, and adds a Done when check; spec's "Accepted risk" says the data checks stay (owner's option 1). |
| R2 | resolved | factual | 02 | "Done when" asks for `scripts/build` argument tests and a `scripts/deploy` call test through "the stubbing seam story 18 settled (R31, R33)", but story 18 settled the opposite: R31's final fix and R34 removed the command overrides, making the scripts thin wrappers no test runs (decisions live in `cdk/deploy_plan.py`); no test under `cdk/tests/` runs either script. Specify text-level tests in the style of `test_deployer_stack.py:82-86`, or drop them. | 2026-11-20: found; verified R31/R34 in `.stories/done/18-deploy-from-codebuild/review.md:87,90`. 2026-11-20 (walkthrough): fixed: issue 02's Done when now has text-level tests of `scripts/deploy` and `scripts/check`, states that no test runs the scripts and no env var changes their steps (story 18 R31, R34), and checks `scripts/build`'s arguments by hand (owner's option 1). |
| R3 | resolved | completeness | 02 | The `fallback` mode's reason goes away and the ticket doesn't decide what replaces it: `cdk/deploy_plan.py:86-87` says `scripts deploy SfabDeployer` falls back to the fixture "for the pages the tests need", but with no tests and `cdk/app.py` synthesizing only `SfabDeployer` there, generate and `cargo lambda build` may be unneeded for it; decide whether that case still builds, and list the stale comments (`scripts/deploy:8-9,18`, `deploy_plan.py:13,38,86-87`). | 2026-11-20: found; verified `deploy_plan.py:86-87`. 2026-11-20 (walkthrough): fixed: issue 02's `fallback` mode runs no `scripts/build` (the app then synthesizes only `SfabDeployer`, `cdk/app.py:5-8`), lists the comments to rewrite, and the text-level test asserts `scripts/build` only in the `require-secret` branch (owner's option 1). |
| R4 | resolved | completeness | spec.md, 01 | Once `cargo-audit` leaves (02), the cache holds only the cargo registry, git db and uv downloads, worth seconds, while the long step, the Lambda compile that needs ARM/Medium (03), isn't cached (`lambda/target`, which CI does cache): the spec should say what the restored cache buys, and decide whether `lambda/target` joins it. | 2026-11-20: found; cache paths in `buildspec.yml:46-51`, CI's in `ci.yml:33-39`. 2026-11-20 (walkthrough): fixed: spec Goal 1 says the restored cache saves seconds of downloads, and "Out of scope" defers caching `lambda/target` (1.4 GB locally) until a Medium build is timed (owner's option 1). |
| R5 | open | consistency | spec.md | The H1, "its cache restores, and it only builds and deploys", leaves out Goal 3, ARM/Medium, which the Goal, Scope, "After this story" and issue 03 carry. | 2026-11-20: found. |
