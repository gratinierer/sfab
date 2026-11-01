# Measure whether rejected requests are billed

Label: ready-for-human
Blocked by: 01

`Blocked by` is provisional until the story is sliced: this also needs the throttles and the
kill switch, which no ticket builds yet.

AWS's current docs don't say whether API Gateway bills REST API requests that it rejects, and
unofficial sources conflict. Its
[former pricing page](https://web.archive.org/web/20211202141251/https://docs.aws.amazon.com/apigateway/latest/developerguide/api-gateway-pricing.html),
since removed, said authentication failures, missing or invalid API keys and throttled requests
aren't charged. This test confirms that still holds, and covers a disabled endpoint, which the
page didn't mention. This no longer affects floods that CloudFront's WAF blocks, which AWS says are free on
the flat-rate plan. It only affects traffic that gets past WAF, or goes straight to `execute-api`.
Filed from review finding R17. It needs the owner's AWS account.

## Test

Send a known number of requests, about 1,000 each, of these kinds:

1. Straight to `execute-api` without the origin key.
2. Straight to `execute-api` while the default endpoint is disabled.
3. Through to API Gateway, so they get throttled.

A day or two later, compare the request counts in Cost Explorer. This costs under a cent. The
WAF limit of 10 requests per IP per 5 minutes applies to the third kind, so run it from several
IPs or relax the rule briefly, and restore it afterwards. Also confirm that WAF-blocked requests
add no charge.

## Decision this feeds

If direct requests are billed, a leaked origin URL has a real cost. The runbook's last resort
then matters, which is replacing the API behind CloudFront. Otherwise the origin key is enough.

Record the result in `spec.md` "Open questions".

## Done when

- The counts are recorded, and `spec.md` reflects what they mean for "Estimated worst cases".
