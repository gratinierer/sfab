# CDK settings, `ip_hash_key` and the lifecycle rule

Label: ready-for-agent
Blocked by: 01, 2-identity-api#01

## Scope

- `genesis_codes_per_day = 100` and `genesis_ip_window_hours = 4` in the CDK code, passed to the Lambda and the daily
  job.
- `ip_hash_key` in `sfab.config.toml` and `sfab.config.example.toml`, 32 to 128 letters and digits, checked by
  `cdk/sfab_config.py` like `origin_key`.
- A lifecycle rule deleting `genesis/ips/` objects after 1 day.
- CloudFront forwards the viewer address to the Lambda, and nothing else can set it. Record what's confirmed in
  `docs/aws-facts.md`.
- `GET /world/genesis-code` reaches the Lambda through the REST API.

## Done when

- Synth refuses a missing or malformed `ip_hash_key`.
- A CDK test asserts the lifecycle rule and the two settings.
