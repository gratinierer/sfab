# No secret in a template

Label: ready-for-agent
Blocked by: 01

`spec.md` "The config secret", the dynamic-references bullet.

## Scope

- `origin_key` (the API Gateway API key value, `cdk/stacks.py` :400, and CloudFront's `x-api-key` origin header, :473)
  and `ip_hash_key` (the API Lambda's `IP_HASH_KEY`, :346) reach CloudFormation as dynamic references to
  `sfab/config`'s entries, not as values. Each reference names the secret's version,
  `{{resolve:secretsmanager:sfab/config:SecretString:<key>::<version_id>}}`, with issue 01's `Config.version_id`, so a
  new value in the secret is a new version, which changes the template and makes CloudFormation update the resource.
- First confirm in AWS's documentation that CloudFormation doesn't re-resolve an unchanged dynamic reference, and
  that a changed version id in the reference updates the resource that holds it, and record both in
  `docs/aws-facts.md`. If either doesn't hold, stop and ask the owner.
- First confirm in AWS's documentation which of these properties accept a Secrets Manager dynamic reference, and
  record each in `docs/aws-facts.md`. For any that doesn't, stop and ask the owner rather than choosing a workaround.
- Synth still loads and checks the values (issue 01); only the template changes.
- Rewrite the `OriginKey` comment in `cdk/stacks.py` (:397-399), whose "a changed value replaces the key" assumes the
  value is in the template: a new secret version now changes the reference instead.

## Done when

- A CDK test asserts that no synthesized template of any stack contains the fixture's `origin_key` or `ip_hash_key`
  text, and that each place above holds a `{{resolve:secretsmanager:sfab/config:SecretString:<key>::<version_id>}}`
  reference with the fixture's `version_id`, and that a different `version_id` gives a different template.
- `cdk/generate_public_openapi.py`'s existing check that `origin_key` never reaches the public description still
  passes.
