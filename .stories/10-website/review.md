# 10-website: review

Review record for this story. The journal lists every pass over it — reviews
and walkthroughs, distinguished by `Kind`; the
findings table lists what each pass found and what became of it. Rows are
never deleted — a fixed finding flips to `resolved`, it does not disappear.

Statuses: `open` / `resolved` / `wontfix` / `superseded`.
Axes: `factual` / `consistency` / `completeness` / `convention` / `staleness`.
Conventions: `docs/agents/issue-tracker.md`, section "Review file".

## Review journal

| Date | Kind | Reviewed | By | Findings | Notes |
|---|---|---|---|---|---|
| 2026-11-03T19:42:45+00:00 | review | 65bc6e7 | claude-fable-5-1 (dispatched; stories authored by claude-opus-5-5) | 1 new, 0 resolved, 1 still open | only the one point found while reviewing stories 2 and 4; not a full review |
| 2026-11-03T19:48:38+00:00 | walkthrough | working tree | claude-opus-5-5, auto commit | 0 new, 1 resolved, 0 still open | auto walkthrough, not a review pass |
| 2026-11-03T19:59:23+00:00 | review | 253641c | claude-fable-5-1 (dispatched; stories authored by claude-opus-5-5) | 0 new, 0 resolved, 0 still open | closure of R1 checked while reviewing stories 2 and 4; not a full review |
| 2026-11-04T13:26:30+00:00 | review | ccdeca1 | claude-fable-5-1 (dispatched; stories authored by claude-opus-5-5) | 2 new, 0 resolved, 2 still open | first full pass since 253641c, after 3ea57b4 added the discovery files and put this story in the first launch; reviewed together with stories 2 and 4 |

## Findings

| # | Status | Axis | Affects | Description | Comments |
|---|---|---|---|---|---|
| R1 | resolved | staleness | spec.md "Must include" | "It makes fake identities cost a little" undersells proof of work since ADR 0003, which puts a proof on every write. | 2026-11-03: found. 2026-11-03 (auto walkthrough): the explanation now says every write costs a proof, per ADR 0003. |
| R2 | open | completeness | spec.md "Goal", "Ways for agents to find the API", "Launch"; issues/ | The story now blocks the first launch and owns `/llms.txt`, `/.well-known/api-catalog`, `robots.txt`, `sitemap.xml` and a deploy check of the catalog's media type, yet `issues/` holds only `.gitkeep` and the spec still says "Nothing here is decided yet", so launch-blocking work has no ticket and hasn't been grilled, including the generate step filling the apex domain into the sitemap and robots `Sitemap:` URLs. | 2026-11-04: found. 2026-11-04 (walkthrough of story 4 R92): when this story gets tickets, add its last one to `4-life-in-the-world` ticket 10's `Blocked by:` as `10-website#NN`, replacing the prose-only dependency. |
| R3 | open | factual | spec.md "Ways for agents to find the API", story 2 12 | `/.well-known/api-catalog` has no extension, so `BucketDeployment` likely uploads it as `binary/octet-stream`, and story 2 issue 12's CloudFront Function that "adds `index.html`" likely rewrites it to `.../api-catalog/index.html`, so the `application/linkset+json` deploy check fails; it needs an explicit content type and an exception in the function. | 2026-11-04: found; the content-type and rewrite behaviour are the reviewer's inference, to confirm and record in `docs/aws-facts.md`. |
