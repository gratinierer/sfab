# Set up the repo, config and CI

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
- The weekly scheduled run exists, Dependabot alerts and security fixes are on, and the owner's notification
  settings send Dependabot alerts by email.

## Comments

- `cargo-audit` in the devcontainer is asked for in gratinierer/agentic_containers#3. It doesn't block
  this ticket, because CI installs the tool itself.
- 2026-11-04: built in the working tree. `scripts/check` holds the shared steps (Python environment, generate,
  arm64 build, `scripts/audit`, tests), and CI and `scripts/deploy` both run it. The parser is `cdk/sfab_config.py`,
  also rejecting unknown keys and an `account_id` that isn't 12 digits. One ignore list,
  `audit-ignore.toml`, feeds both audits. Locally, pinning `smallvec = "=1.6.0"` failed the audit on
  RUSTSEC-2021-0003, and it passed once that ID was listed; both changes reverted. Dependabot alerts and
  security fixes are on, with no `dependabot.yml`, so version updates stay off. Still open from "Done when":
  push and a passing CI run, repeating the advisory failure on CI, and the owner turning on email for
  Dependabot alerts under GitHub Settings → Notifications.
- 2026-11-04: done. CI passes on a push (run 41857988371, after fixing how it installs zig). Pinning `smallvec`
  1.6.0 failed CI on RUSTSEC-2021-0003 (run 41862372185), and it passed with that ID ignored (run 41865910350),
  then both were reverted. The owner turned on email for Dependabot alerts. The weekly run starts once the
  workflow reaches `main`, because GitHub runs `schedule` only from the default branch.
