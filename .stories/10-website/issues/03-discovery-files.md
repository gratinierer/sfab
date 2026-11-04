# Add the files agents use to find the API

Label: ready-for-agent
Blocked by: 02

`spec.md` "Ways for agents to find the API" and "Design and content" (placeholders). `/llms.txt` repeats what the
pages say, so it follows them.

## Scope

- `{{site_host}}` in the generate step beside `{{api_host}}`, both from `domain_name`.
- `website/llms.txt`, `website/.well-known/api-catalog`, `website/robots.txt` and `website/sitemap.xml`, with
  absolute URLs, the sitemap listing `/` and `/agents/`.
- A cache behaviour for `/.well-known/api-catalog` in story 2's website distribution, with a CloudFront Function
  on viewer response that sets `Content-Type: application/linkset+json; profile="https://www.rfc-editor.org/info/rfc9727"`
  and the `Link` header with `rel="api-catalog"`, as `spec.md` says. The file deploys with the rest of the site.

## Done when

- A test of the generate step checks that no `{{` is left in any built file.
- A CDK assertion test covers the cache behaviour and its viewer-response function.
- On a deploy: `GET` and `HEAD` of `/.well-known/api-catalog` both answer with that `Content-Type` and the `Link`
  header, which settles the viewer-response row in `docs/aws-facts.md`, and `/llms.txt`, `/robots.txt` and
  `/sitemap.xml` are served.
