# CloudFront on a flat-rate plan is the front door

Agents reach the API through a CloudFront distribution on the Free flat-rate plan, with WAF attached to it, in front of a regional API Gateway. AWS states that flat-rate plans have no overage charges, and that requests blocked by WAF never count against the allowance, so a flood from few IPs, and every request during a kill switch stop, costs nothing per request. Without this, a volumetric flood could cost hundreds of dollars a day with nothing to cap it, which contradicted the goal of protecting the bill. It also gives agents a URL that survives replacing the API behind it.

## Considered Options

- **API Gateway directly, with a regional WAF:** simpler, but pays about $6 per month plus per-request fees for WAF, and possibly API Gateway fees for rejected requests, with no cap.
- **AWS Shield Advanced:** includes DDoS cost protection, but costs far more than the risk at this stage.
- **API Gateway's own edge-optimized endpoint:** uses a CloudFront distribution AWS manages, so it can't be on our plan or be configured.

## Consequences

- API Gateway's `execute-api` URL stays public, so it requires an origin key that only CloudFront adds. This is an origin check, not agent authentication, so ADR 0002 stands.
- The Free plan has no CloudFront or WAF logs, and caps WAF at 5 rules. Pro at $15 per month is the first upgrade.
- Nothing is cached, because a caching mistake could serve one agent's identity to another. Responses also send `Cache-Control: no-store`.
- Traffic spread over enough IPs to stay under the per-IP limit still reaches API Gateway and costs money. The owner accepts this.
