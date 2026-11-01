# A call limit pauses the API and resumes by itself

Status: Superseded by ADR-0017

Budget data can lag by a day or more, so on its own the budget stop lets a flood run up about $24 before it fires. A CloudWatch alarm on the API Lambda's `Invocations` trips at 100,000 calls per 24 hours, flips WAF to Block within minutes, and reopens when the alarm clears, which cuts a flood's cost to about $1-2 a day. The budget stop stays, stays manual and always wins, because calls vary in price and stored data costs money too. This is the one exception to "restarting is never automatic", because the daily limit caps what each day can cost. These figures are estimates, and none of this has been tested.

## Considered Options

- **A counter in S3:** S3 can't increment safely, and each write costs more than the call it counts.
- **A counter in DynamoDB:** adds a write and a service to every call, and sees nothing rejected before the Lambda.
- **API Gateway's `Count` metric:** free, but anyone finding the `execute-api` URL could trip the limit with rejected calls, with no WAF limit on them, and flipping WAF couldn't bring the count down.
- **A full stop with a manual restart:** simpler, but about 35 IPs could keep the API down until the owner responds, every time.
- **Moving to a fixed-price Hetzner server:** a fixed bill and a simpler design, using SQLite. Dropped because this limit removes most of that advantage, AWS stays cheaper at low traffic, and a server needs patching, backups and monitoring. It's worth another look if traffic grows past about 2 million requests a month.

## Consequences

- About 35 IPs can pause the API daily, which the owner accepts, because the bill comes first.
- Rejected and throttled calls aren't counted, so if AWS bills them, only the budget stop protects against them. `issues/02-measure-rejected-request-billing.md` in story `2-identity-api` tests this.
- A paused Lambda reports no data, so the alarm must treat missing data as not breaching, or the API would never resume.
