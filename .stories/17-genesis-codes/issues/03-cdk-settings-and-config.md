# CDK settings, `ip_hash_key` and the lifecycle rule

Label: ready-for-agent
Blocked by: 01

## Scope

- `genesis_codes_per_day = 100` and `genesis_ip_window_hours = 4` in the CDK code. The environment variables, named
  here once for issues 01 and 02: the API Lambda (`cdk/stacks.py`) gets `IP_HASH_KEY` and `GENESIS_IP_WINDOW_HOURS`,
  and the daily job (`cdk/world_job.py`) gets `GENESIS_CODES_PER_DAY` and `GENESIS_IP_WINDOW_HOURS`. The exact-set env
  tests in `cdk/tests/test_stacks.py` and `cdk/tests/test_world_job.py` follow.
- `ip_hash_key` in `sfab.config.toml` and `sfab.config.example.toml`, 32 to 128 letters and digits, checked by
  `cdk/sfab_config.py` like `origin_key`, and passed to the API Lambda in its environment (`cdk/stacks.py`), the
  one place it's used. Unlike `origin_key`, it never goes to CloudFront or the API key.
- A lifecycle rule deleting `genesis/ips/` objects after 1 day.
- CloudFront forwards the viewer address to the Lambda, and nothing else can set it. Record what's confirmed in
  `docs/aws-facts.md`.
- `GET /world/genesis-code` reaches the Lambda through the REST API.
- IAM for `genesis/`: the API Lambda gets HEAD (GET) and DELETE on `genesis/codes/`, GET and PUT on `genesis/ips/` and
  `genesis/pool.json`; the daily job gets list, GET, PUT and DELETE under `genesis/`, with its `ListBucket` condition
  widened to the prefix. Update `API_READ_WRITE_PREFIXES`, `API_DELETE_PREFIXES` in `cdk/stacks.py`, `cdk/world_job.py`,
  their CDK tests, the permission lists in story 2's `spec.md` "Architecture", story 2's `spec.md` sentence
  "`DELETE` is the API Lambda's only delete" and the `cdk/stacks.py` comment "The only prefix where the API Lambda
  deletes", which both stop being true.

## Done when

- Synth refuses a missing or malformed `ip_hash_key`, and the exact-set env tests assert each Lambda gets the variables
  named in the scope, and no others.
- A CDK test asserts the lifecycle rule and the two settings.
- CDK tests assert the new `genesis/` permissions, and that nothing else widened.
- The deployed proofs (a birth's IP record matches the caller's real address, and a request carrying its own
  forwarding header can't change it) belong to `2-identity-api#01`'s Done when, because that deploy waits for this
  issue.
- Story 2, whose documents this issue edits, gets a `/review-story` pass after the edit, so its review isn't
  left stale (`docs/agents/issue-tracker.md` "Before implementing a story").
