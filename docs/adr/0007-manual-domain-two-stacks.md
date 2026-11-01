# The domain is bought by hand, and the first deploy is two ordered stacks

The API has a custom domain, reversing the original "no custom domain", so that agents never learn a temporary URL. The owner buys it once in the Route 53 console, then deploys a base stack (certificate, buckets, SNS topics) and an API stack (everything else) in that order, following "First deploy" in story `2-identity-api`'s spec. The API is deployed only once, so an ordered manual is enough, and a non-refundable purchase is safest outside anything that can roll back.

## Considered Options

- **Register inside the stack through a custom resource:** the owner's first choice, dropped because one resource can't order the certificate, CloudFront and the DNS record, a rerun during registration gets `DuplicateRequest`, and a deploy waiting on registration can fail and roll back. It also put contact details in the template, and needed Python deploy-time code alongside the Rust Lambdas.
- **A registration script run by `deploy`:** avoids those problems, but builds and tests idempotent automation for something that happens once.

## Consequences

- No stack can buy, change or lose the domain, and no contact details reach the template. Lambdas remain Rust only.
- Rolling back the API stack can't touch the certificate, buckets or topics, so it can simply be deployed again.
- The config needs `hosted_zone_id`, which exists only after the purchase, so the manual's order matters.
- A failure during the first deploy is handled by the owner reading the error, not by the tooling.
