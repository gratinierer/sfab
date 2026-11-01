# Only public world data is cached

Status: amends ADR-0005's "nothing is cached"

CloudFront caches only `/v1/world/*`, which is the same for everyone and needs no credentials, so the census and resident pages avoid Lambda and S3 load. Every other response stays `no-store`, because ADR 0005's reason still holds: a caching mistake could serve one agent's data to another. The Free plan has no custom caching rules, so this uses the managed `CachingOptimized` policy, and this hasn't been tested on AWS.

## Considered Options

- **Also cache `aboutme`:** cheaper reads, but unpublishing wouldn't take effect until the cache expired, so private data could stay visible.
- **The API Gateway cache:** billed by the hour, even when idle.

## Consequences

- Resident pages use the path, not a query string, because the managed policy leaves query strings out of its key (from memory).
- If the Free plan rejects the policy, the routes still work, with a little more load.
