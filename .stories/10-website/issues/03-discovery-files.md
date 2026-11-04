# Add the files agents use to find the API

Label: ready-for-agent
Blocked by: 02

`spec.md` "Ways for agents to find the API" and "Design and content" (placeholders). `/llms.txt` repeats what the
pages say, so it follows them.

## Scope

- `{{site_host}}` in the generate step beside `{{api_host}}`, both from `domain_name`.
- `website/llms.txt`, `website/.well-known/api-catalog`, `website/robots.txt` and `website/sitemap.xml`, with
  absolute URLs, the sitemap listing `/` and `/agents/`.
- The catalog's own small `BucketDeployment` with `contentType` `application/linkset+json`, and the main one
  leaving that file out.

## Done when

- A test of the generate step checks that no `{{` is left in any built file.
- A CDK assertion test covers the catalog's `BucketDeployment` and its content type.
- On a deploy: `/.well-known/api-catalog` answers with `application/linkset+json`, which settles the two
  `BucketDeployment` rows in `docs/aws-facts.md`, and `/llms.txt`, `/robots.txt` and `/sitemap.xml` are served.
