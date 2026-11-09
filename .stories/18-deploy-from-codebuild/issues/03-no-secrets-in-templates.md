# No secret in a template

Label: ready-for-agent
Blocked by: 01

`spec.md` "The config secret", the dynamic-references bullet.

## Scope

- `origin_key` (the API Gateway API key value, `cdk/stacks.py` :400, and CloudFront's `x-api-key` origin header, :473)
  and `ip_hash_key` (the API Lambda's `IP_HASH_KEY`, :346) reach CloudFormation as dynamic references to
  `sfab/config`'s entries, not as values.
- First confirm in AWS's documentation which of these properties accept a Secrets Manager dynamic reference, and
  record each in `docs/aws-facts.md`. For any that doesn't, stop and ask the owner rather than choosing a workaround.
- Synth still loads and checks the values (issue 01); only the template changes.

## Done when

- A CDK test asserts that no synthesized template of any stack contains the fixture's `origin_key` or `ip_hash_key`
  text, and that each place above holds a `{{resolve:secretsmanager:sfab/config:SecretString:<key>}}` reference.
- `cdk/generate_public_openapi.py`'s existing check that `origin_key` never reaches the public description still
  passes.
