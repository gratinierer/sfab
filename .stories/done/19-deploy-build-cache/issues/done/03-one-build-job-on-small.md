# Build the Lambdas one job at a time on ARM/Small

`spec.md` "Goal" 3 and "Scope".

## Scope

- The build stays on ARM/Small (`BUILD_GENERAL1_SMALL`, `cdk/deployer_stack.py:107`), whose minutes the free tier
  covers (`docs/aws-facts.md:128`), and compiles one crate at a time: `buildspec.yml` sets `CARGO_BUILD_JOBS: "1"`,
  quoted, in its `env: variables:` block, beside `shell: bash`, which applies it to every phase (only the build
  phase compiles), with a comment saying why: two big AWS SDK crates compiling at once
  pass ARM/Small's 4 GB (Comments, 2026-11-20). `scripts/check` and CI keep cargo's default, since they don't run on
  ARM/Small.
- A text-level test (in `cdk/tests/test_tool_versions.py` or `cdk/tests/test_deployer_stack.py`, beside the other
  buildspec tests) asserts the buildspec's `env.variables.CARGO_BUILD_JOBS` is the string `"1"` (read with `yaml.safe_load`, as the
  other buildspec tests do), and `test_deployer_stack.py:72` still
  asserts `BUILD_GENERAL1_SMALL`. Write the test first.
- `BUILD_TIMEOUT_MINUTES` (`cdk/deployer_stack.py:38`) is 60. A one-job build is expected to take 20-30 minutes, so it
  stays, unless the owner's build below takes more than 45 minutes: then say so in a comment here, and ask.
- `docs/aws-facts.md`: add two dated rows. One, `tested`: ARM/Small runs out of memory building the Lambdas with
  cargo's default jobs (build `e8caf7db`). Two, `memory` until the owner's build below: one build job fits
  ARM/Small, citing the local measurement in Comments and its caveats as the source. Set it to `tested`, with the
  build's id, once that build gets through `cargo lambda build`. Rows :127-:129 stay ARM/Small.

## Done when

- `scripts/check` passes, with the test asserting `CARGO_BUILD_JOBS` is `1` in the buildspec.
- `docs/aws-facts.md` has both memory rows, the second still `memory` until the owner's build.
- The owner has moved `production` to a commit with the buildspec change (CodeBuild reads `buildspec.yml` from
  `production`, `cdk/deployer_stack.py:101-103`, so no `SfabDeployer` deploy is needed), and a build started with no
  `--compute-type-override` gets through `cargo lambda build` on ARM/Small. Record its BUILD phase time here. If it
  still runs out of memory, the fallback is ARM/Medium at $0.0070 a minute (Comments), which the owner decides.
  This needs the owner's deploy, so if the agent can't check it, it leaves a comment here saying so.

## Comments

- 2026-11-20: found during story 2's first deploy. Build `sfab-deploy:e8caf7db-d979-42f6-aec3-208c9053abb1` on
  ARM/Small failed in `cargo lambda build`: `rustc` compiling `aws-sdk-lambda` was killed with `signal: 9,
  SIGKILL`. The same commit (`b46e588`) built and deployed `SfabBase`, `SfabApi` and `SfabWebsite` with
  `--compute-type-override BUILD_GENERAL1_MEDIUM` (builds `8772fe1d-…`, `41e3ddd1-…`, `68b9a51a-…`). Until this
  issue is deployed, every build needs that override.
- 2026-11-20: measured locally (x86_64 cross-compiling to arm64, a clean `cargo lambda build --release --arm64
  --locked` per run, the summed RSS of every compiler process sampled every 0.2 s; numbers may be about 15% off
  CodeBuild's native arm64 build). Today's settings with 2 jobs, as on ARM/Small's 2 vCPUs: peak 3,854 MiB, 327 s,
  the peak while `aws_sdk_apigateway` and `aws_sdk_lambda` compiled together. `codegen-units=16` for
  `aws-sdk-lambda` only, 2 jobs: 3,246 MiB, 324 s, not enough, since `aws_sdk_s3` (2,239 MiB) and
  `aws_sdk_apigateway` (2,009 MiB) can still overlap. One job: **2,315 MiB**, 635 s, peak `aws_sdk_s3` alone. The
  three Medium builds above took 11-12.5 minutes of BUILD phase; Medium costs $0.0070 a minute against Small's
  $0.0034 and isn't in the free tier (price list, 2026-11-20). The owner chose Small with one job, for the free
  minutes, accepting the longer build (2026-11-20, R8).
- 2026-11-20: implemented (`buildspec.yml` sets `CARGO_BUILD_JOBS: "1"`, a buildspec test asserts it, and
  `docs/aws-facts.md` has both rows, the second still `memory`). The last Done-when item is still open: the agent
  can't move `production` or start a build, so the owner's build on ARM/Small, its BUILD phase time, and setting
  the second row to `tested` with that build's id are the owner's.
- 2026-11-20 (owner's builds, run by claude-opus-5-5): on ARM/Small with one build job and no override, build
  `sfab-deploy:41dbb6a3-c8c5-4da1-a76c-60e84a022a94` built the Lambdas with no SIGKILL and deployed all four stacks, its
  BUILD phase in 1,190 s (under 45 minutes); `sfab-deploy:aca6660e-9b05-4c66-8473-f50675dea6eb` took 1,041 s. The
  `docs/aws-facts.md` row is `tested`. Done when met.
