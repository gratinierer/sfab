# Grant the build role s3:ListBucket on the cache bucket

Label: ready-for-agent

`spec.md` "Goal" 1 and "Scope".

## Scope

- Add `s3:ListBucket` to the bucket-level statement on `self.cache.bucket_arn` in `cdk/deployer_stack.py` (:153-156),
  beside `s3:GetBucketAcl` and `s3:GetBucketLocation`. No other resource or action changes.
- Update the expected statement in `test_the_build_role_has_exactly_its_grants_and_nothing_wider`
  (`cdk/tests/test_deployer_stack.py` :155-159) to include it. Write the test first.
- First check AWS's CodeBuild documentation on S3 cache permissions for whether it names `s3:ListBucket`, and correct
  `docs/aws-facts.md` :131 to the full list. Keep `docs` with the source if the docs name it, or cite the build's
  log (`sfab-deploy:fd2f07ad-a345-4b28-a410-15771438178a`, 2026-11-20) if they don't. Date the row.

## Done when

- `scripts/check` passes, with the test asserting `s3:ListBucket` on the cache bucket's ARN only.
- `docs/aws-facts.md` :131 names `s3:ListBucket` and says where that comes from.
- After the owner deploys `SfabDeployer` and runs two builds with issue 03 deployed (one build job on ARM/Small), or started with
  `--compute-type-override BUILD_GENERAL1_MEDIUM`, since today's ARM/Small build runs out of memory in
  `cargo lambda build`, the second build's log shows no
  `Unable to download cache`. Then set the `docs/aws-facts.md` row to `tested`. This needs the owner's deploy, so
  when the agent can't check it, it leaves a comment here saying so.
