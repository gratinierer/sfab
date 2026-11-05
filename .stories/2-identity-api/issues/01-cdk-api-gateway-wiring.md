# CDK: build both stacks and wire CloudFront, the REST API and the Lambda

Label: ready-for-agent
Blocked by: 03, 05, 06

Build the REST API from `openapi/sfab.yaml` with `SpecRestApi` in the Python CDK, behind a
CloudFront distribution, as described in `spec.md` "Architecture". Filed from review finding
R12.

## Scope

- The base stack: the certificate for `<domain_name>` and `api.<domain_name>`, both buckets, both
  SNS topics with the email subscription and a policy on the alerts topic allowing both Budgets and
  CloudWatch, and all three budgets, as in `spec.md` "Abuse and cost controls",
  using the service name recorded in issue 03. Confirm the filter in the console's budget preview.
- The API stack: the API Lambda, from `lambda/target/lambda/api/bootstrap` on arm64, with `REQUIRED_BITS`
  (the CDK's `required_bits`), `IDENTITIES_BUCKET` and `CLAIMS_BUCKET` in its environment, which issue 06's
  `lambda/api/src/main.rs` reads, and its reserved concurrency, the REST API, the origin key,
  the stage's one default throttle, with access logs off, CloudFront with its plan, the WAF rate rule, the CloudFront Function and
  the `api.` DNS record.
- The API Lambda's permissions from `spec.md` "Architecture", with a CDK assertion test, and an email-only
  alarm on its errors to the alerts topic, like the kill switch's and the daily job's. Moved from issue 06
  (review R177), because 06 has no stack to put them in.
- The CDK assertion tests in `spec.md` "Testing" for these.

The code is agent work. The deploy needs the owner, because this container has no AWS credentials,
so the owner runs steps 4 to 6 of `spec.md` "First deploy", which also gives the proofs below.
The API has alert emails from this deploy, but no automatic stop until issue 07, which is the very next
deploy. Keep its URL unpublished until the public launch.

This is stage 1, along with issues 03-09. Later stages add routes to the same stacks.

## Traps to handle

- **No invoke permission:** `SpecRestApi` doesn't add a Lambda permission, because it
  doesn't know the integrations inside the OpenAPI file. Grant API Gateway the right to
  invoke the API Lambda explicitly, or every route fails.
- **Edits may not deploy:** CDK's automatic deployment ID hashes the API model, which
  `SpecRestApi` doesn't build from code, so later `sfab.yaml` edits might never reach
  the stage. Pass the OpenAPI definition to `addToLogicalId()` on the deployment, as the
  CDK docs recommend. This hasn't been tested on a real deploy.
- **No CloudWatch role:** access logs are off, so nothing needs API Gateway's account-level CloudWatch role.
  CDK's default depends on the `@aws-cdk/aws-apigateway:disableCloudWatchRole` flag, so set
  `cloudWatchRole` to false explicitly.
- **Origin key:** an API key only works once it's associated with a usage plan and stage. Create
  a usage plan on the `v1` stage, with no throttle because the stage holds the throttles, and
  associate the key with it. Put `security` on every operation in `sfab.yaml`, because
  root-level `security` is ignored. Leave the key's `Name` unset, because changing `Value`
  replaces the key, and a named key can't be replaced. Filed from review finding R61.
- **Target account:** every stack sets `env` to the config's `account_id` and `us-east-1`, so credentials for another
  account fail, as spec.md "Delivery" says, with an assertion test. `cdk/app.py` loads the config but builds no
  environment until there's a stack to use it. Filed from code review C25.
- **Placeholders:** `sfab.yaml` holds two, `{{api_lambda_arn}}` and `{{api_host}}`, for the unknown-route `404`'s
  `detail`. Fill in both, and fail the synth if any `{{` is left in the document passed to `SpecRestApi`, with an
  assertion test, because a missed one deploys fine and shows agents the raw placeholder. Filed from code review C12.
- **Origin path:** `RestApiOrigin` defaults its origin path to the stage name, but public URLs
  already include `/v1`, so requests would reach the origin as `/v1/v1/...`. Set the origin path
  to `/` explicitly, or use an `HttpOrigin`, and add a CDK assertion test. Filed from review
  finding R74.

## Done when

- Every route reaches the API Lambda after a first deploy on a fresh account, and a real `birth` through
  CloudFront succeeds, which proves the Lambda rebuilds the hashed path the client sent.
- CDK assertion tests cover the API Lambda's permissions and its errors alarm.
- An edit to `sfab.yaml` reaches the `v1` stage on the next `cdk deploy`.
- CDK assertion tests cover the invoke permission, access logs off, no CloudWatch role, and all three budgets' alerts and
  filters.
- On the first deploy through CloudFront, these are proven. AWS's docs support each, but none
  is tested, and the second isn't stated outright:
  - `GET /v1/identity/self` receives `Authorization` at the origin.
  - `CachingDisabled` and `AllViewerExceptHostHeader` are accepted on the Free flat-rate plan.
    If not, stop and bring it to the owner. Pro doesn't help, because custom caching rules
    start at Business.
  - The plan subscribes through `AWS::PricingPlanManager::Subscription`, with the hosted zone's ARN in
    its `ResourceArns`.
  - The Free plan accepts the web ACL, and with no custom responses configured, WAF answers a
    block with a plain `403` that isn't `problem+json`, both for the rate rule and when its
    default action is Block.
  - A CloudFront Function on the Free plan returns the `problem+json` `404` for a path outside
    `/v1/`, with a `detail` pointing to `/v1/openapi.yaml`.
  - A request to `http://` gets a `403` and never reaches the origin.
  - The email subscription was confirmed with `--authenticate-on-unsubscribe true`, as in `spec.md`
    "First deploy", and its attributes show that.
- On the first import and deploy of `sfab.yaml`, these `memory` rows of `docs/aws-facts.md` are
  proven or disproved, and their rows updated:
  - The import accepts `$default` as a `requestBody` content type.
  - The import doesn't resolve a `$ref` inside `x-amazon-apigateway-integration`, so the YAML
    anchor is needed.
  - `$context.error.messageString` arrives already JSON-quoted: a malformed body's `400` parses
    as JSON. If not, every gateway response template is wrong, so fix them before going on.
  - Whether `$context.error.validationErrorString` with `$util.escapeJavaScript` gives valid JSON
    in a gateway response template.
- Before anything else depends on it, the owner confirms in the console that the distribution shows
  the Free flat-rate plan, because otherwise CloudFront and WAF bill per request.

## Sources

- <https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/add-origin-custom-headers.html>
- <https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/template-preconfigured-origin-settings.html>
- <https://docs.aws.amazon.com/AWSCloudFormation/latest/TemplateReference/aws-resource-pricingplanmanager-subscription.html>
- <https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/flat-rate-pricing-plan.html>
- <https://docs.aws.amazon.com/cdk/api/v2/docs/aws-cdk-lib.aws_apigateway-readme.html>
- <https://github.com/aws/aws-cdk/blob/main/packages/aws-cdk-lib/aws-apigateway/lib/restapi.ts>
