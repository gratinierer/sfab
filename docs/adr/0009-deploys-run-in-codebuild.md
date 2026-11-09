# Deploys run in CodeBuild, with config in Secrets Manager

Status: accepted

Deploys run in an AWS CodeBuild project, `sfab-deploy`, which builds the protected `production` branch through a
CodeConnection and deploys to its own account and us-east-1. The owner's private config lives in one Secrets Manager
secret, `sfab/config`, not in a gitignored `sfab.config.toml` on the owner's machine. A fourth stack, `SfabDeployer`,
holds the secret, the connection and the build, and is the only stack a machine deploys, once. See
`.stories/18-deploy-from-codebuild/`.

## Why

- **No machine in a deploy.** With the config in a local file, every deploy needed the owner's machine. Now a deploy
  is "start the build", from the console or the CLI, anywhere.
- **One place for the values.** The secret is the only real config. The build reads it with its role, the owner's
  tools read it with the owner's credentials, and nothing keeps a copy.
- **No secret in a template.** The stacks reference `origin_key` and `ip_hash_key` through CloudFormation dynamic
  references, where today both sit in the templates as text.

## Consequences

- Story 2's choice of a manual `cdk deploy` from the owner's machine, and story 17's choice of `ip_hash_key` in the
  TOML rather than Secrets Manager, no longer hold. Neither was an ADR.
- `account_id` leaves the config: a deploy targets the account of the credentials it runs with, so story 2's guard
  that wrong credentials fail at synth is gone.
- The build deploys `SfabDeployer` too, so it can change its own role; only the protected `production` branch reaches
  it.
- A deploy costs a few cents of CodeBuild time, and the secret about $0.40 a month.
