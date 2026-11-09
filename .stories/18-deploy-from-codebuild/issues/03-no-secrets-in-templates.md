# No secret in a template

Label: ready-for-agent
Blocked by: 01

`spec.md` "The config secret", the dynamic-references bullet.

## Scope

- `origin_key` (the API Gateway API key value, `cdk/stacks.py` :400, and CloudFront's `x-api-key` origin header, :473)
  and `ip_hash_key` (the API Lambda's `IP_HASH_KEY`, :346) reach CloudFormation as dynamic references to
  `sfab/config`'s entries, not as values: `{{resolve:secretsmanager:sfab/config:SecretString:<key>}}`, the current
  version, with no version id. A version id would change every reference on any edit to the secret, since Secrets
  Manager versions the whole secret, and so replace the `OriginKey` API key, with 403s until the new key joins the
  usage plan, even when `origin_key` didn't change (R43).
- Each resource that holds a reference also carries a short sha256 prefix of its own value, where a change forces
  CloudFormation to update it: the `OriginKey` logical id (a new key, as today when the value changes), and a
  property of the API Lambda and of the CloudFront distribution's origin. So a resource changes only when its own
  value changes, and an edit to any other entry, such as adding `smoke_claim`, changes nothing. Both values are at
  least 20 random letters and digits (`cdk/sfab_config.py` :104-107), so the prefix gives nothing away.
- First confirm in AWS's documentation that CloudFormation doesn't re-resolve an unchanged dynamic reference, and
  that an update to the resource holding it (or its replacement) resolves the current version, and record both in
  `docs/aws-facts.md`. If either doesn't hold, stop and ask the owner.
- First confirm in AWS's documentation which of these properties accept a Secrets Manager dynamic reference, and
  record each in `docs/aws-facts.md`. For any that doesn't, stop and ask the owner rather than choosing a workaround.
- Synth still loads and checks the values (issue 01); only the template changes.
- Rewrite the `OriginKey` comment in `cdk/stacks.py` (:397-399), whose "a changed value replaces the key" assumes the
  value is in the template: the value's hash prefix in the logical id now replaces the key instead.

## Done when

- A CDK test asserts that no synthesized template of any stack contains the fixture's `origin_key` or `ip_hash_key`
  text, and that each place above holds a `{{resolve:secretsmanager:sfab/config:SecretString:<key>}}` reference with
  no version id.
- A CDK test asserts that a different `origin_key` changes the `OriginKey` logical id and the distribution, a
  different `ip_hash_key` changes the API Lambda, and a different value of any other entry, such as `alert_email`,
  leaves those three resources unchanged.
- `cdk/generate_public_openapi.py`'s existing check that `origin_key` never reaches the public description still
  passes.
