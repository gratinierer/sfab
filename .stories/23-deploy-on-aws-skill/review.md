# 23-deploy-on-aws-skill: review

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
| 2026-11-22T16:11Z | review | 950a64a6a962 | claude-sonnet-5-5 (dispatched; story authored by claude-opus-5-5, orchestrated by claude-opus-5-5) | 6 new | full: no prior review; auto round 1; same vendor family as the author, so independence is weaker |

## Findings

| # | Status | Axis | Affects | Description | Comments |
|---|---|---|---|---|---|
| R1 | open | consistency | spec.md, 01 | `spec.md` Decisions "Commit policy" justifies pushing by ADR 0010, but ADR 0010 says "Pushing still always needs an explicit request" and `commit-policy.md`'s exception says skills "never push"; issue 01 adds one policy line and no ADR amending ADR 0010, leaving the policy self-contradictory. | 2026-11-22: needs the user (ea475b3d60): the fix changes ADR 0010 (add an ADR amending it, or reword the policy's 'never push' exception) and the story's scope, so it's an important finding for the owner to rule on. |
| R2 | resolved | completeness | spec.md, 01 | Step 6 runs `scripts/smoke`, which execs `cdk/.venv/bin/python` (absent with `skipcheck` on a fresh clone) and needs `smoke_claim` in `sfab/config`; its exit 2 (config) and 3 (WAF block, wait 5 min and rerun) are not handled by step 6 or the `auto` stop list. | 2026-11-22: auto: spec.md step 6 now makes a missing cdk/.venv as scripts/deploy does and maps smoke exits 0-3, with 3 reported as wait-and-rerun, not a failed deploy. |
| R3 | resolved | completeness | spec.md, 01 | Step 5 passes several stacks via `--environment-variables-override name=DEPLOY_STACKS,value=...`; the space-separated value (`SfabApi SfabWebsite`) needs quoting in CLI shorthand, and neither spec nor issue names the valid names (`cdk/deploy_plan.py:31`: SfabBase, SfabApi, SfabWebsite, SfabDeployer, `--all`) for validating before a build starts. | 2026-11-22: auto: spec.md step 5 now quotes the DEPLOY_STACKS shorthand value and checks names against STACKS/ALL in cdk/deploy_plan.py before starting a build. |
| R4 | resolved | completeness | spec.md | Steps 1-3 compare `main` with `origin/main` and `production` with main's head but no step runs `git fetch`, so the comparisons can use stale refs; the "asks before reaching outside the machine" rule doesn't say whether a fetch is asked about. | 2026-11-22: auto: spec.md step 1 now starts with git fetch origin main production, unasked because it only reads, as step 4's sts call already is. |
| R5 | open | consistency | spec.md, 01 | The `smoke` argument is defined only for `auto`; step 6 always "offers" the smoke test and issue 01 lists `smoke` without saying what it does interactively (redundant, or skip the offer). | 2026-11-22: needs the user (c22a12175c): what interactive `smoke` does (skip the offer and run it, or have no effect) is a product decision the grilling didn't record. |
| R6 | open | completeness | spec.md, 01 | The skill takes `AWS_PROFILE` from `.env`, but shell state doesn't persist between Bash calls, so the spec should say every `aws`/`cdk`/`scripts/smoke` call carries `AWS_PROFILE=...` explicitly. | |
