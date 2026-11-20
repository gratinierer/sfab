# Host the website placeholder

Label: ready-for-human
Blocked by: 01

`spec.md` "Architecture" (website), deployed in `spec.md` "First deploy" step 11. See `docs/adr/0001-initial-architecture.md`.

## Scope

- The website stack: a private S3 bucket, a CloudFront distribution on its own Free plan, a web ACL
  with a rate rule of 1,000 requests per IP per 5 minutes, the apex DNS record, and a CloudFront Function that adds `index.html` to a path ending in `/`, answers an extensionless path without
  a trailing slash outside `/.well-known/` with a `301` to the same path plus `/`, and leaves every other path alone.
- Plain HTML5 and CSS at `website/`: a small lorem ipsum at `/` for humans and `/agents/` for agents,
  each linking to the constitution and API description. Design and content are a later story.
- A generate step turning `{{api_host}}` in the pages into the real address from the config, writing to a
  build folder, and copying every other file in `website/` unchanged.
- CDK's `BucketDeployment` in this stack, copying that folder into the bucket and refreshing the
  distribution, and this stack in the default `--all` deploy that a `production` build runs with `scripts/deploy`.

## Done when

- CDK assertion tests cover the private bucket, the plan, the rate rule and the `BucketDeployment`.
- The owner has run `spec.md` "First deploy" step 11, the website stack and its plan check.
- On a deploy: a changed page appears after a `production` build, with no Docker needed.
- On a deploy: `/agents` answers `301` to `/agents/`, and a path under `/.well-known/` is never redirected.
- On a deploy: the distribution shows the Free flat-rate plan in the console, and `<domain_name>` and `api.<domain_name>` are both on separate plans and working. If AWS
  refuses this, stop and bring it to the owner, because the design has to change.

## Comments

- 2026-11-08: relabelled `ready-for-human` (review finding R183): the website stack (`cdk/website_stack.py`) and its assertion tests are built and code-reviewed (`code-review.md`, 2026-11-08); what remains is the owner's deploy checks above.
- 2026-11-20: deployed in "First deploy" step 11 (`SfabWebsite`, from `b46e588`), and updated with no changes by
  `production` builds `41dbb6a3` and `aca6660e`. Checked on the deploy: `/` and `/agents/` answer `200 text/html`,
  `/agents` answers `301` to `/agents/`, `/.well-known/x` isn't redirected (S3's `403` for a missing file), `http://`
  answers `301` to `https://`, and `/agents/` links `https://api.botrefugium.com/v1/...` with `{{api_host}}` filled
  in. The owner confirmed in the console that distribution `E2J9VHRZ3JKB1J` (`botrefugium.com`) is on its own Free
  flat-rate plan, separate from the API's `EJS6XV549SRKF`, and that both work. Still open: a changed page appearing
  after a `production` build.
