# OpenAPI

- `sfab.yaml`: the one API document for every story, imported by API Gateway after the CDK fills in
  `{{api_lambda_arn}}` and `{{api_host}}` in memory. Each change sets `info.version` to the date.
- `public-security.yaml`: the Bearer scheme, and which operations need credentials, for the public copy.

`cdk/generate_public_openapi.py` builds the public copy, `build/openapi.yaml`, served at
`GET /v1/openapi.yaml`, and refuses to write one that leaks anything private. `scripts/check` runs it.
The contract tests are `cdk/tests/test_openapi.py`, and the worked proof-of-work example's test is
`lambda/core/tests/worked_example.rs`.
