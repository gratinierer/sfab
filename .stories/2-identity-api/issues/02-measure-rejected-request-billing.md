# Measure whether rejected requests are billed

Label: ready-for-human
Blocked by: 01

`Blocked by` is provisional until the story is sliced: this also needs the WAF and the kill
switch, which no ticket builds yet.

AWS doesn't say whether API Gateway bills REST API requests that are throttled, or
rejected with `403` while the default endpoint is disabled. Unofficial sources conflict.
Nor is it stated whether WAF keeps charging about $0.60 per million during a stop. Filed
from review finding R17. It needs the owner's AWS account.

## Test

1. After the first deploy, send a known number of requests, about 1,000, to the disabled
   endpoint.
2. Send another known number that get throttled.
3. A day or two later, compare the request counts in Cost Explorer, for both API Gateway
   and WAF.

This costs under a cent. The WAF limit of 10 requests per IP per 5 minutes will interfere,
so run from several IPs or relax the rule briefly, and restore it afterwards.

## Decision this feeds

If disabled-endpoint requests are billed, also flip the WAF default action to Block during
a stop, which costs less per request, provided WAF evaluates before the endpoint rejects
the request. Otherwise leave the kill switch as it is.

Record the result in `spec.md` "Open questions".

## Done when

- The counts are recorded, and the kill switch decision above is made and reflected in
  `spec.md`.
