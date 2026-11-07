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
| A CloudFront plan's `ResourceArns` must hold one distribution and one WAF web ACL, and may add a hosted zone; `PlanTier` is `FREE` and `UsageLevel` `DEFAULT` | docs | [plan-cfn] | 2026-11-07 |
| Cancelling or downgrading a paid plan takes effect at the end of the month, while a downgrade's features go at once | docs | [plan-api] | 2026-11-01 |
| Free Tier accounts can't use flat-rate plans | docs | [plans] | 2026-10-30 |
| Each plan covers one distribution with up to one apex domain, and an account can have 3 Free plans | docs | [plans] | 2026-11-01 |
| Free allows 5 cache behaviors | docs | [plans] | 2026-11-01 |
| A plan can include a hosted zone, and past its allowance AWS might move the zone back to pay-as-you-go | docs | [plan-cfn], [plans] | 2026-11-01 |
| Free covers the zone's fee, 50 records, unlimited alias queries and 1 million other queries a month | docs | [plans], the reviewer's reading | 2026-11-01 |
| Two plans can share one apex domain, such as `<domain>` and `api.<domain>` | memory | Not stated by AWS; the first deploy proves it | 2026-11-01 |
| `CachingOptimized` leaves query strings out of its cache key, with a 24-hour default TTL | docs | [cache-policies], the reviewer's reading | 2026-11-01 |
| A private S3 origin doesn't add `index.html` to a folder path | memory | | 2026-11-01 |
| `BucketDeployment` sets each object's content type by guessing from its extension, so an extensionless file gets S3's default `binary/octet-stream` | memory | Why story `10-website` sets the catalog's type in CloudFront | 2026-11-04 |
| `BucketDeployment`'s `contentType` applies to every object that deployment uploads | docs | CDK s3-deployment README, the reviewer's reading | 2026-11-04 |
| `BucketDeployment` prunes by default, deleting files not in its source, and only its own deployment-level `exclude` protects one, so two deployments on one bucket delete each other's files | docs | CDK s3-deployment README ("Prune", "Exclude and Include Filters"), the reviewer's reading; why story `10-website` uses one | 2026-11-04 |
| A CloudFront Function on viewer response can set `Content-Type` and add a `Link` header | memory | Ticket 03 of story `10-website` proves it on a deploy | 2026-11-04 |
| `CachingDisabled` is accepted on Free | memory | Not stated by AWS; issue 01 of story `2-identity-api` proves it | 2026-11-01 |
| `HTTPS_ONLY` answers `http://` with `403` | docs | [https-viewers], the reviewer's reading; issue 01 of story `2-identity-api` proves it | 2026-11-01 |
| Port 80 can't be closed on a distribution | memory | | 2026-11-01 |
| A new account may need Support verification before creating CloudFront resources | forum | re:Post search summaries | 2026-11-01 |

## WAF

| Fact | Status | Source | Checked |
|---|---|---|---|
| Rate rules take up to several minutes to react, usually under 30 seconds | docs | [waf-rate] | 2026-11-01 |
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
| `$context.error.messageString` arrives already JSON-quoted, so a gateway response template uses it bare, as `"detail": $context.error.messageString` | memory | Every template in `openapi/sfab.yaml` relies on it; issue 01 of story `2-identity-api` checks it on a deploy | 2026-11-04 |
| `$context.error.validationErrorString` can hold double quotes, such as `(["name"])`, so it can't sit inside a quoted JSON string in a gateway response template, and whether `$util.escapeJavaScript` works there is unknown | memory | Why `BAD_REQUEST_BODY` in `openapi/sfab.yaml` uses `messageString`; issue 01 of story `2-identity-api` can check both on a deploy | 2026-11-04 |
| A wrong or missing API key on a key-required method raises the `INVALID_API_KEY` gateway response, a `403`, rather than `DEFAULT_4XX` alone | memory | | 2026-11-03 |
| An unknown route or method returns `MISSING_AUTHENTICATION_TOKEN` as `403`, and the `413` can't be customised | docs | [response-types] | 2026-11-01 |
| OpenAPI import accepts `$default` as a `requestBody` content type, giving a model that validates every content type, beside `application/json` | memory | Why `openapi/sfab.yaml` lists both; issue 01 of story `2-identity-api` proves it on import | 2026-11-04 |
| OpenAPI import doesn't resolve a `$ref` inside `x-amazon-apigateway-integration`, so `openapi/sfab.yaml` repeats it with a YAML anchor instead | memory | Issue 01 of story `2-identity-api` checks it on import | 2026-11-04 |
| REST requests cost $3.50 per million | docs | AWS's pricing page | 2026-11-01 |
| A CloudFormation `RestApi` with both `Body` and `Name` is named `Name`, not the OpenAPI `info.title`, so CDK's `metricCount()` names the right `ApiName` | memory | AWS doesn't say which wins; the kill switch reads the name with `GetRestApi`, and issue 10's alarm check confirms the 50,000-call warning | 2026-11-07 |
| `disableExecuteApiEndpoint` affects all stages, returns `403`, and takes effect only after a stage update | memory | spec.md "Kill switch" says AWS documents it; not rechecked. CloudFormation's `RestApi` page only says it disables the default endpoint | 2026-11-07 |
| `CreateDeployment` with a `stageName` points that existing stage at the new deployment | memory | The kill switch's stop step and the restart list rely on it; issue 10's drill proves it | 2026-11-07 |
| Failed authentication, missing or invalid API keys and throttled requests aren't charged | docs | [old-pricing], a 2021 archive of a removed page, so possibly out of date; issue 02 of story `2-identity-api` confirms it | 2026-11-01 |

## CDK

| Fact | Status | Source | Checked |
|---|---|---|---|
| `RestApiOrigin` defaults its origin path to the stage name | docs | [rest-api-origin], v2.275.0, line 44 | 2026-11-01 |
| A distribution defaults to `ALLOW_GET_HEAD`, and to `ALLOW_ALL` for the viewer protocol unless a flag makes it redirect | docs | [cdk-behavior] | 2026-11-01 |
| `BucketDeployment` copies a folder during `cdk deploy`, can refresh a distribution, and needs no Docker | memory | Issue 12 of story `2-identity-api` proves it | 2026-11-01 |
| `SpecRestApi` adds no Lambda invoke permission, and creates no method resources | docs | Earlier review, R12 and R61 | 2026-10-30 |
| `cdk bootstrap aws://<account>/<region>` still synthesizes the app when run beside a `cdk.json`, though it names the target, so run it where there's none | docs | CDK CLI 2.1147.0 source, `defineEnvironments` in `lib/cli/cdk-toolkit.js`; corrected from "doesn't need the app" | 2026-11-04 |

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
| `PutObject` with `If-None-Match: *` fails with `412` when the key exists, and with `409` when a concurrent conditional write wins | memory | `birth`'s replay check, in `lambda/identity`; issue 09's smoke test exercises it | 2026-11-05 |
| `PutObject` with `If-Match` fails with `412` on a changed ETag and `404` on a deleted key, and `GetObject` returns the ETag that `If-Match` takes | memory | `reclaim`, in `lambda/identity`, and `lambda/core/src/s3.rs` | 2026-11-05 |
| S3 rounds a lifecycle expiry up to the next midnight UTC, and removes expired objects some time later, so a route must filter by date itself | docs | [s3-expire], the reviewer's reading | 2026-11-01 |
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
| API Gateway `Count` is "the total number API requests", without saying if rejected or throttled requests count, and its documented statistic is `SampleCount`, which the kill switch and the 50,000-call warning use | docs | [apigw-metrics]; issue 02 of story `2-identity-api` tests it | 2026-11-07 |
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
| Each SNS email has an unsubscribe link needing no login, and the subscription attribute `ConfirmationWasAuthenticated` shows how it was confirmed | memory | Issue 01 of story `2-identity-api` confirms it | 2026-11-01 |
| Domain charges appear under the service name "Amazon Registrar" | memory | The first purchase's bill decides | 2026-11-01 |
| Updating a budget replaces all of it, so a filter must be sent again | memory | | 2026-11-01 |
| `DescribeBudget` returns `CalculatedSpend.ActualSpend` and `BudgetLimit` as strings, and needs `budgets:ViewBudget` on `arn:aws:budgets::<account>:budget/<name>` | memory | The kill switch's daily spend check | 2026-11-07 |
| WAF's `UpdateWebACL` replaces the whole web ACL and needs the `LockToken` from `GetWebACL`, so a change of default action sends the rules back too | memory | The kill switch and `docs/restart-after-a-stop.md` | 2026-11-07 |
| A web ACL's CloudFormation `Ref` is `name\|id\|scope` | memory | The kill switch splits it | 2026-11-07 |
| SNS invokes a subscribed Lambda asynchronously, and EventBridge schedules too, so Lambda retries a failed run twice by default | memory | Why the kill switch keeps the default retries | 2026-11-07 |
| An asynchronous invocation throttled for lack of concurrency, such as at reserved concurrency 0, isn't a function error: Lambda keeps it queued and retries it for up to the maximum event age, 6 hours by default, whatever `MaximumRetryAttempts` says | memory | Why a stop at midnight can delay the daily world job's run rather than skip it; `cdk/world_job.py` and `docs/restart-after-a-stop.md` step 3 | 2026-11-07 |
| `GetObject` on a missing key returns `403` when `s3:ListBucket` is allowed only under a condition on `s3:prefix`, because the request has no prefix to match | memory | Why the daily world job lists `world/` first and reads only keys it listed (`lambda/world/census`) | 2026-11-07 |
| A hosted zone costs about $0.50 per month without a plan | docs | The Route 53 pricing page, the reviewer's reading | 2026-11-01 |
| Route 53 bills every query answered except alias records, at $0.40 per million up to a billion | docs | The Route 53 pricing page | 2026-11-01 |
| S3 and Lambda prices used in cost tables | memory | | 2026-11-01 |

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
