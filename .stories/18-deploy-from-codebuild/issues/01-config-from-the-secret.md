# The config loader reads `sfab/config`

Label: ready-for-agent

`spec.md` "The config secret": `cdk/sfab_config.py` reads the Secrets Manager secret instead of `sfab.config.toml`.

## Scope

- `sfab_config.load()` reads `sfab/config` in us-east-1 through boto3 when `SFAB_CONFIG` isn't set, and the fixture
  file when it is. The checks are the ones the TOML has today, plus "the secret is missing", "it has no value" and
  "its value isn't a JSON object", each named in the one error.
- `Config` also carries the secret's `version_id`, the `VersionId` that `GetSecretValue` returned, for issue 03's
  references. `account_id` and `version_id` aren't keys in the secret: the required keys become the secret's
  entries only, not every `Config` field (today `REQUIRED_KEYS` at `cdk/sfab_config.py:49`), and the secret refuses
  both names as unknown keys. The fixture carries both as keys, a fixed `version_id` beside its `account_id`, so
  tests never call AWS.
- `account_id` comes from `sts get-caller-identity`, not the config, through its own function, `deploy_account()`,
  which takes, in this order, `CDK_DEFAULT_ACCOUNT` (the CDK CLI sets it from the credentials), then the fixture's
  `account_id` when `SFAB_CONFIG` names a loadable fixture, then STS, separate from `load()`, so `cdk/app.py` can target `SfabDeployer`'s account when the config doesn't load. With
  `SFAB_CONFIG`, the fixture still carries `account_id` and supplies it with no STS call when `CDK_DEFAULT_ACCOUNT`
  is unset, so tests never reach AWS;
  in the secret, `account_id` is refused as an unknown key. Record in `docs/aws-facts.md` that the CDK CLI sets
  `CDK_DEFAULT_ACCOUNT` for the app from the resolved credentials and leaves it unset without them, so
  `deploy_account()` then calls STS, and synth with neither credentials nor `SFAB_CONFIG` fails. `Config` keeps an `account_id` field so the
  stacks don't change; the fixture keeps its `000000000000`. `load()` fills `Config.account_id` from the fixture's
  `account_id` under `SFAB_CONFIG`, never from `CDK_DEFAULT_ACCOUNT`, so a fixture-backed synth targets
  `000000000000` and the fixture still can't deploy, even through a raw `cdk deploy`; with the secret it fills it
  from `deploy_account()`.
- A deploy run in a region other than us-east-1 refuses before synth, naming both regions. `scripts/deploy` checks
  it, not `load()`, so the smoke script, `waf_check.py` and the generators still run anywhere; `load()` always reads
  the secret in us-east-1. The region is the one boto3 resolves (`AWS_REGION`, then `AWS_DEFAULT_REGION`, then the
  profile), which CodeBuild sets to the project's region; none at all refuses too.
- `smoke_claim`: the smoke script reads it from the same source as the rest (secret or fixture), not from raw TOML
  (`cdk/smoke.py` :801-813). `Config` gains an optional `smoke_claim` (`None` when absent), which `load()` fills from
  either source, and `_smoke_claim` validates that value instead of reading a file.
- The path seam goes: `load()` takes no path, its source is `SFAB_CONFIG`'s fixture or else the secret, and
  `config_path()` and `DEFAULT_PATH` (`cdk/sfab_config.py:21,63-70`) are removed, since with `SFAB_CONFIG` unset they
  name the deleted `sfab.config.toml`. `cdk/smoke.py`'s `main(config_path=None)` and `load_target(args,
  config_path)` (:783-793, :833-841) drop the path and call `load()`, so the owner's smoke run reads the secret;
  `cdk/tests/test_smoke_args.py` sets `SFAB_CONFIG` to its TOML files with `monkeypatch.setenv` instead of passing
  their paths.
- `scripts/check`: with `SFAB_CONFIG` unset it uses the secret. When the config doesn't load, whether the secret
  can't be read, as in an agent's container with no AWS credentials, or is readable but incomplete, such as before
  `hosted_zone_id` is known, it falls back to the fixture with a message naming the problems, as it does today for a missing
  TOML. `scripts/deploy` runs it in a mode that requires the secret, so a deploy can't generate the public
  description from the fixture. The one exception, `scripts/deploy SfabDeployer` with no loadable config, is issue
  02's: it allows the fallback, because that stack publishes nothing generated. Remove `scripts/check`'s block
  (:10-13) that exports the fixture when `sfab.config.toml` is missing; the fallback is `load_for_check`'s alone.
- `scripts/deploy` drops its `sfab.config.toml` checks, keeps refusing `SFAB_CONFIG`, and takes `DEPLOY_STACKS`-style
  arguments: `--all` or one or more of the app's stack names, refusing anything else before any CDK command. It
  passes `--require-approval never`.
- The decisions live in Python and are tested in-process with stubbed Secrets Manager and STS; the two scripts are
  thin wrappers that tests never run, and no environment variable changes which steps they run, so a build's
  `--environment-variables-override` can't skip a check or replace `cdk`.
  - `cdk/deploy_plan.py` takes the arguments and the environment and, in this order, refuses a set `SFAB_CONFIG`, checks
    the arguments, then the region, then loads the config. It returns the stacks to deploy and whether `scripts/check` must use the secret
    or may fall back (only `SfabDeployer` with no loadable config, issue 02), or the errors.
  - `sfab_config.load_for_check(require_secret)` makes `scripts/check`'s choice: the secret, or the fixture with a
    message naming the problems. With `SFAB_CONFIG` set it uses that fixture and never reads the secret, as CI does;
    with `require_secret` it refuses a set `SFAB_CONFIG`, naming it. The generators call it, and `scripts/check` passes `--require-secret` through to
    them when given it.
  - `scripts/deploy` syncs `cdk/.venv`, runs the plan in it and stops on its errors, then runs `scripts/check`, with
    `--require-secret` unless the plan allows the fallback, then `cdk deploy` with the plan's stacks.
- Add `boto3`, pinned, to `cdk/requirements.in` and regenerate the hash-locked `cdk/requirements.txt`, since the
  loader, `deploy_account()` and `scripts/deploy`'s region check use it and `scripts/check` syncs with
  `--require-hashes`; `scripts/audit` then covers it.
- `cdk/tests/test_app.py`'s `test_synth_refuses_the_copied_example` synthesizes from the example file; it becomes a
  test with an incomplete fixture through `SFAB_CONFIG`, still holding `CHANGE_ME` values, which after issue 02
  expects only `SfabDeployer` and a note naming them, since `app.py` runs in a subprocess no stub reaches. `cdk/waf_check.py` (:10) and `cdk/smoke.py` (:7) say
  they need no AWS credentials, which stops being true.
- Remove `sfab.config.example.toml` and the `.gitignore` line for `sfab.config.toml`, and update the comments and
  docstrings that name the file (`cdk/sfab_config.py`, `scripts/check`, `scripts/deploy`, `cdk/smoke.py`,
  `cdk/tests/conftest.py`, `cdk/tests/test_stacks.py`, `cdk/tests/test_smoke_args.py`).

## Done when

- Tests, with a stubbed Secrets Manager and STS (no real AWS), cover: a full secret loads; each missing, blank,
  `CHANGE_ME`, unknown or malformed entry is named; a missing secret, an empty one and a non-JSON one; the account
  comes from the caller identity; `SFAB_CONFIG` still selects the fixture; `version_id` is
  the stub's `VersionId`.
- Tests cover `deploy_plan`: a set `SFAB_CONFIG` is refused before anything else; `--all`, one and several stack
  names pass; a flag or an unknown name is refused, and so
  is a region other than us-east-1, or none, each before the config is loaded.
- Tests cover `load_for_check` falling back to the fixture when the secret can't be read and when it is readable but
  incomplete, and refusing to with `require_secret`; using a set `SFAB_CONFIG` without touching Secrets Manager, and
  refusing it with `require_secret`. They never run the generate step, so they write nothing under
  `build/`.
- `scripts/check` passes in CI with the fixture.
