# The config loader reads `sfab/config`

Label: ready-for-agent

`spec.md` "The config secret": `cdk/sfab_config.py` reads the Secrets Manager secret instead of `sfab.config.toml`.

## Scope

- `sfab_config.load()` reads `sfab/config` in us-east-1 through boto3 when `SFAB_CONFIG` isn't set, and the fixture
  file when it is. The checks are the ones the TOML has today, plus "the secret is missing", "it has no value" and
  "its value isn't a JSON object", each named in the one error.
- `account_id` comes from `sts get-caller-identity`, not the config, through its own function, `deploy_account()`,
  separate from `load()`, so `cdk/app.py` can target `SfabDeployer`'s account when the config doesn't load. With
  `SFAB_CONFIG`, the fixture still carries `account_id` and supplies it with no STS call, so tests never reach AWS;
  in the secret, `account_id` is refused as an unknown key. `Config` keeps an `account_id` field so the
  stacks don't change; the fixture keeps its `000000000000`.
- A deploy run in a region other than us-east-1 refuses before synth, naming both regions.
- `smoke_claim`: the smoke script reads it from the same source as the rest (secret or fixture), not from raw TOML
  (`cdk/smoke.py` :801-813).
- `scripts/check`: with `SFAB_CONFIG` unset it uses the secret. When the secret can't be read, as in an agent's
  container with no AWS credentials, it falls back to the fixture with a message, as it does today for a missing
  TOML. `scripts/deploy` runs it in a mode that requires the secret, so a deploy can't generate the public
  description from the fixture. The one exception, `scripts/deploy SfabDeployer` with no loadable config, is issue
  02's: it allows the fallback, because that stack publishes nothing generated.
- `scripts/deploy` drops its `sfab.config.toml` checks, keeps refusing `SFAB_CONFIG`, and takes `DEPLOY_STACKS`-style
  arguments: `--all` or one or more of the app's stack names, refusing anything else before any CDK command. It
  passes `--require-approval never`.
- `cdk/tests/test_app.py`'s `test_synth_refuses_the_copied_example` synthesizes from the example file; it becomes the
  same test on a stubbed secret holding `CHANGE_ME` values. `cdk/waf_check.py` (:10) and `cdk/smoke.py` (:7) say
  they need no AWS credentials, which stops being true.
- Remove `sfab.config.example.toml` and the `.gitignore` line for `sfab.config.toml`, and update the comments and
  docstrings that name the file (`cdk/sfab_config.py`, `scripts/check`, `scripts/deploy`, `cdk/smoke.py`,
  `cdk/tests/conftest.py`, `cdk/tests/test_stacks.py`, `cdk/tests/test_smoke_args.py`).

## Done when

- Tests, with a stubbed Secrets Manager and STS (no real AWS), cover: a full secret loads; each missing, blank,
  `CHANGE_ME`, unknown or malformed entry is named; a missing secret, an empty one and a non-JSON one; the account
  comes from the caller identity; a wrong region refuses; `SFAB_CONFIG` still selects the fixture.
- Tests cover `scripts/deploy`'s argument check: `--all`, one and several stack names pass; a flag or an unknown name
  is refused before `cdk` runs.
- Tests cover `scripts/check` falling back to the fixture without credentials, and refusing to in the mode
  `scripts/deploy` uses.
- `scripts/check` passes in CI with the fixture.
