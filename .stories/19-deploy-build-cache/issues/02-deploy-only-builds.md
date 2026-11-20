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
- **`scripts/deploy`** runs `scripts/build --require-secret` in the `require-secret` mode and `scripts/build` in
  the `fallback` mode, where it ran `scripts/check`. Its header comment drops "then runs every check, audits
  included" and says the audit and tests are left to `scripts/check`, CI and Dependabot. `cdk/deploy_plan.py`'s
  mode names stay; rewrite its comments and docstrings that name `scripts/check` as the deploy's step.
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
- Tests cover `scripts/build`'s arguments (none, `--require-secret`, an unknown one refused) and assert that
  `scripts/deploy` calls `scripts/build` with the plan's mode, and never `scripts/check`, `scripts/audit`,
  `cargo test` or pytest. Use the stubbing seam story 18 settled for `scripts/check` tests
  (`.stories/done/18-deploy-from-codebuild/review.md` R31, R33), or say in a comment here why none fits.
- `buildspec.yml` installs no `cargo-audit`, and the tool-versions tests pass.
- On the owner's next build, the BUILD phase log shows the generate and build steps and then `cdk deploy`, with no
  audit and no test output. This needs the owner's build, so when the agent can't check it, it leaves a comment
  here saying so.
