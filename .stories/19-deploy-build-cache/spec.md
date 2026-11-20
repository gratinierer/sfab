# A working deploy build: its cache restores, it only builds and deploys, and it runs on ARM/Medium

GitHub issue: [gratinierer/sfab#19](https://github.com/gratinierer/sfab/issues/19)
Authored: 2026-11-20 by claude-opus-5-5

## Goal

The `sfab-deploy` CodeBuild project (`cdk/deployer_stack.py`, story 18) spends most of its install phase compiling
`cargo-audit`, and its build phase runs the whole test suite, for an audit and tests that don't belong in a
deploy. This story makes three changes:

1. **The cache restores.** The build caches the cargo and uv downloads in its S3 cache bucket, but never gets them
   back. Build `sfab-deploy:fd2f07ad-a345-4b28-a410-15771438178a` (2026-11-20) logged, at the start of
   DOWNLOAD_SOURCE:

   ```
   Unable to download cache: AccessDenied: User: arn:aws:sts::983314983598:assumed-role/SfabDeployer-Role1ABCC5F0-.../AWSCodeBuild-...
   is not authorized to perform: s3:ListBucket on resource: "arn:aws:s3:::sfabdeployer-cache18f6ee16-stozym8iowuy"
   because no identity-based policy allows the s3:ListBucket action
   ```

   Saving the cache at the end works ("Cache uploaded successfully (size: 145.66 MiB)"), since that needs only
   `s3:PutObject`. So each build saves a cache the next one can't read.

   Once `cargo-audit` leaves (2), the cache holds the cargo registry, git db and uv downloads only, which saves
   seconds per build. It's still worth one grant: the cache paths are there, and they've never worked.

2. **The deploy only builds and deploys.** `scripts/deploy` runs all of `scripts/check`: generate, build, audit and
   test. So the build installs `cargo-audit`, and runs `cargo test --workspace` and pytest before `cdk deploy`. With
   the cache failing, `cargo install cargo-audit` compiled from source on every build: 4 min 09 s of that build's
   263 s INSTALL phase. The other install steps took about 16 s together: uv, cargo-lambda and ziglang about 10 s,
   the CDK CLI about 4 s, and rustup with the pinned toolchain about 2 s.

   Auditing and testing check the code, which is GitHub's job and the local check's, not a deploy gate's (owner,
   2026-11-20). A deploy can't fix an advisory or a failing test, and an advisory in an unrelated crate shouldn't
   block an urgent fix. For advisories, GitHub already does that job: Dependabot alerts are on, and their email is
   the alert for new advisories against unchanged code (`.stories/2-identity-api/spec.md` "Delivery"). The audit
   and the tests still run wherever `scripts/check` runs: before each commit on a machine, and in the manual CI
   workflow. The deploy keeps what `cdk deploy` needs: the generated public copy and website pages, from the real
   config, and the arm64 Lambda binaries.

   **Accepted risk:** with CI manual only, nothing automatic checks that the commit `production` points at passed
   `scripts/check`. The owner runs it before moving `production` forward, as before each commit.
   The deploy drops the tests of the code, not the checks of the data it ships: the committed data files the
   Lambdas parse at runtime are still checked: the world files at synth, as today, and the federation files, which
   only `cargo test` checks today, by the deploy's build or synth (issue 02), so a bad edit fails the deploy
   rather than the first request.

3. **The build has enough memory.** ARM/Small (4 GB) runs out of memory compiling the Lambdas: build
   `sfab-deploy:e8caf7db-d979-42f6-aec3-208c9053abb1` (2026-11-20) had `rustc` killed with SIGKILL compiling
   `aws-sdk-lambda` under `cargo lambda`'s release settings. Story 2's first deploy ran on ARM/Medium (8 GB) with
   `--compute-type-override BUILD_GENERAL1_MEDIUM`. Dropping the audit and tests (2) doesn't help, because the
   failure is in the Lambda build, which the deploy keeps.

After this story the build restores its cache, installs no `cargo-audit`, runs only generate and build before
`cdk deploy` (and neither for `SfabDeployer` alone with no config), and runs on ARM/Medium.

## Scope

- **The cache restores** (issue 01). The build role gets `s3:ListBucket` on the cache bucket, in the existing
  bucket-level statement beside `s3:GetBucketAcl` and `s3:GetBucketLocation` (`cdk/deployer_stack.py` :153-156),
  scoped to that bucket only. `docs/aws-facts.md`'s CodeBuild row on the S3 caching permissions (:131) is
  corrected, since the build above shows its list isn't enough.
- **The deploy only builds and deploys** (issue 02). The first steps of `scripts/check` (Python environment,
  generate, build the Lambdas) move into a new `scripts/build`. `scripts/check` runs it, then the audit and the
  tests, so it does exactly what it does today. `scripts/deploy` runs `scripts/build` instead of `scripts/check`, on
  CodeBuild and on a machine alike. `buildspec.yml` no longer installs or caches `cargo-audit`. CI is unchanged: it
  runs `scripts/check`. The two federation files, which only `cargo test` checks today, still fail the deploy if
  they don't parse, and `docs/federation.md` and story 2's spec ("Delivery", "Build and deploy") follow.
- **ARM/Medium** (issue 03). `cdk/deployer_stack.py`'s `compute_type` becomes `BUILD_GENERAL1_MEDIUM`, and story 2's
  "Prerequisites" and `docs/aws-facts.md` name the ARM/Medium quota instead of ARM/Small's.

## Out of scope

- **When CI runs.** It stays manual only (owner, 2026-11-04). Turning its push and weekly triggers back on is for
  when the repo goes public (`.stories/12-public-repo/`), when Actions minutes on standard runners are free.
- **Requiring green CI before a deploy.** With CI manual, there's no run to require. That comes with the public
  repo too.
- **Caching `lambda/target`.** The release Lambda compile is the long step, but `lambda/target` isn't cached (CI's
  `ci.yml` does cache it). Its `aarch64-unknown-linux-gnu` part is 1.4 GB locally (2026-11-20), moved both ways on
  every build, and with fat LTO a cached `target/` saves the dependency compiles, not the final link. That waits
  until a build on ARM/Medium (issue 03) is timed, to compare the compile with the transfer.
- **Other ways to shorten the install**, such as a custom build image. Without `cargo-audit` the install takes about
  20 s.
- **The failure of that same build's BUILD phase**: `scripts/deploy` exited at once without output. That build ran
  the buildspec from before `d70af57` ("Install uv into the deploy build's tools venv"), which fixes the pyenv-shim
  uv under `cdk/`. If a build on the current `production` still fails that way, it gets a ticket of its own.
