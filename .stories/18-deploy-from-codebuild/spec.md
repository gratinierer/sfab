# Deploy from CodeBuild, with config in Secrets Manager

GitHub issue: [gratinierer/sfab#18](https://github.com/gratinierer/sfab/issues/18)
Authored: 2026-11-09 by claude-opus-5-5

## Goal

Today a deploy needs the owner's machine: `scripts/deploy` reads the gitignored `sfab.config.toml` there and runs
`cdk deploy` with the owner's `aws login` credentials. This story moves the deploy into AWS. A CodeBuild project
builds the protected `production` branch and deploys to its own account and region, and the private config lives in
one Secrets Manager secret instead of a file. After the one-time setup, a deploy is "start the build", from anywhere,
with no machine involved.

Nothing is deployed yet: story 2's first deploy (`2-identity-api#01`) waits for this story, so the very first world
is already deployed this way and there's no `sfab.config.toml` to migrate. `docs/adr/0009-deploys-run-in-codebuild.md`
records the decision. It replaces two earlier choices that weren't ADRs: story 2's "manual `cdk deploy`" from the
owner's machine (`.stories/2-identity-api/spec.md` "Delivery", Clarifications) and story 17's `ip_hash_key` in the
TOML, "not Secrets Manager" (`.stories/17-genesis-codes/spec.md` Clarification 16).

## The config secret

- One Secrets Manager secret, `sfab/config`, in us-east-1, holding a JSON object with one key/value pair per entry, so
  the console shows and edits each entry on its own: `alert_email`, `abuse_contact`, `domain_name`, `hosted_zone_id`,
  `origin_key`, `ip_hash_key`, and the optional `smoke_claim`. The rules `cdk/sfab_config.py` applies to the TOML
  today carry over unchanged: every value a string, none blank or `CHANGE_ME`, unknown keys refused, the
  `origin_key` and `ip_hash_key` patterns, and one error naming every problem. `smoke_claim` stays outside the blank
  and `CHANGE_ME` checks, as today: only the smoke script's `--production` checks it.
- **`account_id` leaves the config.** The deploy targets the account of the credentials it runs with, read with
  `sts get-caller-identity`: the build's own account in CodeBuild, the owner's on a machine. This drops story 2's
  guard that credentials for another account fail at synth, which is accepted: the build can only reach its own
  account, and the owner has one. The test fixture keeps its `account_id`, so tests never call STS.
- **The region stays fixed at us-east-1 in the CDK code**, because the CloudFront certificate and the
  CLOUDFRONT-scope WAF need it. A deploy run in any other region refuses before synth: `scripts/deploy` checks the region boto3 resolves, and the
  owner's tools that only read the secret run from any region.
- **The stack never writes the values.** CDK creates the secret with no value it controls, and the owner fills it
  with `aws secretsmanager put-secret-value` or the console. A template that carried the values, or placeholders,
  would overwrite the owner's values on a later update. The secret is retained if its stack is deleted.
- **No secret sits in a template.** The stacks reference `origin_key` and `ip_hash_key` through CloudFormation
  dynamic references to `sfab/config`, so the synthesized templates hold a pointer, not the value. The values still
  show to anyone who can read the Lambda's configuration or the CloudFront origin, which is accepted. Where AWS
  doesn't accept a dynamic reference, `docs/aws-facts.md` records it and the issue says what happens instead.
- **Synth still validates.** The loader reads the secret at synth for the plain values, such as the domain, and
  checks every entry, so a bad value still fails before CloudFormation.
- **Tests never read the secret.** `SFAB_CONFIG` keeps pointing the loader at the committed fixture, as CI does, and
  `scripts/deploy` still refuses to run with it set, so the fixture can't deploy. The TOML file and
  `sfab.config.example.toml` go away; the fixture keeps the TOML format.
- **A leak:** replace the value in the secret, then start a build. Each resource holding a dynamic reference also
  carries a short hash of its own value, which synth reads with the values, so a new `origin_key` replaces the API key
  and updates the CloudFront header, and a new `ip_hash_key` updates `IP_HASH_KEY`. An unchanged reference isn't
  resolved again, and an edit to any other entry never replaces the API key.

## The deployer stack

A fourth stack, `SfabDeployer`, in the same CDK app as `SfabBase`, `SfabApi` and `SfabWebsite`, holds:

- the secret `sfab/config`;
- an AWS CodeConnection to GitHub for this repository. CloudFormation creates it as `PENDING`, and the owner approves
  it once in the console;
- the CodeBuild project `sfab-deploy`: source this repository through the connection, branch `production`, an Arm
  small Linux image, and an environment variable `DEPLOY_STACKS` defaulting to `--all`, overridable when a build
  starts;
- the build's IAM role, which may read `sfab/config` and assume the CDK bootstrap roles (`deploy`, `file-publishing`,
  `image-publishing`, `lookup`) and nothing else. The bootstrap's CloudFormation execution role keeps today's power;
- an S3 cache bucket for cargo and uv downloads, with a lifecycle rule so the cache can't grow without bound.

**The failure alert lives in `SfabBase`**, not `SfabDeployer`: an EventBridge rule on the `sfab-deploy` project's
`FAILED` and `STOPPED` build states publishes to the existing alerts topic. `SfabDeployer` must deploy before the
config exists, so it can't depend on `SfabBase`'s topic or on `alert_email`; the rule names the project by its fixed
name instead. Until `SfabBase` exists, which needs the full config, a failed build sends no email, so the owner
watches those first builds in the console.

**Synth without the config.** CDK synthesizes the whole app even to deploy one stack, and the other three stacks need
the config. So while `sfab/config` is missing, has no value, or fails the loader's checks, the app synthesizes only
`SfabDeployer` and prints a note naming what's missing. The same holds when the Lambda builds or the website pages
are missing, which `cdk/app.py` refuses today. `cdk/app.py` can't tell which stacks the CLI was asked for, so `scripts/deploy`
guards the rest: it runs `deploy_plan` first (a set `SFAB_CONFIG` refused, then the arguments, the region and the config), and with no loadable config it accepts only `SfabDeployer`, refusing
`--all` or any other stack with every problem named, so a forgotten value can't silently leave a stack out. A raw
`cdk deploy --all` stays unguarded, which is why every deploy goes through `scripts/deploy`. `scripts/deploy
SfabDeployer` with no loadable config runs `scripts/check` with its fixture fallback, so
the generate step writes the fixture's pages, which the tests need and `SfabDeployer` doesn't publish. The next full
deploy regenerates them from the secret.

**The build deploys its own stack.** `--all` includes `SfabDeployer`, so a change to the build itself ships without a
machine too, and the build can rewrite its own role. Only the protected `production` branch reaches the build, and
whoever can push there can already change everything the build deploys, so this adds no new risk.

## A build

- **Started by hand** with `aws codebuild start-build --project-name sfab-deploy` or the console, only by the owner's
  identity. No push starts it, and no other identity gets `codebuild:StartBuild`.
- **Builds `production`**, a protected branch the owner moves forward from `main` when a deploy is wanted.
- **Runs `scripts/deploy $DEPLOY_STACKS`**: `scripts/check` (generate, build, audit, test), then `cdk deploy` with
  `--require-approval never`. `scripts/deploy` accepts only `--all` or one or more of the app's stack names, and
  refuses anything else before running a CDK command, so an override can't carry other CDK flags. Overriding
  `DEPLOY_STACKS`, such as `--environment-variables-override name=DEPLOY_STACKS,value=SfabApi`, deploys single
  stacks, as story 2's "First deploy" does.
- **The same script runs on a machine** for an emergency deploy, with the owner's credentials reading the same
  secret.
- **Pinned tools:** the build installs the versions `.github/workflows/ci.yml` already pins (Rust from
  `lambda/rust-toolchain.toml`, cargo-lambda, zig, uv, cargo-audit) and the CDK CLI, which gets a pinned version in
  the repo, since nothing pins it today. CI and the build read the versions from one place.
- **No smoke run.** The smoke script uses up the `--no-ip-limit` codes in the owner's local
  `smoke-genesis-codes.txt`, so the owner runs it after a deploy, as today. It, `cdk/waf_check.py` and the generators
  read their config through the same loader, so on the owner's machine they read the secret with `aws login`
  credentials.

## One-time manual setup

Done once, by the owner, in story 2's "First deploy" and `2-identity-api#03`, which this story rewrites:

1. `aws login` and `cdk bootstrap aws://<account>/us-east-1`. Check CodeBuild's concurrent-build quota for the Arm
   small image in Service Quotas, and ask for an increase if it's 0, as story 2's "Prerequisites" does for Lambda.
2. `scripts/deploy SfabDeployer` from a machine, which works before the secret exists ("Synth without the config").
3. Approve the GitHub connection in the console.
4. Create and protect the `production` branch (issue 05).
5. Fill `sfab/config`, except `hosted_zone_id` until the domain is bought, then the rest of story 2's steps,
   starting builds with `DEPLOY_STACKS` naming one stack at a time where story 2 deploys them one by one. A code
   change between steps, such as story 2's registrar service name in the budget filter, is committed and `production`
   moved forward before the next build.

Buying the domain, confirming the SNS subscription and checking the Free flat-rate plan stay manual, as story 2 has
them.

## Cost

With prices to confirm and record in `docs/aws-facts.md`: one secret at about $0.40 a month plus API calls; an Arm
small CodeBuild build of roughly 15-20 minutes at a few cents per deploy; the CodeConnection free; the cache bucket
a few cents a month. Deploys are rare and started by hand, so this stays well under the budget.

## Clarifications

Grilled with the owner on 2026-11-09. The owner accepted every recommendation unless noted.

1. **Scope:** only the deploy moves. `genesis-codes` and `takedown` don't read the config; the smoke script and
   `waf_check` read the secret through the loader. The owner asked whether the other tools run on every deploy; they
   don't, they're run by hand.
2. **Trigger:** by hand only.
3. **Source:** a `production` branch, chosen by the owner over `main` with an override.
4. **CodeBuild alone**, no CodePipeline.
5. **Secrets Manager**, one secret with one key/value pair per entry. The owner asked why not separate keys and
   values; one secret already holds them separately.
6. **One app, a fourth stack.** The owner first answered "integrate everything in one stack", then, on learning the
   app already has three stacks, chose to pass the stacks to deploy as arguments, which gives `SfabDeployer` and a
   first deploy that names it.
7. **Least-privilege build role** through the CDK bootstrap roles.
8. **A new story, issue and ADR.**
9. **Every config entry in the secret**, `account_id` from the credentials.
10. **Dynamic references** for the secrets in the templates.
11. **No smoke run in the build.** The owner asked how the smoke script reads the secret: through the shared loader,
    with the owner's credentials.
12. **Arm image, pinned tools, S3 cache.**
13. **`scripts/deploy` stays** as the one command for the build and a machine. The owner asked whether it's still
    needed.
14. **Build failures email the owner** through the alerts topic.
15. **The build deploys its own stack** (`--all` includes `SfabDeployer`).
16. **One-time setup stays manual.**
17. **`2-identity-api#01` waits for this story**, and story 2's "First deploy" is rewritten.
18. **Only the owner starts builds.**
19. **`DEPLOY_STACKS`**, defaulting to `--all`, added by the owner so a build can deploy chosen stacks.
