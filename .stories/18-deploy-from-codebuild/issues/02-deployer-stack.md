# The `SfabDeployer` stack and the build

Label: ready-for-agent
Blocked by: 01

`spec.md` "The deployer stack" and "A build".

## Scope

- A fourth stack, `SfabDeployer`, in `cdk/app.py`, in the account from the credentials and us-east-1, holding:
  - the secret `sfab/config`, created with no value the template controls and retained on stack deletion;
  - a CodeConnection to GitHub for `gratinierer/sfab`;
  - the CodeBuild project `sfab-deploy`: this repo through the connection, branch `production`, an Arm small Linux
    image, `DEPLOY_STACKS=--all`, and a buildspec in the repo that installs the pinned tools and runs
    `scripts/deploy $DEPLOY_STACKS`;
  - its role: `secretsmanager:GetSecretValue` on `sfab/config`, `sts:AssumeRole` on the four CDK bootstrap roles, the
    connection use CodeBuild needs, its own logs and the cache bucket, and nothing else;
  - an S3 cache bucket for cargo and uv, with a lifecycle rule.
- In `SfabBase`: an EventBridge rule on `sfab-deploy`'s `FAILED` and `STOPPED` states, publishing to the alerts topic,
  naming the project by its fixed name.
- `SfabDeployer`'s account comes from `deploy_account()` (issue 01), not from the config. The synth tests, which run
  `app.py` in a subprocess (`cdk/tests/test_app.py:10-14`), set `CDK_DEFAULT_ACCOUNT` and pass their configs as fixtures
  through `SFAB_CONFIG`, a missing file for "no config" and an incomplete one for "incomplete", so no subprocess
  reaches Secrets Manager or STS.
- `cdk/app.py` synthesizes only `SfabDeployer`, with a note naming what's missing, while the config fails to load or
  the Lambda builds or website pages are missing. It can't see which stacks the
  CLI selected, so `scripts/deploy` holds the guard below; a raw `cdk deploy --all` stays unguarded.
- `scripts/deploy` runs the loader first. `scripts/deploy SfabDeployer` works with no loadable config: it accepts only `SfabDeployer` then, and runs
  `scripts/check` with its fixture fallback, so the generate step writes the fixture's pages the tests need. It's the one case
  where `scripts/deploy` lets `scripts/check` fall back (issue 01). `--all` or any other stack
  name fails with the config's errors.
- The CDK CLI gets a pinned version in the repo, and `.github/workflows/ci.yml` and the buildspec read every tool
  version from one file, `tool-versions.env` at the repo root: `ci.yml` loads it instead of its inline `env:` pins
  (`UV_VERSION`, `CARGO_LAMBDA_VERSION`, `ZIGLANG_VERSION`, `CARGO_AUDIT_VERSION`), with `CDK_CLI_VERSION` added.
  `ci.yml` loads the file into `$GITHUB_ENV` in a step right after checkout, before the cache step whose key reads
  `env.CARGO_AUDIT_VERSION` (`ci.yml:39`).
- Confirm in AWS's documentation and record in `docs/aws-facts.md`: CodeBuild's GitHub source through a
  CodeConnection (resource type and auth type), the Arm small image's name and price, which CDK bootstrap roles a
  `cdk deploy` assumes, Secrets Manager's price, and the concurrent-build quota a new account starts with (forum
  reports say 0; record it as unconfirmed if AWS doesn't document it).

## Done when

- CDK tests assert `SfabDeployer`'s resources: the secret has no template-controlled value and is retained, the
  project's source branch, image, `DEPLOY_STACKS` default and buildspec path, and the role's exact grants, so
  nothing wider sneaks in.
- A CDK test asserts the EventBridge rule in `SfabBase` and its target.
- A test asserts `ci.yml` and the buildspec take every tool version from `tool-versions.env` and pin none inline.
- Tests cover synth with no config, an incomplete one and missing builds (only `SfabDeployer`, with the note, in the
  caller's account), and a full config (all four stacks).
- A test covers `scripts/deploy SfabDeployer` with no config (the check generates from the fixture, and only
  `SfabDeployer` deploys) and `scripts/deploy --all` with no config (refused before any CDK command).
- The deployed proof (a build from `production` deploys) belongs to `2-identity-api#01`, which waits for this issue.
