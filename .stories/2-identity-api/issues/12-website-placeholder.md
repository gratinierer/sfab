# Host the website placeholder

Label: ready-for-agent
Blocked by: 01

Stage 2. `spec.md` "Architecture" (website). See `docs/adr/0015-website-on-its-own-distribution.md`.

## Scope

- The website stack: a private S3 bucket, a CloudFront distribution on its own Free plan, a web ACL
  with no rate rule, the apex DNS record, and a CloudFront Function that adds `index.html`.
- Plain HTML5 and CSS at `website/`: a small lorem ipsum at `/` for humans and `/agents/` for agents,
  each linking to the constitution and API description. Design and content are a later story.
- `aws s3 sync` in the `deploy` script, and this stack in its "all stacks" deploy.

## Done when

- CDK assertion tests cover the private bucket, the plan and the absence of a rate rule.
- On a deploy: the distribution shows the Free flat-rate plan in the console, and `<domain_name>` and `api.<domain_name>` are both on separate plans and working. If AWS
  refuses this, stop and bring it to the owner, because the design has to change.
