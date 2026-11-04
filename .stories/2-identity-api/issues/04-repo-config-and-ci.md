# Set up the repo, config and CI

Label: ready-for-agent

Create the monorepo and the config every later ticket uses: `spec.md` "Architecture" (layout) and
"Delivery" (local config, CI).

## Scope

- `/cdk/` with a Python virtual environment, `/openapi/`, `/lambda/` as a Cargo workspace, and
  `/world/`, `/federation/` and `/website/`, with placeholders that later tickets fill in.
- The config parser, reading `sfab.config.toml` or the file named by `SFAB_CONFIG`.
- An optional `smoke_claim` that the parser ignores and only the smoke script reads, commented out in the example,
  as `spec.md` "Local config" says.
- `sfab.config.example.toml` with `CHANGE_ME` values, the gitignored real file, and the fixture
  `cdk/tests/fixtures/sfab.config.test.toml`.
- GitHub Actions, using the fixture, running the same steps as the devcontainer.
- The dependency audits from `spec.md` "Delivery": a committed `Cargo.lock`, a fully pinned
  `cdk/requirements.txt`, `cargo audit` and `uvx pip-audit` as a step after build and before tests,
  the committed ignore list, a weekly scheduled run, actions pinned by commit hash,
  `rust-toolchain.toml`, and Dependabot alerts and security fixes on, with version updates off.
- The `deploy` script runs the same steps, audits included, and stops if any fails.

## Done when

- Parser tests cover a missing file, a missing value, `CHANGE_ME` and a badly formed `origin_key`,
  and that the example and the fixture hold the same required keys, with a real file holding `smoke_claim`
  still passing.
- CI runs on a push and passes.
- CI fails on a known advisory, shown once by temporarily pinning a vulnerable crate, and passes
  again once that advisory is in the ignore list.
- The weekly scheduled run exists, and Dependabot alerts and security fixes are on.

## Comments

- `cargo-audit` in the devcontainer is asked for in gratinierer/agentic_containers#3. It doesn't block
  this ticket, because CI installs the tool itself.
