# A simple first line of defence, tested by "could this cost $1000 unnoticed?"

This project isn't critical, and a bad day can be learned from, while a surprise bill can't. So every design and review decision is tested against one question: **could this cost about $1000 unnoticed?** Machinery for availability and automatic recovery doesn't pass that test unless it's cheap to keep.

Cost protection is one daily limit, and the budget behind it. Every 5 minutes, the kill switch function reads API Gateway's `Count` since midnight UTC, and over 100,000 it runs the full stop. The budget, at $35 and excluding domain charges, triggers the same stop. Restarting is always by hand. It doesn't matter whether the limit is reached after a minute or never.

## Considered Options

- **A CloudWatch alarm with automatic resume (ADR 0016):** an alarm acts only when its state changes, a rolling window would make the API flap under attack, and the missing-data setting can be ignored. The owner would rather react by hand.
- **The Lambda's `Invocations`:** can't be tripped by direct requests, but also can't see them, so a billed flood of rejected requests would go uncounted.
- **Budget alerts alone:** they can lag by a day or more.

## What this removed

These served availability or tidiness, not the bill, and each is easy to add back if production shows a need: the automatic resume; the storage ceiling, because the limit already bounds stored data to about $2 per month of growth; the per-route throttle table, for one stage default; Parameter Store, so settings live in the CDK code; the monthly budget reset; the build stamp; `reclaim`'s retry window (ADR 0004), so a lost `reclaim` response loses that identity; and the CloudFront cache for world routes (ADR 0010), while the daily snapshot stays.

The origin key stays. It doesn't protect the bill, but it costs nothing and keeps one way in, which shows the test isn't a reason to delete everything that doesn't pass it.

## Consequences

- About 35 IPs can reach the limit in a day, and someone with the `execute-api` URL may trip it from one machine. The API stays down until the owner responds, which is accepted.
- The risks that aren't API calls need their own defences: MFA and no long-lived keys, a check that the flat-rate plans are attached, Cost Anomaly Detection, and alerts that reach the owner's phone.
- If someone floods the disabled `execute-api` endpoint during a stop, and AWS bills that, the owner has to delete the API by hand. `issues/02-measure-rejected-request-billing.md` in story `2-identity-api` measures it.
- None of this has been tested on AWS.
