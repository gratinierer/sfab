# The deploy runs scripts/build, not scripts/check

Label: ready-for-agent

`spec.md` "Goal" 2 and "Scope".

## Scope

- **New `scripts/build [--require-secret]`**: the first three steps of today's `scripts/check`, moved unchanged:
  the Python environment (`cdk/.venv`), generate (`generate_public_openapi.py`, `generate_website.py`, passing
  `--require-secret` through), and `cargo lambda build --release --arm64 --locked`. It refuses any other argument,
  as `scripts/check` does today. Its header comment says it makes what `cdk deploy` needs.
- **`scripts/check [--require-secret]`** runs `scripts/build` with the same argument, then "Audit dependencies"
  and "Test", in today's order. Its behaviour doesn't change. Its header comment says the deploy runs only
  `scripts/build`.
- **`scripts/deploy`** runs `scripts/build --require-secret` in the `require-secret` mode, where it ran
  `scripts/check --require-secret`. In the `fallback` mode (`SfabDeployer` alone, with no loadable config) it runs no
  `scripts/build`: it syncs the venv and runs `cdk deploy SfabDeployer`, and the app synthesizes only `SfabDeployer`,
  because the Lambda builds and pages are missing (`cdk/app.py:5-8`). The fixture's pages were there for the tests,
  which the deploy no longer runs. Its header comment drops "then runs every check, audits included" and says the
  audit and tests are left to `scripts/check`, CI and Dependabot. `cdk/deploy_plan.py`'s mode names stay; rewrite
  what names `scripts/check` as the deploy's step: its docstring (:12-14), `Plan`'s docstring (:38), the comment at
  :86-87, and `scripts/deploy`'s comments (:8-9, :18).
- **The deploy still refuses bad data.** The Lambdas compile in committed data files with `include_str!` and parse
  them at runtime, and today only `cargo test` checks that they parse: `federation/world.yaml` and
  `federation/worlds.yaml` (`lambda/world/federation/src/directory.rs:18,22`), `world/quarters.yaml` (`homes`),
  `world/shelves.yaml` (`library`), the founders file `SFAB_FOUNDERS_FILE` names (`vouches`), the OpenAPI document
  (`lambda/api/src/lib.rs:31`) and the constitution text (`constitution`). Recheck that list with
  `grep -rn include_str! lambda --include=*.rs`. `scripts/build` must fail on a file that doesn't parse, with the same
  checks the tests make, so a bad edit fails the deploy instead of panicking on the first request. How is the
  implementer's choice, recorded in a comment here with the reason: for example a `build.rs` per crate, or
  `scripts/build` running only those parse tests (`cargo test -p <crate> <filter>`), not the whole suite.
- **`docs/federation.md:57`** says "Run `scripts/check`, which refuses an invalid or repeated domain, commit, and
  deploy." Make it say what refuses a bad domain after this change.
- **`buildspec.yml`** drops the `cargo install cargo-audit` block and the `/root/.cargo/bin/cargo-audit` cache path.
  Its header comment says it runs `scripts/deploy`, which builds and deploys without the audit or tests, and that
  it installs every pin except `CARGO_AUDIT_VERSION`, which only CI uses. `tool-versions.env`'s header comment says
  the same.
- **`cdk/tests/test_tool_versions.py`**: `test_the_buildspec_loads_the_pins_and_uses_every_one` (:62) expects every
  pin except `CARGO_AUDIT_VERSION`, and a new test asserts the buildspec doesn't mention `cargo-audit`.
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
  `scripts/build` in the `require-secret` branch only, and never `scripts/check`, `scripts/audit`, `cargo test` or `pytest`, and that `scripts/check`
  calls `scripts/build` before the audit and the tests.
- `scripts/build`'s arguments are checked by hand once (none, `--require-secret`, an unknown one refused), with the
  results in a comment here.
- `buildspec.yml` installs no `cargo-audit`, and the tool-versions tests pass.
- `scripts/build` fails when any one of the data files above doesn't parse. Check each by hand once (break it,
  run `scripts/build`, restore it) and list the results in a comment here.
- On the owner's next build, the BUILD phase log shows the generate and build steps and then `cdk deploy`, with no
  audit and no test output. This needs the owner's build, so when the agent can't check it, it leaves a comment
  here saying so.
