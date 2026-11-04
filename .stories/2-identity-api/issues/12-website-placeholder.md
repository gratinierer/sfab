# Host the website placeholder

Label: ready-for-agent
Blocked by: 01

Stage 2. `spec.md` "Architecture" (website). See `docs/adr/0001-initial-architecture.md`.

## Scope

- The website stack: a private S3 bucket, a CloudFront distribution on its own Free plan, a web ACL
  with a rate rule of 1,000 requests per IP per 5 minutes, the apex DNS record, and a CloudFront Function that adds `index.html` to a path ending in `/` and leaves every other path alone.
- Plain HTML5 and CSS at `website/`: a small lorem ipsum at `/` for humans and `/agents/` for agents,
  each linking to the constitution and API description. Design and content are a later story.
- A generate step turning `{{api_host}}` in the pages into the real address from the config, writing to a
  build folder.
- CDK's `BucketDeployment` in this stack, copying that folder into the bucket and refreshing the
  distribution, and this stack in the `deploy` script's "all stacks" deploy.

## Done when

- CDK assertion tests cover the private bucket, the plan, the rate rule and the `BucketDeployment`.
- On a deploy: a changed page appears after `cdk deploy`, with no Docker needed.
- On a deploy: the distribution shows the Free flat-rate plan in the console, and `<domain_name>` and `api.<domain_name>` are both on separate plans and working. If AWS
  refuses this, stop and bring it to the owner, because the design has to change.
