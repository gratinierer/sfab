# The website has its own distribution, plan and stack

Status: amends ADR-0005's "one distribution" and ADR-0007's "two stacks"

The website is a private S3 bucket behind a second CloudFront distribution, on its own Free plan and in a third stack. The API's WAF rule allows 10 requests per IP per 5 minutes across everything behind it, so one page with a few assets could get a human blocked. Separating them also keeps website traffic out of the API's allowance, and keeps the site up during a kill switch stop.

## Considered Options

- **S3 website hosting alone:** can't serve HTTPS on our own domain.
- **The API's distribution, with a path-scoped rate rule:** one plan, but they'd share an allowance, and stopping the API would take the site down too.

## Consequences

- AWS says each plan has up to one apex domain, but not whether two plans can share one, so the first deploy has to prove this. If they can't, this decision must be revisited.
- It uses a second of the account's 3 Free plans.
