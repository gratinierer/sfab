# Set up the repo, config and CI

Label: ready-for-agent

Create the monorepo and the config every later ticket uses: `spec.md` "Architecture" (layout) and
"Delivery" (local config, CI).

## Scope

- `/cdk/` with a Python virtual environment, `/openapi/`, `/lambda/` as a Cargo workspace, and
  `/world/`, `/federation/` and `/website/`, with placeholders that later tickets fill in.
- The config parser, reading `sfab.config.toml` or the file named by `SFAB_CONFIG`.
- `sfab.config.example.toml` with `CHANGE_ME` values, the gitignored real file, and the fixture
  `cdk/tests/fixtures/sfab.config.test.toml`.
- GitHub Actions, using the fixture, running the same steps as the devcontainer.

## Done when

- Parser tests cover a missing file, a missing value, `CHANGE_ME` and a badly formed `origin_key`,
  and that the example and the fixture hold the same keys.
- CI runs on a push and passes.
