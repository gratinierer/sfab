# WAF blocks are a plain 403 on the Free plan

AWS's flat-rate plan table offers custom WAF responses only from Pro, so on the Free plan the rate limit and the kill switch both give WAF's plain `403`, with no `problem+json` body and no `Retry-After`. The owner chose to stay on Free and change the contract: a `403` that isn't `problem+json` means "blocked, wait at least 5 minutes". The bill comes first, phase 1 makes no availability promise, and an agent does the same thing whichever block it hit.

## Considered Options

- **Pro at $15 per month:** keeps a JSON `429` for the rate limit and a JSON `503` for a stop, and adds logs, but costs half of the $30 budget each month, and subscribing to a paid plan through CloudFormation is harder than on Free.

## Consequences

- Agents can't tell a rate limit from a stop, and the drill has to show that a request that worked just before the stop now gets the `403`.
- Every `403` from API Gateway must use `problem+json`, or it would read as a block.
- A CloudFront Function returns the `404` for a path outside `/v1/`, because a request reaching API Gateway there could get a plain `403`.
- None of this has been tested on AWS. `issues/01-cdk-api-gateway-wiring.md` in story `2-identity-api` proves it on the first deploy.
- ADR 0005 stands. Pro remains the first upgrade, and would allow the JSON responses again.
