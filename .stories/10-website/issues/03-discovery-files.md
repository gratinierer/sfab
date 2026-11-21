# Add the files agents use to find the API

Label: ready-for-agent
Blocked by: 02

`spec.md` "Ways for agents to find the API" and "Design and content" (placeholders). `/llms.txt` repeats what the
pages say, so it follows them.

## Scope

- `{{site_host}}` in the generate step beside `{{api_host}}`, both from `domain_name`.
- `website/llms.txt`, `website/.well-known/api-catalog`, `website/robots.txt` and `website/sitemap.xml`, with
  absolute URLs, the sitemap listing `/` and `/agents/`.
- A cache behaviour for `/.well-known/api-catalog` in story 2's website distribution, with the managed
  `CachingOptimized` policy and a CloudFront Function
  on viewer response that sets `Content-Type: application/linkset+json; profile="https://www.rfc-editor.org/info/rfc9727"`
  and the `Link` header with `rel="api-catalog"`, its URL written into the function by the CDK from `domain_name`,
  as `spec.md` says. The file deploys with the rest of the site.

## Done when

- A test of the generate step checks that no `{{` is left in any built text file (`.html`, `.txt`, `.xml` and
  `api-catalog`), and that every other file, such as `svg`, `css`, `woff2` and a committed
  `.well-known/sfab-federation/<token>`, is copied unchanged.
- A CDK assertion test covers the cache behaviour, its `CachingOptimized` policy and its viewer-response function,
  including the `Link` value built from the fixture's `domain_name`.
- On a deploy: `GET` and `HEAD` of `/.well-known/api-catalog` both answer with that `Content-Type` and the `Link`
  header, which settles the viewer-response row in `docs/aws-facts.md`, and `/llms.txt`, `/robots.txt` and
  `/sitemap.xml` are served.
- On a deploy: every link on both pages and in `/llms.txt` resolves, except those to the GitHub repo, which stays
  private until `4-life-in-the-world`'s launch step 8 makes it public; once it is, they must resolve too.
