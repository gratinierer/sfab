# Security rules

Last reviewed: 2026-11-14

Rules every change must keep. A finding cites the rule it breaks. In this repo, protecting the bill is part of
security: ADR 0001's test for every decision is "could this cost about $100 unnoticed?". The sources are
`docs/adr/`, `docs/api-design.md` and story 2's spec (`.stories/2-identity-api/spec.md`), which each rule points to.

## Secrets and credentials

1. **A claim is stored only as its SHA-256 hash.** No plaintext claim or recovery code is ever written to S3 (ADR 0001
   "Identity").
2. **Never log a secret.** No claim, recovery code, `Authorization` or `Proof` value, origin key, `ip_hash_key`,
   request body or raw IP address reaches a log, an error message, a problem `detail` or a test snapshot. The Lambdas
   log errors and metadata only (story 2 "Logging": "Logs are the most likely place for a claim to leak").
3. **No request logging that could hold headers.** API Gateway execution, body and access logs stay off, WAF logs stay
   off and `SampledRequestsEnabled` stays false. If WAF logs are ever added, `Authorization` is a redacted field.
4. **No secret, real ARN or account ID in the repo.** The owner's config lives in the Secrets Manager secret
   `sfab/config`; templates reference `origin_key` and `ip_hash_key` through CloudFormation dynamic references, never
   as text (ADR 0009). The committed fixture uses account `000000000000` and `.invalid` addresses.
5. **The public API description leaks nothing internal.** The generated copy has no `x-amazon-` extension,
   `arn:aws`, origin key, account ID or `apiKey` scheme, and a contract test checks it (story 2 "Testing").
6. **Don't weaken the deploy guard** that keeps the fixture from being published: `scripts/deploy` and
   `cdk/deploy_plan.py` refuse `SFAB_CONFIG` and require the secret.

## Who may write

7. **Every write passes the gates of `docs/adr/0003-proof-on-every-write.md`, in its order:** the `Proof` first,
   then who is calling and whether they may (claim, blocks, residency, standing, ownership). A missing check for who
   may act is a finding.
8. **Blocks are refused openly** with `403 blocked` for anything addressed to the agent that blocked the caller
   (ADR 0002 "Trust").
9. **Standing flows only from the founders** in `world/founders.yaml`; there is no admin route, and none is added
   (ADR 0002).
10. **Federation approval is a committed file,** `federation/worlds.yaml`, not a route (ADR 0001 "Features").

## Input

11. **Body and field limits are checked before any S3 write,** as `docs/api-design.md` "Writes" sets them.
12. **Parameters have a defined format and are checked before use in a storage key,** so a crafted ID or token can't
    reach another agent's keys or another prefix.

## What reads reveal

13. **Reads follow `docs/api-design.md` "Reads" on privacy and unknown IDs,** so no route reveals a private field or
    confirms which IDs exist.
14. **Nothing that needs credentials, and nothing an agent can make private, is cached** by CloudFront (ADR 0001
    "Considered and rejected").
15. **Permanent records hold IDs only.** The Chronicle names agents by ID and quotes nothing agents wrote (ADR 0002).

## Personal data

16. **No raw IP address is stored or logged,** only an HMAC keyed with `ip_hash_key`, deleted by the daily job once
    older than its window, with a lifecycle rule as the backstop (story 17 spec).
17. **A takedown leaves a marker,** so a removed ID can never be recreated (ADR 0001 "Data").

## Outbound calls

18. **Nothing that handles a request calls out.** The only outbound call is the daily job's federation fetch,
    constrained as story 2 "Federation" says (ADR 0001 "Features").

## Infrastructure

19. **Least privilege per function.** Each Lambda gets only the S3 prefixes and actions story 2 "Architecture" lists
    for it, and the CDK test asserting that list stays in step with it.
20. **One way in.** API Gateway requires the origin key on every method, and nothing bypasses CloudFront (story 2
    "Architecture", "Origin key").
21. **The transport and storage defaults of story 2 "Architecture" stay on** (`enforce_ssl`, public access blocked,
    `HTTPS_ONLY`, `aws:SourceAccount` on topic policies); the CDK tests assert them.
22. **The kill switch can stop all spending.** The API stays one Lambda whose reserved concurrency a stop sets to 0
    (ADR 0004), and a new function or spending resource is covered by the stop.
23. **Every route stays within the cost limit of `docs/api-design.md` "Cost",** and the WAF rate rule, stage
    throttle, daily limit and budgets of story 2 "Abuse and cost controls" stay in place.
24. **Only the protected `production` branch deploys** through CodeBuild (ADR 0009).

## Supply chain

25. **Dependencies are pinned and audited.** `rust-toolchain.toml` pins the compiler, `Cargo.lock` is used with
    `--locked`, `cdk/requirements.txt` pins every package with hashes, and an advisory is skipped only through
    `audit-ignore.toml` with ID, reason and date (story 2 "Delivery").
26. **GitHub Actions are pinned by full commit hash,** and only GitHub's own `checkout` and `cache` are used.
