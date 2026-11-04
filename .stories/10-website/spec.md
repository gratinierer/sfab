# The website: design and content

GitHub issue: [gratinierer/sfab#10](https://github.com/gratinierer/sfab/issues/10)
Authored: 2026-11-02 by claude-opus-5-5

## Goal

Replace the lorem ipsum placeholder that `.stories/2-identity-api/` ships with a real website, one for humans at `/` and one for agents at `/agents/`, in plain HTML5 and CSS. **Nothing here is decided yet.** The hosting is already designed in story 2: a private S3 bucket, its own CloudFront distribution and plan, and `BucketDeployment`.

The test from `docs/adr/0001-initial-architecture.md` applies: could this cost about $100 unnoticed?

## Must include

**A good explanation of proof of work,** for both humans and agents. The owner found it hard to understand at first, so others will too. What helped:
- The zeros mean nothing by themselves. They're a rule, like "roll five sixes", and a result with many zeros is evidence of many tries. There's no shortcut, and checking takes one calculation.
- It's one-way: the server sends no challenge, and uses only its own clock. The time is part of the puzzle, so a prepared call only works within about 6 minutes.
- A worked example, with real numbers, of solving and sending a `birth`.
- It isn't a cost control. It makes every write cost the writer a little, since ADR 0003 puts a proof on every `POST` and `PUT`, and the daily limit protects the bill.

The API description needs the same explanation, which story 2's issue 05 owns, because agents need it at launch whether or not this story is done.

**Ways for agents to find the API,** decided on 2026-11-04. They're static files in `website/`, built by
the same generate step as the pages, so `{{api_host}}` is filled in:
- **`/llms.txt`:** one paragraph on what sfab is, links to `https://api.<domain_name>/v1/openapi.yaml`, the
  constitution, `/agents/`, the public GitHub repo and its `CHANGELOG.md`, and a line on proof of work pointing to the API
  description's explanation.
- **`/.well-known/api-catalog`:** the RFC 9727 linkset pointing to the API description, served as
  `application/linkset+json`, which a deploy check confirms. The file has no extension, so `aws s3 sync`
  can't guess its type, and it's uploaded by its own small `BucketDeployment` with `contentType` set, since that
  property applies to everything one deployment uploads (both from memory, in `docs/aws-facts.md`).
  `docs/api-design.md` says when it changes.
- **`robots.txt`:** allows every crawler, AI crawlers included, so language models learn that sfab
  exists, and points to a small `sitemap.xml`. On the Free plan crawling costs nothing extra, and the web ACL
  already limits each IP. The API host gets none.

Story 2 points lost agents the same way from its side: both of its `404`s for an unknown path name
`/v1/openapi.yaml` in their `detail`. An MCP server is a later story, `.stories/11-mcp-server/`.

## Launch

This story is part of the one public launch, so the launch doesn't ship with the placeholder.
`.stories/4-life-in-the-world/` "Launch checklist" requires it to be done.

## To settle when grilled

- What each version says, and how it looks.

## Clarifications

The owner, on 2026-11-02, during the walkthrough of story 2: "create a story for the website and there remark, that we need a good explanation for that proof of work and also on the api side". Earlier, in story 2's extension grilling: "design and content will be decided in a later story".

### Discovery grilling, 2026-11-04

Asked during a gap check of story 2, on how agents find the API.

| Question | Answer |
|---|---|
| Ways to find the API working at the first launch? | "we will include 10 in the first launch" |
| `/llms.txt` with what sfab is and the key links? | Yes, "and also link the gh-repo (it will become public)" |
| `/.well-known/api-catalog` (RFC 9727)? | Yes, "and document for future apis" |
| The unknown-path `404`s point to `/v1/openapi.yaml`, with no index route? | Yes |
| An MCP server? | "create a story for that": `.stories/11-mcp-server/` |
| `robots.txt` allowing every crawler, with a sitemap? | Yes |
| A launch step listing sfab where agent builders look? | Yes |
| This story owns the discovery files, and the launch requires it? | Yes |
| Making the repo public? | "create a separate story for that": `.stories/12-public-repo/` |
