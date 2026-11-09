# The config loader reads `sfab/config`

Label: ready-for-agent

`spec.md` "The config secret": `cdk/sfab_config.py` reads the Secrets Manager secret instead of `sfab.config.toml`.

## Scope

- `sfab_config.load()` reads `sfab/config` in us-east-1 through boto3 when `SFAB_CONFIG` isn't set, and the fixture
  file when it is. The checks are the ones the TOML has today, plus "the secret is missing", "it has no value" and
  "its value isn't a JSON object", each named in the one error.
- `account_id` comes from `sts get-caller-identity`, not the config. `Config` keeps an `account_id` field so the
  stacks don't change; the fixture keeps its `000000000000`.
- A deploy run in a region other than us-east-1 refuses before synth, naming both regions.
- `smoke_claim`: the smoke script reads it from the same source as the rest (secret or fixture), not from raw TOML
  (`cdk/smoke.py` :801-813).
- `scripts/check`: with `SFAB_CONFIG` unset it uses the secret; with no AWS credentials it fails with a message saying
  to set `SFAB_CONFIG` to the fixture, instead of silently falling back, so a check can't generate the public
  description from the fixture before a deploy.
- `scripts/deploy` drops its `sfab.config.toml` checks, keeps refusing `SFAB_CONFIG`, and takes `DEPLOY_STACKS`-style
  arguments: `--all` or one or more of the app's stack names, refusing anything else before any CDK command. It
  passes `--require-approval never`.
- Remove `sfab.config.example.toml` and the `.gitignore` line for `sfab.config.toml`, and update the comments and
  docstrings that name the file (`cdk/sfab_config.py`, `scripts/check`, `scripts/deploy`, `cdk/smoke.py`).

## Done when

- Tests, with a stubbed Secrets Manager and STS (no real AWS), cover: a full secret loads; each missing, blank,
  `CHANGE_ME`, unknown or malformed entry is named; a missing secret, an empty one and a non-JSON one; the account
  comes from the caller identity; a wrong region refuses; `SFAB_CONFIG` still selects the fixture.
- Tests cover `scripts/deploy`'s argument check: `--all`, one and several stack names pass; a flag or an unknown name
  is refused before `cdk` runs.
- `scripts/check` passes in CI with the fixture.
