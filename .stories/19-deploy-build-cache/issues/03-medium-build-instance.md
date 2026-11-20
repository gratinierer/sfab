# Run the deploy build on ARM/Medium

Label: ready-for-agent

`spec.md` "Goal" 3 and "Scope".

## Scope

- Change `compute_type` in `cdk/deployer_stack.py` (:107) from `BUILD_GENERAL1_SMALL` to `BUILD_GENERAL1_MEDIUM`,
  and the expected `ComputeType` in `cdk/tests/test_deployer_stack.py` (:72). Write the test first.
- Comment the line: ARM/Small's 4 GB runs out of memory compiling the Lambdas with `cargo lambda`'s release
  settings (fat LTO, one codegen unit), so the build needs ARM/Medium.
- Rename what still says "small": the comment `# The Arm small image` (`cdk/deployer_stack.py:29`), since the image
  name stays and only the size changes, and the test `test_the_project_runs_on_the_arm_small_image_and_deploys_all_by_default`
  (`cdk/tests/test_deployer_stack.py:67`).
- `.stories/2-identity-api/spec.md` "Prerequisites" (:1027-1029) asks for the ARM/Small build quota
  (`L-5ED1D25B`). Change it to the ARM/Medium one, with its quota code from Service Quotas. The account's
  ARM/Medium quota is already 60 (2026-11-20).
- `docs/aws-facts.md`: row :127 names `BUILD_GENERAL1_SMALL` as the compute type, and row :129 gives the ARM/Small
  quota. Correct both to ARM/Medium (its price-list name and quota code), and add a dated row for the memory
  failure, citing the two builds below.

## Done when

- `scripts/check` passes, with the test asserting `BUILD_GENERAL1_MEDIUM`.
- The spec's "Prerequisites" and `docs/aws-facts.md` name ARM/Medium and its quota.
- The owner has deployed `SfabDeployer` from a machine (`scripts/deploy SfabDeployer`), and a build started with no
  `--compute-type-override` gets through `cargo lambda build`. This needs the owner's deploy, so if the agent can't
  check it, it leaves a comment here saying so.

## Comments

- 2026-11-20: found during story 2's first deploy. Build `sfab-deploy:e8caf7db-d979-42f6-aec3-208c9053abb1` on
  ARM/Small failed in `cargo lambda build`: `rustc` compiling `aws-sdk-lambda` was killed with `signal: 9,
  SIGKILL`. The same commit (`b46e588`) built and deployed `SfabBase`, `SfabApi` and `SfabWebsite` with
  `--compute-type-override BUILD_GENERAL1_MEDIUM` (builds `8772fe1d-…`, `41e3ddd1-…`, `68b9a51a-…`). Until this
  issue is deployed, every build needs that override.
