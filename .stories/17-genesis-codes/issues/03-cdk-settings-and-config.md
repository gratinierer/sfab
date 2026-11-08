# CDK settings, `ip_hash_key` and the lifecycle rule

Label: ready-for-agent
Blocked by: 01, 2-identity-api#01

## Scope

- `genesis_codes_per_day = 100` and `genesis_ip_window_hours = 4` in the CDK code, passed to the Lambda and the daily
  job.
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
  their CDK tests, and the permission lists in story 2's `spec.md` "Architecture".

## Done when

- Synth refuses a missing or malformed `ip_hash_key`, and a CDK test asserts it reaches the API Lambda's environment.
- A CDK test asserts the lifecycle rule and the two settings.
- CDK tests assert the new `genesis/` permissions, and that nothing else widened.
- On the first deploy with this issue, these are proven and recorded in `docs/aws-facts.md`: a birth's IP record
  matches the caller's real address, and a request carrying its own forwarding header can't change it.
