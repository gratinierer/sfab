# AWS facts

AWS behaviour the project's stories rely on, with how far each fact can be trusted. Use it as a starting point, not as proof: facts go stale, and a review should still recheck anything old or high-stakes.

**Status**, strongest first:

- `tested`: seen on a real deploy.
- `docs`: read in AWS's documentation or source.
- `forum`: unofficial reports only.
- `memory`: nobody has checked it yet.

When a fact is checked, update its row and date. When a deploy proves or disproves one, set it to `tested`, or correct it.

## CloudFront flat-rate plans

| Fact | Status | Source | Checked |
|---|---|---|---|
| Free includes 1 million requests and 100 GB per month, with no overage charges | docs | [plans] | 2026-11-01 |
| Requests blocked by WAF don't count against the allowance | docs | [plans] | 2026-11-01 |
| Custom WAF responses are available from Pro, not on Free | docs | [plans] | 2026-11-01 |
| Free allows 5 WAF rules | docs | [plans] | 2026-11-01 |
| WAF request logs are available from Pro | docs | [plans] | 2026-11-01 |
| Header-based WAF filtering starts at Pro, and regex-based at Business | docs | [plans] | 2026-11-01 |
| Custom caching rules start at Business | docs | [plans] | 2026-11-01 |
| Default origin request rules, using managed policies, are on every plan | docs | [plans] | 2026-11-01 |
| Serverless edge compute is on every plan | docs | [plans] | 2026-11-01 |
| Pro costs $15 per month, and Business $200 | docs | [plans] | 2026-10-30 |
| Creating a subscription fails with `ValidationException` if an associated resource uses features outside the plan's tier | docs | [plan-api] | 2026-11-01 |
| CloudFormation can't change a plan's tier, and creates paid plans in `PENDING_APPROVAL` | docs | [plan-cfn] | 2026-11-01 |
| A CloudFront plan's `ResourceArns` must hold one distribution and one WAF web ACL, and may add a hosted zone; `PlanTier` is `FREE` and `UsageLevel` `DEFAULT` | docs, tested | [plan-cfn]; `SfabApi`'s `FlatRatePlan` (`AWS::PricingPlanManager::Subscription`) reached `CREATE_COMPLETE` on 2026-11-20 with the distribution, the web ACL and the hosted zone in `ResourceArns` | 2026-11-20 |
| Cancelling or downgrading a paid plan takes effect at the end of the month, while a downgrade's features go at once | docs | [plan-api] | 2026-11-01 |
| Free Tier accounts can't use flat-rate plans | docs | [plans] | 2026-10-30 |
| Each plan covers one distribution with up to one apex domain, and an account can have 3 Free plans | docs | [plans] | 2026-11-01 |
| Free allows 5 cache behaviors | docs | [plans] | 2026-11-01 |
| A plan can include a hosted zone, and past its allowance AWS might move the zone back to pay-as-you-go | docs | [plan-cfn], [plans] | 2026-11-01 |
| Free covers the zone's fee, 50 records, unlimited alias queries and 1 million other queries a month | docs | [plans], the reviewer's reading | 2026-11-01 |
| Two plans can share one apex domain, such as `<domain>` and `api.<domain>` | tested | Not stated by AWS; the first deploy proves it; 2026-11-20: `api.<domain>` (`EJS6XV549SRKF`) and `<domain>` (`E2J9VHRZ3JKB1J`) each on its own Free plan, both working, which the owner confirmed in the console | 2026-11-20 |
| `CachingOptimized` leaves query strings out of its cache key, with a 24-hour default TTL | docs | [cache-policies], the reviewer's reading | 2026-11-01 |
| A private S3 origin doesn't add `index.html` to a folder path | memory | | 2026-11-01 |
| `BucketDeployment` sets each object's content type by guessing from its extension, so an extensionless file gets S3's default `binary/octet-stream` | memory | Why story `10-website` sets the catalog's type in CloudFront | 2026-11-04 |
| `BucketDeployment`'s `contentType` applies to every object that deployment uploads | docs | CDK s3-deployment README, the reviewer's reading | 2026-11-04 |
| `BucketDeployment` prunes by default, deleting files not in its source, and only its own deployment-level `exclude` protects one, so two deployments on one bucket delete each other's files | docs | CDK s3-deployment README ("Prune", "Exclude and Include Filters"), the reviewer's reading; why story `10-website` uses one | 2026-11-04 |
| A CloudFront Function on viewer response can set `Content-Type` and add a `Link` header | memory | Ticket 03 of story `10-website` proves it on a deploy | 2026-11-04 |
| `S3BucketOrigin.with_origin_access_control` gives a private bucket an origin access control and the bucket policy that lets only that distribution read it | memory | `cdk/website_stack.py` relies on it; issue 12 of story `2-identity-api` proves it on a deploy | 2026-11-07 |
| A `cloudfront-js-2.0` viewer-request function can rewrite `request.uri` before the origin sees it, or answer with its own response, such as a `301` | memory | `cdk/website_stack.py`'s `folder_pages` function relies on it; issue 12 of story `2-identity-api` proves it on a deploy | 2026-11-07 |
| `CachingDisabled` is accepted on Free | tested | Not stated by AWS; issue 01 of story `2-identity-api` proves it; 2026-11-20: the API distribution deployed with it on its Free plan, and authenticated reads through CloudFront passed in the smoke run | 2026-11-20 |
| `HTTPS_ONLY` answers `http://` with `403` | docs, tested | [https-viewers], the reviewer's reading; issue 01 of story `2-identity-api` proves it; 2026-11-20: `http://api.<domain>/v1/world/constitution` answered `403` with `X-Cache: Error from cloudfront`, from CloudFront rather than the origin | 2026-11-20 |
| Port 80 can't be closed on a distribution | memory | | 2026-11-01 |
| A new account may need Support verification before creating CloudFront resources | forum | re:Post search summaries | 2026-11-01 |
| The managed origin request policy `AllViewerExceptHostHeader` (`b689b0a8-53d0-40ab-baf2-68738e2966ac`) forwards every viewer header except `Host`, and adds CloudFront's headers for protocol, HTTP version, TLS version, device type and viewer location | docs | [orp-managed] | 2026-11-08 |
| `CloudFront-Viewer-Address` is a viewer location header holding the viewer's IP address and source port, such as `198.51.100.10:46532`, and can be added in an origin request policy but not a cache policy | docs | [cf-headers] | 2026-11-08 |
| So `AllViewerExceptHostHeader` forwards `CloudFront-Viewer-Address` to the origin | docs, tested | [orp-managed] and [cf-headers] together, the reviewer's reading: the policy page says "all … viewer location headers" without naming each one. Issue 01 of story `2-identity-api` proves it, by a birth whose IP record matches the caller's address; 2026-11-20: a `birth` through CloudFront with an ordinary code wrote `genesis/ips/<HMAC>` of the caller's real IPv4, so the origin received the viewer address | 2026-11-20 |
| A `CloudFront-Viewer-Address` header the viewer sends is replaced by CloudFront's own value, so a client can't set it | tested | AWS doesn't say. Its list of request headers CloudFront removes or replaces omits it ([cf-custom-origin]). Story 17's IP rule relies on it; issue 01 of story `2-identity-api` proves it, by a birth that sends its own header; 2026-11-20: a `birth` sending `CloudFront-Viewer-Address`, `X-Forwarded-For`, `X-Real-IP` and `Forwarded` naming `203.0.113.77` got `409 too-soon` from the caller's real address's record, and no record for the forged address was written | 2026-11-20 |
| CloudFront appends the connecting address to a viewer's `X-Forwarded-For` rather than replacing it, so its leading entries are the viewer's to choose | docs | [cf-custom-origin] "Client IP addresses"; why story 17 reads `CloudFront-Viewer-Address` instead | 2026-11-08 |

## WAF

| Fact | Status | Source | Checked |
|---|---|---|---|
| Rate rules take up to several minutes to react, usually under 30 seconds | docs | [waf-rate] | 2026-11-01 |
| With no custom response configured, a WAF block, by a rule or by a default action of Block, is a plain `403` that isn't `problem+json` | tested (rule) | `cdk/smoke.py` and `cdk/waf_check.py` read it as "blocked"; issue 01 of story `2-identity-api` proves it on the first deploy; 2026-11-20: `scripts/waf-check` got a plain, non-`problem+json` `403` from the rate rule. A default action of Block is proven by `2-identity-api#10`'s kill switch drill | 2026-11-20 |
| The lowest rate limit is 10 | docs | Reviewer's reading of the WAF docs | 2026-11-01 |
| A custom response's body is text, HTML or JSON, and a custom header can't set `content-type` | docs | Earlier review, R18 | 2026-10-30 |
| WAF samples return each header's name and value for 3 hours | docs | Earlier review, R50 | 2026-10-30 |

## API Gateway

| Fact | Status | Source | Checked |
|---|---|---|---|
| An API key needs a usage plan to work | docs | [usage-plans] | 2026-11-01 |
| Root-level `security` is ignored, so it must be set per operation | docs | [known-issues] | 2026-11-01 |
| A `securitySchemes` type on import must be `apiKey` | docs | [known-issues] | 2026-11-01 |
| API key values are 20-128 alphanumeric characters, and adding a key to a plan can take minutes | docs | [usage-plans] | 2026-11-01 |
| In CloudFormation, changing a key's `Value` replaces it, and a named key can't be replaced | docs | [apikey-cfn] | 2026-11-01 |
| A gateway response can override its status code | docs | [gateway-response] | 2026-11-01 |
| In a REST proxy event, `path` lacks the stage, and Rust's `lambda_http` adds it unless `AWS_LAMBDA_HTTP_IGNORE_STAGE_IN_PATH` is set | memory | | 2026-11-03 |
| `$context.error.messageString` arrives already JSON-quoted, so a gateway response template uses it bare, as `"detail": $context.error.messageString`: `BAD_REQUEST_BODY` and `DEFAULT_5XX` both parse as JSON | tested | a throwaway REST API imported from the filled `openapi/sfab.yaml`, with no Lambda (story 2, issue 01, 2026-11-16) | 2026-11-16 |
| `$context.error.validationErrorString` holds double quotes, such as `(["name"])`, and inside a quoted JSON string `$util.escapeJavaScript($context.error.validationErrorString)` gives valid JSON, apostrophes included. It names what's wrong, such as `object has missing required properties (["name"])`, where `messageString` only says `Invalid request body` | tested | a throwaway REST API imported from the filled `openapi/sfab.yaml`, with no Lambda (story 2, issue 01, 2026-11-16) | 2026-11-16 |
| A pattern or enum failure's `validationErrorString` quotes the input that failed, such as `ECMA 262 regex "…" does not match input string "…"`, so it may hold a malformed claim or recovery code and stays out of every problem `detail` (`SECURITY_RULES.md` rule 2) | memory | Story 2's code review C166 removed it from `BAD_REQUEST_BODY`; the import tests saw only a missing-property message, and `cdk/tests/test_openapi.py` keeps it out of every gateway response | 2026-11-17 |
| A missing or wrong API key on a key-required method raises the `INVALID_API_KEY` gateway response, a `403`, rather than `DEFAULT_4XX` alone | tested | a throwaway REST API imported from the filled `openapi/sfab.yaml`, with no Lambda (story 2, issue 01, 2026-11-16); then the deployed API: the smoke run of 2026-11-20 (story 2, issue 09) got the `problem+json` `403` with `type: origin-key-replaced` from `execute-api` both without the key and with a wrong one | 2026-11-20 |
| An unknown route or method returns `MISSING_AUTHENTICATION_TOKEN` as `403`, and the `413` can't be customised | docs | [response-types] | 2026-11-01 |
| OpenAPI import accepts `$default` as a `requestBody` content type, giving a model for every content type, beside `application/json` | tested | a throwaway REST API imported from the filled `openapi/sfab.yaml`, with no Lambda (story 2, issue 01, 2026-11-16): `POST /identity/birth`'s `requestModels` held both | 2026-11-16 |
| On `INVALID_API_KEY` and `MISSING_AUTHENTICATION_TOKEN`, the response's `Content-Type` comes from the template's media type key, so a template keyed `application/json` answers `application/json` and ignores the `gatewayresponse.header.Content-Type` parameter. Keyed `application/problem+json`, it answers `application/problem+json` whatever the request's `Accept`. `BAD_REQUEST_BODY` and `DEFAULT_5XX` honour the parameter. `Cache-Control` is honoured on all four | tested | a throwaway REST API imported from the filled `openapi/sfab.yaml`, with no Lambda (story 2, issue 01, 2026-11-16) | 2026-11-16 |
| OpenAPI import, without failing on warnings, accepts `openapi/sfab.yaml`, and only warns that a `default` response and the string schema of `GET /openapi.yaml` are ignored | tested | a throwaway REST API imported from the filled `openapi/sfab.yaml`, with no Lambda (story 2, issue 01, 2026-11-16) | 2026-11-16 |
| OpenAPI import doesn't resolve a `$ref` inside `x-amazon-apigateway-integration`, so `openapi/sfab.yaml` repeats it with a YAML anchor instead | memory | Issue 01 of story `2-identity-api` checks it on import | 2026-11-04 |
| REST requests cost $3.50 per million | docs | AWS's pricing page | 2026-11-01 |
| A CloudFormation `RestApi` with both `Body` and `Name` is named `Name`, not the OpenAPI `info.title`, so CDK's `metricCount()` names the right `ApiName` | memory | AWS doesn't say which wins; the kill switch reads the name with `GetRestApi`, and issue 10 checks that the 50,000-call warning's metric has data | 2026-11-07 |
| `disableExecuteApiEndpoint` affects all stages, returns `403`, and takes effect only after a stage update | memory | spec.md "Kill switch" says AWS documents it; not rechecked. CloudFormation's `RestApi` page only says it disables the default endpoint | 2026-11-07 |
| `CreateDeployment` with a `stageName` points that existing stage at the new deployment | memory | The kill switch's stop step and the restart list rely on it; issue 10's drill proves it | 2026-11-07 |
| Requests rejected for a missing API key, throttled requests and requests to a disabled default endpoint aren't charged | tested | Issue 02 of story `2-identity-api`: on 2026-11-16, Cost Explorer's `USE1-ApiGatewayRequest` showed 534 for 520 `200`s plus up to 37 other requests, against 905 + 2,759 + 1,667 rejections; [old-pricing] said the same, without covering a disabled endpoint | 2026-11-17 |

## CDK

| Fact | Status | Source | Checked |
|---|---|---|---|
| `RestApiOrigin` defaults its origin path to the stage name | docs | [rest-api-origin], v2.275.0, line 44 | 2026-11-01 |
| CDK drops a trailing `/` from an origin path, so `origin_path="/"` synthesizes an empty `OriginPath`, which CloudFront's rule that a path mustn't end in `/` allows | docs | CDK v2.275.0's synth, pinned by `cdk/tests/test_stacks.py` (`test_the_origin_is_the_api_without_a_doubled_stage…`) | 2026-11-07 |
| A distribution defaults to `ALLOW_GET_HEAD`, and to `ALLOW_ALL` for the viewer protocol unless a flag makes it redirect | docs | [cdk-behavior] | 2026-11-01 |
| `BucketDeployment` copies a folder during `cdk deploy`, refreshes a distribution's `distribution_paths` with an invalidation, and needs no Docker | memory | Issue 12 of story `2-identity-api` proves it; `cdk/website_stack.py` relies on the refresh | 2026-11-07 |
| `SpecRestApi` adds no Lambda invoke permission, and creates no method resources | docs | Earlier review, R12 and R61 | 2026-10-30 |
| `cdk bootstrap aws://<account>/<region>` still synthesizes the app when run beside a `cdk.json`, though it names the target, so run it where there's none | docs | CDK CLI 2.1147.0 source, `defineEnvironments` in `lib/cli/cdk-toolkit.js`; corrected from "doesn't need the app" | 2026-11-04 |
| The CDK CLI sets `CDK_DEFAULT_ACCOUNT` for the app from the account of the resolved credentials, and leaves it unset when it can't resolve one, so `sfab_config.deploy_account()` then calls STS, and synth with neither credentials nor `SFAB_CONFIG` fails | docs | CDK CLI 2.1146.1 source, `prepareDefaultEnvironment` in `lib/index.js` (toolkit-lib `api/cloud-assembly/environment.ts`); [cdk-env] says only that it comes from the credentials | 2026-11-09 |

## Secrets Manager and the SDK

| Fact | Status | Source | Checked |
|---|---|---|---|
| `GetSecretValue` raises `ResourceNotFoundException` both for a missing secret and for one with no `AWSCURRENT` value, the second with a message naming the staging label, which is how `sfab_config` tells them apart | memory | [sm-get] documents only the code, not the messages; the first `scripts/deploy SfabDeployer` before the secret is filled shows the real message | 2026-11-09 |
| A secret value's `SecretString` is at least 1 character, so an empty value can't be stored, but a blank one can | docs | [sm-get] | 2026-11-09 |
| boto3 (botocore 1.43) resolves the region from `AWS_DEFAULT_REGION`, then the profile, and ignores `AWS_REGION`, so `sfab_config.Aws.region()` reads `AWS_REGION` first itself | docs | botocore 1.43.126 source, `BOTOCORE_DEFAUT_SESSION_VARIABLES["region"]` in `configprovider.py`, and a session with only `AWS_REGION` set resolving `None` | 2026-11-09 |
| boto3 can use `aws login` credentials only with the `crt` extra: without it, every call fails with "Missing Dependency: Using the login credential provider requires an additional dependency. You will need to pip install "botocore[crt]"", so `cdk/requirements.in` pins `boto3[crt]` | tested | The first `scripts/deploy SfabDeployer` (story 2, issue 03), then `sfab_config.load()` with `awscrt` 0.38.0 | 2026-11-15 |
| An `AWS::SecretsManager::Secret` with neither `SecretString` nor `GenerateSecretString` creates an empty secret, with no value | docs | [cfn-secret], both properties: "If you omit both … you create an empty secret" | 2026-11-09 |
| CDK's `secretsmanager.Secret` with neither `secret_string_value` nor `generate_secret_string` synthesizes `GenerateSecretString: {}`, so the secret starts with a random password, and an update could regenerate it; `SfabDeployer` uses a raw `CfnSecret` instead | docs | aws-cdk-lib 2.275.0's synth | 2026-11-09 |
| A secret costs $0.40 a month, and API calls $0.05 per 10,000 | docs | [sm-pricing] | 2026-11-09 |
| CloudFormation retrieves a `secretsmanager` dynamic reference's value only when it creates a resource or when an update modifies the resource holding the reference; a changed secret alone isn't picked up, so an unchanged reference isn't resolved again | docs | [cfn-dynref-sm] "Considerations" | 2026-11-09 |
| A `secretsmanager` dynamic reference with neither `version-stage` nor `version-id` resolves the `AWSCURRENT` version, so the update or replacement that resolves it reads the current value | docs | [cfn-dynref-sm] "Reference pattern" | 2026-11-09 |
| The `secretsmanager` dynamic reference can be used in all resource properties, except in custom resources, `AWS::CloudFormation::Init` metadata and EC2 `UserData`, and shouldn't sit in a property of the resource's primary identifier. So `AWS::ApiGateway::ApiKey`'s `Value` (its `Ref` is the key ID, not the value), `AWS::Lambda::Function`'s `Environment.Variables` and `AWS::CloudFront::Distribution`'s origin `OriginCustomHeaders` `HeaderValue` all accept one | docs | [cfn-dynref-sm] "Considerations", [cfn-dynref] "General considerations", [apikey-cfn] "Ref", the implementer's reading | 2026-11-09 |
| A stack template can hold up to 60 dynamic references | docs | [cfn-dynref] | 2026-11-09 |

## CodeBuild and CodeConnections

| Fact | Status | Source | Checked |
|---|---|---|---|
| A CodeBuild project reads a GitHub repo through a CodeConnections connection with source `Type: GITHUB`, the clone URL as `Location`, and `Auth` `Type: CODECONNECTIONS` with the connection's ARN as `Resource` | docs | [cb-source], [cb-sourceauth] (`OAUTH`, `CODECONNECTIONS` or `SECRETS_MANAGER`), [cb-tokens] | 2026-11-09 |
| The build role needs `codeconnections:GetConnection` and `codeconnections:GetConnectionToken` on the connection | docs | [cb-github-app], [cb-tokens] | 2026-11-09 |
| `AWS::CodeConnections::Connection` with `ProviderType: GitHub` is created `PENDING`, and only the console makes it `AVAILABLE` | docs | [cfn-connection] | 2026-11-09 |
| The Arm small image is `aws/codebuild/amazonlinux-aarch64-standard:3.0` (Amazon Linux 2023), on environment type `ARM_CONTAINER` with compute type `BUILD_GENERAL1_SMALL` (`arm1.small`) | docs | [cb-images]; the compute type from the price list's `arm1.small` | 2026-11-09 |
| `arm1.small` on demand costs $0.0034 a minute in us-east-1, so a 20-minute build about 7 cents; the free tier gives 100 minutes a month of `general1.small` or `arm1.small` | docs | [cb-price-list] (`USE1-Build-Min:ARM:g1.small`), [cb-pricing] for the free tier | 2026-11-09 |
| The default quota of concurrently running builds for ARM/Small is 1 per region, adjustable, though "internal metrics will determine the default quotas", and forum reports say a new account can start at 0 | docs, forum | [cb-quotas] (`L-5ED1D25B`); the 0 is unconfirmed, so check Service Quotas before the first build | 2026-11-09 |
| ARM/Small (`BUILD_GENERAL1_SMALL`, 4 GB) runs out of memory building the Lambdas with cargo's default jobs: `rustc` compiling `aws-sdk-lambda` is killed with `signal: 9, SIGKILL` in `cargo lambda build` | tested | Build `sfab-deploy:e8caf7db-d979-42f6-aec3-208c9053abb1` (2026-11-20); `.stories/19-deploy-build-cache/issues/03-one-build-job-on-small.md` Comments | 2026-11-20 |
| With one build job (`CARGO_BUILD_JOBS` set to 1 in `buildspec.yml`), building the Lambdas fits ARM/Small | tested | Build `sfab-deploy:41dbb6a3-c8c5-4da1-a76c-60e84a022a94` (`--all` from `production` at `3522f41`, no override) built and deployed with no SIGKILL, its BUILD phase in 1,190 s, and `aca6660e-9b05-4c66-8473-f50675dea6eb` in 1,041 s. Before it, a local measurement: x86_64 cross-compiling to arm64, peak 2,315 MiB with one job against 3,854 MiB with two | 2026-11-20 |
| A build's environment has `AWS_REGION` and `AWS_DEFAULT_REGION` set to the region the build runs in, the project's, which `deploy_plan`'s region check reads | docs | [cb-env-vars] | 2026-11-09 |
| The service role CodeBuild needs for S3 caching and CloudWatch logs: `s3:GetObject`, `s3:PutObject`, `s3:GetBucketAcl`, `s3:GetBucketLocation`, `s3:ListBucket`, `logs:CreateLogGroup`, `logs:CreateLogStream`, `logs:PutLogEvents`, which `SfabDeployer` scopes to its cache bucket and log group | docs, tested | [cb-service-role], AWS's example with `Resource: *`, which leaves out `s3:ListBucket` (the next row); neither it nor [cb-caching-s3] names it (checked 2026-11-20). With the full list, builds `sfab-deploy:41dbb6a3-c8c5-4da1-a76c-60e84a022a94` and `aca6660e-9b05-4c66-8473-f50675dea6eb` restored the cache (`Unarchiving cache...`, no `Unable to download cache`) | 2026-11-20 |
| Without `s3:ListBucket` on the cache bucket, restoring the S3 cache fails with `Unable to download cache: AccessDenied ... not authorized to perform: s3:ListBucket`, while saving it works | tested | build `sfab-deploy:fd2f07ad-a345-4b28-a410-15771438178a`'s log | 2026-11-20 |
| A `CodeBuild Build State Change` event's `detail` carries `build-status` (`IN_PROGRESS`, `SUCCEEDED`, `FAILED`, `STOPPED`) and `project-name` | docs | [cb-notifications] | 2026-11-09 |
| EventBridge publishes to an SNS topic through the topic's policy for `events.amazonaws.com`; AWS's SNS example has no condition, and `SfabBase` adds `aws:SourceArn` of the rule, as AWS's examples for other targets do | memory | [eb-resource-policy]; the first failed build proves the email arrives | 2026-11-09 |
| `cdk deploy` assumes the bootstrap's `deploy`, `file-publishing`, `image-publishing` and `lookup` roles, named `cdk-hnb659fds-<name>-role-<account>-<region>`, and CloudFormation assumes `cfn-exec`, so the build needs `sts:AssumeRole` on the first four only | docs | [cdk-bootstrap]; the names from CDK CLI 2.1146.1's `bootstrap-template.yaml` | 2026-11-09 |

## Route 53, Budgets and other services

| Fact | Status | Source | Checked |
|---|---|---|---|
| An unverified registrant email forces AWS to suspend the domain, and only some TLDs and new addresses need it | docs | [domain-register] | 2026-11-01 |
| Credits can't pay for domain registration | docs | [domain-register] | 2026-11-01 |
| Budgets update every 8-12 hours, plus an unstated billing delay | docs | Earlier review, R16 | 2026-10-30 |
| An actual-spend alert fires only once per budget period | docs | Earlier review, R2 | 2026-10-30 |
| `DeletionPolicy: RetainExceptOnCreate` deletes a resource when its creating operation rolls back | docs | [deletion-policy] | 2026-11-01 |
| S3 lifecycle rules match the start of a path or an object tag, with no wildcard in the middle | docs | [s3-lifecycle], the reviewer's reading | 2026-11-01 |
| A conditional PUT that fails with `412` is billed like any PUT, which the cost figures for moving in and the Square assume | memory | | 2026-11-03 |
| `PutObject` with `If-None-Match: *` fails with `412` when the key exists, and with `409` when a concurrent conditional write wins | memory | `birth`'s claim write, in `lambda/identity`; only its tests with a reseeded exempt code (`lambda/identity/tests/identity.rs`) reach it, against a fake, since a deployed replay meets its used genesis code first | 2026-11-05 |
| `PutObject` with `If-Match` fails with `412` on a changed ETag and `404` on a deleted key, and `GetObject` returns the ETag that `If-Match` takes | memory | `reclaim`, in `lambda/identity`, `lambda/core/src/s3.rs`, and an attempt's rewrites in `lambda/world/academy` | 2026-11-05 |
| S3 rounds a lifecycle expiry up to the next midnight UTC, and removes expired objects some time later, so a route must filter by date itself | docs | [s3-expire], the reviewer's reading; relied on by the Square's 7-day notices (`lambda/world/square`) and the Academy's 30-day applications under `academy/applications/` (`lambda/world/academy/src/apprenticeship.rs`, rule in `cdk/stacks.py`) | 2026-11-01 |
| A listing returns up to 1,000 keys per request, with each object's size | memory | | 2026-11-01 |
| `ListObjectsV2` with `StartAfter` and `MaxKeys` returns the keys after that one in UTF-8 binary order, and `IsTruncated` says whether more follow, so the last key names the next page | memory | The messages list, in `lambda/messaging` and `lambda/core/src/s3.rs` | 2026-11-07 |
| `GetObject` on a missing key returns `404` only with `s3:ListBucket` on the bucket, and `403` without it, so the API Lambda has `ListBucket` | docs | [s3-getobject] | 2026-11-07 |
| An alarm with a period of at least one hour can evaluate up to 7 days | docs | [alarms] | 2026-11-01 |
| An alarm is evaluated every minute over a sliding window by default, or a wall clock window if set | docs | [alarm-evaluation], [alarm-window] | 2026-11-01 |
| An alarm's `EvaluationWindow` set to a `WallClockWindow` aligns its period to clock boundaries in a time zone, UTC unless set, so a one-day alarm counts from midnight | docs | CDK 2.275.0's `CfnAlarm` reference, from the CloudFormation schema; that a partial day can breach is memory, and issue 10's alarm check forces the alarm only | 2026-11-07 |
| An alarm acts only when its state changes | docs | [alarms] | 2026-11-01 |
| The missing-data setting is ignored when enough older real datapoints exist, and an alarm may re-evaluate old data after a metric stops | docs | [alarms-missing] | 2026-11-01 |
| A standard alarm costs $0.10 per metric per month, with 10 free | docs | [cw-pricing], from search results | 2026-11-01 |
| Lambda `Invocations` equals the requests billed, and excludes throttled invocations | docs | [lambda-metrics] | 2026-11-01 |
| API Gateway `Count` (`SampleCount`) includes requests rejected for a missing API key (`403`) and throttled requests (`429`), but requests to a disabled default endpoint (`403`) appear in no `Count`, not even the account-wide one; `SampleCount` is the statistic the kill switch and the 50,000-call warning use | tested | Issue 02 of story `2-identity-api`, the 2026-11-16 runs: 905 of 905, 3,279 of 3,279, and 0 of 1,667; [apigw-metrics] documents `SampleCount` | 2026-11-17 |
| API Gateway's `ApiName` metric dimension is the REST API's name, with non-ASCII characters removed | docs | [apigw-metrics] | 2026-11-07 |
| API Gateway's account-level limit is about 10,000 requests per second | memory | | 2026-11-01 |
| API Gateway's per-API metrics are sent at one-minute periods, and are free | docs | [apigw-monitoring], [cw-pricing], from search results | 2026-11-01 |
| DynamoDB on-demand writes cost about $0.625 per million | docs | [dynamodb-pricing], from search results | 2026-11-01 |
| AWS Cost Anomaly Detection is free | memory | | 2026-11-01 |
| A budget takes either `FilterExpression` and `Metrics`, or `CostFilters` and `CostTypes`, not both, and `CostTypes` is deprecated | docs | [budgets-create], [budgets-budget] | 2026-11-01 |
| `RECORD_TYPE` is a budget filter key, and "Credit" is a charge type | docs | [budgets-values], [budgets-filters] | 2026-11-01 |
| Credits are excluded by a `RECORD_TYPE` value of exactly `Credit` | memory | Issue 01 of story `2-identity-api` confirms it in the console | 2026-11-01 |
| Changing a budget resets its calculated spend to zero until new usage data arrives | docs | [budgets-update] | 2026-11-01 |
| A budget can have up to ten notifications, each with one SNS subscriber and up to 10 email subscribers. Corrected from five: the `Notification` page now says ten, while spec.md "How the budgets measure spend" still gives five as a reason for three budgets | docs | [budgets-notification] | 2026-11-07 |
| CloudFormation's schema for `AWS::Budgets::Budget` refuses a notification `Threshold` below 0.1, though the Budgets API allows 0, so the check budget alerts at 1% of $1 rather than at an absolute $0.01 | docs | CDK 2.275.0's synth validation against the resource schema (rule F3034); [budgets-notification] | 2026-11-07 |
| `ConfirmSubscription` with `AuthenticateOnUnsubscribe` true disallows unauthenticated unsubscribes | docs | [sns-confirm] | 2026-11-01 |
| Each SNS email has an unsubscribe link needing no login, and the subscription attribute `ConfirmationWasAuthenticated` shows how it was confirmed | tested | Issue 01 of story `2-identity-api` confirms it; 2026-11-20: the alerts subscription, confirmed with `--authenticate-on-unsubscribe true`, shows `ConfirmationWasAuthenticated` `true` | 2026-11-20 |
| Domain charges appear under the service name "Amazon Registrar" | tested | Cost Explorer, the first domain purchase (story 2, issue 03) | 2026-11-15 |
| The VAT on a domain charge appears separately, under the service "Tax", so the stop budget's "Amazon Registrar" exclusion doesn't cover it. The owner accepts that | tested | Cost Explorer, the first domain purchase | 2026-11-15 |
| Updating a budget replaces all of it, so a filter must be sent again | memory | | 2026-11-01 |
| `DescribeBudget` returns `CalculatedSpend.ActualSpend` and `BudgetLimit` as strings, and needs `budgets:ViewBudget` on `arn:aws:budgets::<account>:budget/<name>` | memory | The kill switch's daily spend check | 2026-11-07 |
| WAF's `UpdateWebACL` replaces the whole web ACL and needs the `LockToken` from `GetWebACL`, so a change of default action sends the rules back too | memory | The kill switch and `docs/restart-after-a-stop.md` | 2026-11-07 |
| A web ACL's CloudFormation `Ref` is `name\|id\|scope` | memory | The kill switch splits it | 2026-11-07 |
| SNS invokes a subscribed Lambda asynchronously, and EventBridge schedules too, so Lambda retries a failed run twice by default | memory | Why the kill switch keeps the default retries | 2026-11-07 |
| An asynchronous invocation throttled for lack of concurrency, such as at reserved concurrency 0, isn't a function error: Lambda keeps it queued and retries it for up to the maximum event age, 6 hours by default, whatever `MaximumRetryAttempts` says | memory | Why a stop at midnight can delay the daily world job's run rather than skip it; `cdk/world_job.py` and `docs/restart-after-a-stop.md` step 3 | 2026-11-07 |
| `GetObject` on a missing key returns `403` when `s3:ListBucket` is allowed only under a condition on `s3:prefix`, because the request has no prefix to match | memory | Why the daily world job lists `world/` first and reads only keys it listed (`lambda/world/census`), lists each `home.json` by its full key before reading it (`lambda/world/homes`), and lists `academy/` before reading a course or certificate (`lambda/world/academy`) | 2026-11-07 |
| A hosted zone costs about $0.50 per month without a plan | docs | The Route 53 pricing page, the reviewer's reading | 2026-11-01 |
| Route 53 bills every query answered except alias records, at $0.40 per million up to a billion | docs | The Route 53 pricing page | 2026-11-01 |
| S3 Standard in `us-east-1` charges $0.005 per 1,000 PUT, COPY, POST or LIST requests and $0.0004 per 1,000 GET and all other requests, HEAD among them, and DELETE is free, as the cost tables in story `2-identity-api`'s spec.md "Messages" and "What it can cost" use | docs | AWS price list for `AmazonS3`, `us-east-1` ([s3-price-list]: "$0.004 per 10,000 GET and all other requests"); DELETE from [s3-pricing] | 2026-11-08 |
| Lambda prices used in cost tables | memory | | 2026-11-01 |

[budgets-notification]: https://docs.aws.amazon.com/aws-cost-management/latest/APIReference/API_budgets_Notification.html
[s3-getobject]: https://docs.aws.amazon.com/AmazonS3/latest/API/API_GetObject.html
[s3-lifecycle]: https://docs.aws.amazon.com/AmazonS3/latest/userguide/intro-lifecycle-rules.html
[s3-expire]: https://docs.aws.amazon.com/AmazonS3/latest/userguide/lifecycle-expire-general-considerations.html
[plans]: https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/flat-rate-pricing-plan.html
[plan-api]: https://docs.aws.amazon.com/PricingPlanManager/latest/UserGuide/getting-started-pricingplanmanager-api.html
[plan-cfn]: https://docs.aws.amazon.com/AWSCloudFormation/latest/TemplateReference/aws-resource-pricingplanmanager-subscription.html
[waf-rate]: https://docs.aws.amazon.com/waf/latest/developerguide/waf-rule-statement-type-rate-based-caveats.html
[usage-plans]: https://docs.aws.amazon.com/apigateway/latest/developerguide/api-gateway-api-usage-plans.html
[known-issues]: https://docs.aws.amazon.com/apigateway/latest/developerguide/api-gateway-known-issues.html
[apikey-cfn]: https://docs.aws.amazon.com/AWSCloudFormation/latest/TemplateReference/aws-resource-apigateway-apikey.html
[gateway-response]: https://docs.aws.amazon.com/apigateway/latest/developerguide/api-gateway-swagger-extensions-gateway-responses.gatewayResponse.html
[response-types]: https://docs.aws.amazon.com/apigateway/latest/developerguide/supported-gateway-response-types.html
[old-pricing]: https://web.archive.org/web/20211202141251/https://docs.aws.amazon.com/apigateway/latest/developerguide/api-gateway-pricing.html
[rest-api-origin]: https://github.com/aws/aws-cdk/blob/v2.275.0/packages/aws-cdk-lib/aws-cloudfront-origins/lib/rest-api-origin.ts
[cdk-behavior]: https://docs.aws.amazon.com/cdk/api/v2/docs/aws-cdk-lib.aws_cloudfront.BehaviorOptions.html
[domain-register]: https://docs.aws.amazon.com/Route53/latest/DeveloperGuide/domain-register.html
[budgets-create]: https://docs.aws.amazon.com/aws-cost-management/latest/APIReference/API_budgets_CreateBudget.html
[budgets-budget]: https://docs.aws.amazon.com/aws-cost-management/latest/APIReference/API_budgets_Budget.html
[budgets-values]: https://docs.aws.amazon.com/aws-cost-management/latest/APIReference/API_budgets_ExpressionDimensionValues.html
[budgets-filters]: https://docs.aws.amazon.com/cost-management/latest/userguide/budgets-create-filters.html
[budgets-update]: https://docs.aws.amazon.com/aws-cost-management/latest/APIReference/API_budgets_UpdateBudget.html
[sns-confirm]: https://docs.aws.amazon.com/sns/latest/api/API_ConfirmSubscription.html
[alarm-evaluation]: https://docs.aws.amazon.com/AmazonCloudWatch/latest/monitoring/alarm-evaluation.html
[alarm-window]: https://docs.aws.amazon.com/AmazonCloudWatch/latest/monitoring/alarm-evaluation-window.html
[alarms-missing]: https://docs.aws.amazon.com/AmazonCloudWatch/latest/monitoring/alarms-and-missing-data.html
[cache-policies]: https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/using-managed-cache-policies.html
[https-viewers]: https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/using-https-viewers-to-cloudfront.html
[alarms]: https://docs.aws.amazon.com/AmazonCloudWatch/latest/monitoring/AlarmThatSendsEmail.html
[cw-pricing]: https://aws.amazon.com/cloudwatch/pricing/
[lambda-metrics]: https://docs.aws.amazon.com/lambda/latest/dg/monitoring-metrics-types.html
[apigw-metrics]: https://docs.aws.amazon.com/apigateway/latest/developerguide/api-gateway-metrics-and-dimensions.html
[apigw-monitoring]: https://docs.aws.amazon.com/apigateway/latest/developerguide/monitoring-cloudwatch.html
[dynamodb-pricing]: https://aws.amazon.com/dynamodb/pricing/
[deletion-policy]: https://docs.aws.amazon.com/AWSCloudFormation/latest/TemplateReference/aws-attribute-deletionpolicy.html
[s3-price-list]: https://pricing.us-east-1.amazonaws.com/offers/v1.0/aws/AmazonS3/current/us-east-1/index.json
[s3-pricing]: https://aws.amazon.com/s3/pricing/
[orp-managed]: https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/using-managed-origin-request-policies.html
[cf-headers]: https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/adding-cloudfront-headers.html
[cf-custom-origin]: https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/RequestAndResponseBehaviorCustomOrigin.html
[cdk-env]: https://docs.aws.amazon.com/cdk/v2/guide/configure-env.html
[sm-get]: https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_GetSecretValue.html
[cfn-secret]: https://docs.aws.amazon.com/AWSCloudFormation/latest/TemplateReference/aws-resource-secretsmanager-secret.html
[sm-pricing]: https://aws.amazon.com/secrets-manager/pricing/
[cb-source]: https://docs.aws.amazon.com/AWSCloudFormation/latest/TemplateReference/aws-properties-codebuild-project-source.html
[cb-sourceauth]: https://docs.aws.amazon.com/AWSCloudFormation/latest/TemplateReference/aws-properties-codebuild-project-sourceauth.html
[cb-tokens]: https://docs.aws.amazon.com/codebuild/latest/userguide/multiple-access-tokens.html
[cb-github-app]: https://docs.aws.amazon.com/codebuild/latest/userguide/connections-github-app.html
[cfn-connection]: https://docs.aws.amazon.com/AWSCloudFormation/latest/TemplateReference/aws-resource-codeconnections-connection.html
[cb-images]: https://docs.aws.amazon.com/codebuild/latest/userguide/ec2-compute-images.html
[cb-price-list]: https://pricing.us-east-1.amazonaws.com/offers/v1.0/aws/CodeBuild/current/us-east-1/index.json
[cb-pricing]: https://aws.amazon.com/codebuild/pricing/
[cb-quotas]: https://docs.aws.amazon.com/codebuild/latest/userguide/limits.html
[cb-env-vars]: https://docs.aws.amazon.com/codebuild/latest/userguide/build-env-ref-env-vars.html
[cb-service-role]: https://docs.aws.amazon.com/codebuild/latest/userguide/setting-up-service-role.html
[cb-caching-s3]: https://docs.aws.amazon.com/codebuild/latest/userguide/caching-s3.html
[cb-notifications]: https://docs.aws.amazon.com/codebuild/latest/userguide/sample-build-notifications.html
[eb-resource-policy]: https://docs.aws.amazon.com/eventbridge/latest/userguide/eb-use-resource-based.html
[cdk-bootstrap]: https://docs.aws.amazon.com/cdk/v2/guide/bootstrapping-env.html
[cfn-dynref]: https://docs.aws.amazon.com/AWSCloudFormation/latest/UserGuide/dynamic-references.html
[cfn-dynref-sm]: https://docs.aws.amazon.com/AWSCloudFormation/latest/UserGuide/dynamic-references-secretsmanager.html
