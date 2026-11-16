# Measure whether rejected requests are billed

Label: ready-for-human
Blocked by:

AWS's current docs don't say whether API Gateway bills REST API requests that it rejects, and
unofficial sources conflict. Its
[former pricing page](https://web.archive.org/web/20211202141251/https://docs.aws.amazon.com/apigateway/latest/developerguide/api-gateway-pricing.html),
since removed, said authentication failures, missing or invalid API keys and throttled requests
aren't charged. This test confirms that still holds, and covers a disabled endpoint, which the
page didn't mention. This no longer affects floods that CloudFront's WAF blocks, which AWS says are free on
the flat-rate plan. It only affects traffic that gets past WAF, or goes straight to `execute-api`.
Filed from review finding R17. It needs the owner's AWS account.

## Test

On one UTC day with no other traffic, including smoke runs, send these counts. They differ so
that any billed total can only mean one combination of kinds:

1. 1,000 requests straight to `execute-api` without the origin key.
2. 2,000 requests straight to `execute-api` while the default endpoint is disabled.
3. 4,000 requests through to API Gateway, so they get throttled.

Have the client tally the status codes it receives. Requests in kind 3 that succeed are billed
anyway, so subtract them from the total. A day or two later, read the day's request count in
Cost Explorer. This costs about 3 cents. The
WAF limit of 100 requests per IP per 5 minutes applies to the third kind, so run it from several
IPs or relax the rule briefly, and restore it afterwards. Also confirm that WAF-blocked requests
add no charge, and which kinds raise API Gateway's `Count`, which the daily limit relies on. The test
sends far fewer calls than the limit, so it can't trip the stop by accident.

## Running it before the deploy

Run the API Gateway half on three throwaway REST APIs instead of the deployed stacks. They need no Lambda
and no CodeBuild, so they work while the account waits on its quotas (issue 03). Each kind of request gets
its own API, so each API's `Count` shows on its own whether that kind is counted. Billing is per account, so
the totals in the table still apply. WAF isn't involved here, so no IP limit applies. Whether WAF-blocked
requests cost anything is checked in issue 10's WAF check instead.

Run everything in `us-east-1` with `aws login` credentials, on one UTC day, and send nothing else to API
Gateway that day.

```bash
export AWS_REGION=us-east-1

# Makes a REST API with GET / on a mock integration (no Lambda), deployed to stage t. Prints its ID.
mk() {  # name [--api-key-required]
  local id root
  id=$(aws apigateway create-rest-api --name "$1" --endpoint-configuration types=REGIONAL --query id --output text)
  root=$(aws apigateway get-resources --rest-api-id "$id" --query 'items[0].id' --output text)
  aws apigateway put-method --rest-api-id "$id" --resource-id "$root" --http-method GET \
    --authorization-type NONE $2 >/dev/null
  aws apigateway put-integration --rest-api-id "$id" --resource-id "$root" --http-method GET --type MOCK \
    --request-templates '{"application/json":"{\"statusCode\":200}"}' >/dev/null
  aws apigateway put-method-response --rest-api-id "$id" --resource-id "$root" --http-method GET \
    --status-code 200 >/dev/null
  aws apigateway put-integration-response --rest-api-id "$id" --resource-id "$root" --http-method GET \
    --status-code 200 --response-templates '{"application/json":"{}"}' >/dev/null
  aws apigateway create-deployment --rest-api-id "$id" --stage-name t >/dev/null
  echo "$id"
}
url() { echo "https://$1.execute-api.$AWS_REGION.amazonaws.com/t/"; }
# Sends n requests, 5 at a time, and tallies the status codes. At 20 at a time, the devcontainer's DNS failed
# about a sixth of the lookups (2026-11-16).
hit() { seq "$2" | xargs -P 5 -I{} curl -s -o /dev/null -w '%{http_code} %{errormsg}\n' "$1" | sort | uniq -c; }

KEY=$(mk sfab-billing-key --api-key-required)
OFF=$(mk sfab-billing-disabled)
THR=$(mk sfab-billing-throttled)

# Kind 2: disable the default endpoint. A REST API needs a new deployment for it to take effect.
aws apigateway update-rest-api --rest-api-id "$OFF" \
  --patch-operations op=replace,path=/disableExecuteApiEndpoint,value=true >/dev/null
aws apigateway create-deployment --rest-api-id "$OFF" --stage-name t >/dev/null

# Kind 3: throttle the stage to 1 request per second, with a burst of 1.
aws apigateway update-stage --rest-api-id "$THR" --stage-name t --patch-operations \
  'op=replace,path=/*/*/throttling/rateLimit,value=1' 'op=replace,path=/*/*/throttling/burstLimit,value=1' >/dev/null

echo "$KEY $OFF $THR"   # write these down
```

Wait a minute for the changes to spread. Then send the requests and keep the three tallies:

```bash
hit "$(url "$KEY")" 1000   # expect 1000 × 403 (missing API key)
hit "$(url "$OFF")" 2000   # expect 2000 × 403 (endpoint disabled)
hit "$(url "$THR")" 4000   # expect mostly 429, plus some 200s, which are billed anyway
```

If a tally shows anything unexpected, such as `200`s from the disabled API, stop. The setup didn't take
effect, so that day's numbers are mixed.

Delete the APIs afterwards. Their metrics stay readable after deletion:

```bash
for id in $KEY $OFF $THR; do aws apigateway delete-rest-api --rest-api-id "$id"; sleep 31; done
```

**The next day, read `Count`** for each API over the test day, with the statistic the kill switch uses:

```bash
for name in sfab-billing-key sfab-billing-disabled sfab-billing-throttled; do
  echo "$name"
  aws cloudwatch get-metric-statistics --namespace AWS/ApiGateway --metric-name Count \
    --dimensions Name=ApiName,Value="$name" --period 86400 --statistics SampleCount \
    --start-time <day>T00:00:00Z --end-time <day+1>T00:00:00Z --query 'Datapoints[].SampleCount'
done
```

A day or two later, **read the billed requests** in the Cost Explorer console, which is free (its API costs
$0.01 per call): daily, filtered to the API Gateway service, grouped by usage type, showing **usage
quantity**, not cost. The free tier or credits can make the cost $0 even when requests are counted.
Subtract kind 3's `200`s from the `ApiGatewayRequest` quantity, and the rest is some combination of
1,000, 2,000 and kind 3's `429`s, which only one combination can add up to.

Record the three tallies, the three `Count`s and the billed quantity under `## Comments`.

## Decision this feeds

| Result | What it means | What happens next |
|---|---|---|
| `Count` includes rejected requests | The daily limit sees direct floods | The design stands |
| `Count` misses them, and they aren't billed | Direct floods aren't counted, but cost nothing | The design stands |
| `Count` misses them, and they are billed | A direct flood costs money while the limit counts nothing, and only the budgets see it | Stop and bring it to the owner |
| Requests to a disabled endpoint are billed | The accepted risk in `spec.md` "What it can cost" is real | Bring it to the owner, and the runbook's "delete the API" step applies |

Record the result in `spec.md` "Open questions", and in `docs/aws-facts.md`.

## Done when

- The counts are recorded, and `spec.md` reflects what they mean for "What it can cost".

## Comments

- 2026-11-16: unblocked from 01 and 07 by the owner's choice, while the account waits on its Lambda and CodeBuild
  quotas. The API Gateway half now runs on throwaway APIs ("Running it before the deploy"), and the WAF half moves
  to issue 10's WAF check.
- 2026-11-16: the owner had the agent run setup, sending and deletion from the devcontainer, between 06:16 and
  06:38 UTC. The APIs were `rgweoh1a8h` (key), `pgbdrpm0if` (disabled) and `plgnwhf7tl` (throttled), and all
  three were deleted at about 06:38. Setup needed one retry, because `CreateDeployment` was throttled
  (`TooManyRequestsException`). Tallies, with `000` meaning curl failed and sent nothing:
  - Key, 1,040 sent: 905 × `403`, 135 × `000`. The first 1,000 were sent with no error message logged. The
    second batch's errors show these were DNS failures. 40 of the 905 were a diagnostic re-run.
  - Disabled, 2,000 sent: 1,667 × `403`, 333 × `000` "Could not resolve host".
  - Throttled, 4,000 sent: 520 × `200`, 2,759 × `429`, 721 × `000` "Could not resolve host".
  - Reached AWS: 905 + 1,667 + 3,279 = 5,851, all on 2026-11-16, with no other API Gateway traffic that day.
    The `000`s never resolved the host, so they can't have reached AWS. Billed only the `200`s means 520.
    Each rejected kind adds 905, 1,667 or 2,759 on top. Every combination gives a different total.
  - Still to do: `Count` for 2026-11-16 from 2026-11-17, and Cost Explorer's `ApiGatewayRequest` usage quantity
    for 2026-11-16 once it appears.
