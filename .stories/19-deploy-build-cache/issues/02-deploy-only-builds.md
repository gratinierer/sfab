# The deploy runs scripts/build, not scripts/check

Label: ready-for-agent

`spec.md` "Goal" 2 and "Scope".

## Scope

- **New `scripts/build [--require-secret]`**: the first three steps of today's `scripts/check`, moved unchanged,
  plus the OpenAPI tests between generate and the Lambda build (below):
  the Python environment (`cdk/.venv`), generate (`generate_public_openapi.py`, `generate_website.py`, passing
  `--require-secret` through), and `cargo lambda build --release --arm64 --locked`. It refuses any other argument,
  as `scripts/check` does today. Its header comment says it makes what `cdk deploy` needs.
- **`scripts/check [--require-secret]`** runs `scripts/build` with the same argument, then "Audit dependencies"
  and "Test", in today's order. It runs the same checks as today, with `tests/test_openapi.py` also run early by `scripts/build` and again in the
  full suite. Its header comment says the deploy runs only
  `scripts/build`.
- **`scripts/deploy`** runs `scripts/build --require-secret` in the `require-secret` mode, where it ran
  `scripts/check --require-secret`. In the `fallback` mode (`SfabDeployer` alone, with no loadable config) it runs no
  `scripts/build`: it syncs the venv and runs `cdk deploy SfabDeployer`, and the app synthesizes only `SfabDeployer`,
  because the Lambda builds and pages are missing (`cdk/app.py:5-8`). The fixture's pages were there for the tests,
  which the deploy no longer runs. Its header comment drops "then runs every check, audits included" and says the
  audit and tests are left to `scripts/check`, CI and Dependabot. `cdk/deploy_plan.py`'s mode names stay; rewrite
  what names `scripts/check` as the deploy's step: its docstring (:12-14), `Plan`'s docstring (:38), the comment at
  :86-87, and `scripts/deploy`'s comments (:8-9, :18). Also what says `scripts/check` builds before a synth or
  deploy: `cdk/app.py:28` and its user-facing messages at :61 and :68 ("Run scripts/check, ..."), which name
  `scripts/build` instead, `cdk/stacks.py:43`, the comment at `cdk/tests/test_app.py:54`, and the test name
  `..._the_check_falls_back` at `cdk/tests/test_deploy_plan.py:64`.
- **The deploy still refuses bad data.** The Lambdas compile in committed data files with `include_str!`
  (`grep -rn include_str! lambda --include=*.rs`). Most keep a check the deploy still runs: `world/founders.yaml`,
  `world/quarters.yaml` and `world/shelves.yaml` are checked at synth by `cdk/world_config.py`, which `cdk/app.py`
  runs on every deploy (`homes/src/lib.rs:62`, `library/src/lib.rs:75`, `vouches/src/lib.rs:63`: "checked at synth
  and by its test"); the `vouches` tests read the fixture, not the real founders file (the `fixture-founders`
  feature); and the OpenAPI document (`lambda/api/src/lib.rs:1264`) and the constitution are served as text, not
  parsed. Only `federation/world.yaml` and `federation/worlds.yaml` (`lambda/world/federation/src/directory.rs:18,22`,
  parsed at :34-36) are checked by nothing but `cargo test` (`the_committed_directory_files_parse`). The deploy must
  fail on either one that doesn't parse, with the checks that test makes, so a bad edit fails the deploy instead of
  panicking on the first request. How is the implementer's choice, recorded in a comment here with the reason: for
  example a `build.rs` in the federation crate, `scripts/build` running only that test
  (`cargo test -p sfab-federation --test directory the_committed_directory_files_parse`; without `--test directory`
  it also builds the `daily` and `requests` tests), or a check at synth beside `world_config.py`'s, which would copy
  `parse_world`, `parse_worlds` and `valid_domain` into Python, where they can drift from the Rust.
- **The deploy still checks the API definition.** `openapi/sfab.yaml` is the API Gateway definition
  (`cdk/stacks.py`), and its route, length, validator and gateway-response checks live only in
  `cdk/tests/test_openapi.py`. `scripts/build` runs that file alone, after generate and before
  `cargo lambda build`, so a bad definition fails in seconds rather than after the compile, the way `scripts/check`
  runs pytest: `(cd cdk && SFAB_CONFIG=tests/fixtures/sfab.config.test.toml .venv/bin/python -m pytest -q
  tests/test_openapi.py)`. It takes about 3 s and needs no Rust. `scripts/check` then runs the whole suite as today,
  that file included.
  `test_openapi.py` reads route tables from three stories' specs (`SPECS`, :19-23: `2-identity-api`,
  `4-life-in-the-world`, `17-genesis-codes`), so closing one, which moves it to `.stories/done/`, would break the
  deploy. Make `SPECS` find each spec in `.stories/` or `.stories/done/`, and fail with the story's name when it's in
  neither.
- **`docs/federation.md:57`** says "Run `scripts/check`, which refuses an invalid or repeated domain, commit, and
  deploy." Make it say what refuses a bad domain after this change.
- **`buildspec.yml`** drops the `cargo install cargo-audit` block and the `/root/.cargo/bin/cargo-audit` cache path.
  Its header comment says it runs `scripts/deploy`, which builds and deploys without the audit or tests, and that
  it installs every pin except `CARGO_AUDIT_VERSION`, which only CI uses. `tool-versions.env`'s header comment says
  the same.
- **`cdk/tests/test_tool_versions.py`**: `test_the_buildspec_loads_the_pins_and_uses_every_one` (:61) expects every
  pin except `CARGO_AUDIT_VERSION`, and a new test asserts the buildspec doesn't mention `cargo-audit`.
  `test_tool_versions_pins_every_tool_ci_and_the_build_install` (:29-36) stays as it is: `tool-versions.env` keeps
  `CARGO_AUDIT_VERSION` for CI.
- **`.github/workflows/ci.yml`** keeps running `scripts/check` with `cargo-audit` installed. Only its header
  comment changes: the deploy script runs `scripts/build`, not every step.
- **`.stories/2-identity-api/spec.md` "Delivery"** (:879-886) says `scripts/check` runs before each commit and
  deploy, and its "Build and deploy" bullet (:460-463) says the deploy builds and that CI runs the same steps. Update
  both: the deploy runs `scripts/build`, and the audit and tests run in `scripts/check` before a commit and in CI.
  Point to this story. Check that `docs/adr/0009-deploys-run-in-codebuild.md` and
  `docs/restart-after-a-stop.md` say nothing about the deploy auditing or testing, and fix them if they do.

Write the tests first.

## Done when

- `scripts/check` passes, and still runs generate, build, audit and test, in that order.
- No test runs `scripts/build`, `scripts/check` or `scripts/deploy`, and no environment variable changes which steps
  they run, as story 18 settled (`.stories/done/18-deploy-from-codebuild/review.md` R31, R34): they stay thin
  wrappers, and the decisions stay in `cdk/deploy_plan.py`, tested in-process.
- Text-level tests, in the style of `test_the_buildspec_in_the_repo_runs_scripts_deploy_with_deploy_stacks`
  (`cdk/tests/test_deployer_stack.py:82-86`), read the scripts' source and assert that `scripts/deploy` calls
  `scripts/build` in the `require-secret` branch only, and never `scripts/check`, `scripts/audit`, `cargo test`, or `pytest` on anything but `tests/test_openapi.py`, that `scripts/check`
  calls `scripts/build` before the audit and the tests, and that `scripts/build` runs exactly one `pytest`
  command, on `tests/test_openapi.py`, before its `cargo lambda build` line, and no `cargo test` or `scripts/audit`. They read command lines only, skipping comments, since the
  scripts' header comments name `scripts/check` and the audit.
- `scripts/build`'s arguments are checked by hand once (none, `--require-secret`, an unknown one refused), with the
  results in a comment here.
- `buildspec.yml` installs no `cargo-audit`, and the tool-versions tests pass.
- `scripts/build` fails when a test in `cdk/tests/test_openapi.py` fails. Check it by hand once (break a rule only
  that file checks, such as dropping a `maxLength` from `openapi/sfab.yaml`, so generate still passes; run
  `scripts/build`, see the `pytest` step fail it, restore the file) and note the result in a comment here.
- The deploy's build or synth fails when either federation file doesn't parse. Check each by hand once (break it,
  run `scripts/build` and a synth, restore it) and list the results in a comment here.
- On the owner's next build with issue 03 on `production` (one build job on ARM/Small), or started with
  `--compute-type-override BUILD_GENERAL1_MEDIUM`, since today's ARM/Small build runs out of memory in
  `cargo lambda build`, the BUILD phase log shows the generate and build steps and then `cdk deploy`, with no
  audit and no test output other than `tests/test_openapi.py`'s. This needs the owner's build, so when the agent can't check it, it leaves a comment
  here saying so.

## Comments

- 2026-11-20 (claude-opus-5-5, implementing): **The federation check is a `build.rs` in `sfab-federation`.**
  `parse_world`, `parse_worlds` and `Approved` moved unchanged into `src/directory/files.rs`, which `build.rs`
  includes with `#[path]` beside `src/domain.rs` (`valid_domain`), so the build applies the crate's own rules, with
  no copy to drift. Every compile of the crate runs it, `scripts/build`'s `cargo lambda build` included, and it
  costs only a host build of `saphyr`. Not the `cargo test` option, because this ticket's own text-level test says
  `scripts/build` runs no `cargo test`, and it would add a debug compile of the crate to every deploy; not the synth
  option, because it copies the rules into Python. The test's remaining assertion, that the committed list fits one
  page (`next` is null), isn't a parse check: more worlds than a page wouldn't panic, so the build doesn't check it.
- 2026-11-20 (claude-opus-5-5, implementing): by-hand checks, in the agent's container, which loads the secret
  `sfab/config`:
  - `scripts/build` with no argument: generate, the 28 tests of `tests/test_openapi.py`, the Lambda build; exit 0.
    `scripts/build --require-secret`: the same, exit 0. `scripts/build --bogus`: `build: unknown argument --bogus;
    takes only --require-secret`, exit 2. Like `scripts/check`, it reads only its first argument.
  - `federation/world.yaml` with `name: ""`: `scripts/build` fails in `cargo lambda build`, exit 101, with
    "federation/world.yaml doesn't parse: world.yaml's name must be 1 to 128 characters". A synth with the fixture
    (`SFAB_CONFIG=tests/fixtures/sfab.config.test.toml .venv/bin/python app.py`) passes, exit 0: the synth doesn't
    read the federation files, and the deploy stops at `scripts/build` before it.
  - `federation/worlds.yaml` with `- domain: world.test`: `scripts/build` fails the same way, exit 101:
    `federation/worlds.yaml doesn't parse: worlds.yaml: "world.test" isn't a valid domain`. The synth passes, exit 0.
  - `openapi/sfab.yaml` without the `maxLength: 83` of the `Proof` header (:3703): generate passes, then the `pytest`
    step fails `test_every_string_has_a_max_length` (1 failed, 27 passed) and `scripts/build` exits 1 before the
    Lambda build.
  Each file was restored with `git checkout`, and `git status` showed them unchanged.
- 2026-11-20 (claude-opus-5-5, implementing): the last Done when item needs the owner's build on CodeBuild (issue 03
  on `production`, or `--compute-type-override BUILD_GENERAL1_MEDIUM`). The agent can't start one, so it's left open.
