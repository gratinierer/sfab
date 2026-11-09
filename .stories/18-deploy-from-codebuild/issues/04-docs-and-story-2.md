# Story 2's first deploy and the docs follow

Label: ready-for-agent
Blocked by: 02, 03

## Scope

- `.stories/2-identity-api/spec.md`:
  - "Local config" becomes the secret: its entries, the three named secrets, the fixture, `account_id` from the
    credentials.
  - "Delivery": deploys run in CodeBuild through `scripts/deploy`, started by hand, with an emergency deploy from a
    machine.
  - "First deploy": `aws login` and `cdk bootstrap`; `cdk deploy SfabDeployer`; approve the connection; the
    protected `production` branch (`18-deploy-from-codebuild#05`); fill `sfab/config` except `hosted_zone_id`; then
    the existing steps, each stack deployed by a build with `DEPLOY_STACKS` naming it. "Prerequisites" follow.
- `.stories/2-identity-api/issues/01-*.md` (already `Blocked by` `18-deploy-from-codebuild#04` and `#05`): its deploy
  steps match the rewritten "First deploy", including the deployed proof that a build from `production` deploys.
- `docs/restart-after-a-stop.md`: a redeploy after a stop is a build; update it if any step's command changes.
- `.stories/17-genesis-codes/spec.md` "The IP rule": `ip_hash_key` lives in `sfab/config`, with a note pointing here.
- `.stories/12-public-repo/spec.md:21`, which lists `sfab.config.toml` as never public: the file no longer exists.
- Any other mention of `sfab.config.toml` outside `.stories/done/` (`grep`).

## Done when

- `grep -rn "sfab.config.toml"` outside `.stories/done/` and this story finds only the history notes this issue
  leaves on purpose.
- Stories 2, 12 and 17, whose documents this issue edits, each get a `/review-story` pass after the edit, so their
  reviews aren't left stale (`docs/agents/issue-tracker.md` "Before implementing a story").
