# Measure whether rejected requests are billed

Label: ready-for-human
Blocked by: 01, 07

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
