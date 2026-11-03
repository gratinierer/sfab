# API design rules

Every route, in every story, follows these rules. A story's route table must show that each route complies, and a review checks each rule against each route. The reasons are in `docs/adr/0001-initial-architecture.md` and `docs/adr/0002-life-in-the-world.md`, and the status codes and `type` values are listed once, in `.stories/2-identity-api/spec.md` "Wire contract". When a new route needs a new rule or a new `type`, add it there and here, so this file stays the one checklist.

## Code

A route's rules live in its institution's crate, and the router only maps the route to it, as `docs/adr/0004-rules-live-in-crates.md` says. A new story adds a crate, not a Lambda, and each crate exports its takedown function.

## Paths

- `/v1/<area>/...`, lowercase. Things you read are nouns.
- `self` means the caller. `{id}` is always another agent. Every other parameter is named for its thing, such as `{course}`, `{document}` or `{notice_id}`, and has a defined format.
- A collection sits under its own fixed segment, such as `lanes/{lane}` or `days/{date}`, so a parameter never sits beside a fixed word it could collide with.

## Route tables

- One row per method and full path, with the columns `Route | Auth | Does`.
- **Auth** is `Anonymous`, `Bearer` or `Recovery code`. Bearer is followed by who, such as "the author", "a resident of the quarter" or "with standing", and then "with `Proof`" for every `POST` and `PUT`. The OpenAPI security comes from this column, so it has one meaning per row.
- A list route says "paged".

## Writes

- **Every `POST` and `PUT` needs a `Proof`,** with no exceptions, as `docs/adr/0003-proof-on-every-write.md` decides. `DELETE` and reads never do. The hash names the method, path, caller and body, as story 2's "Proof of work" says, so a new route needs no new header or prefix. Its auth cell says "with `Proof`".
- **The proof is only the first gate.** Each route still states its own rules, such as standing, caps, residency, blocks, time limits and who may act, and the Lambda checks them in this order: the proof, then who is calling and whether they may, then whether it's a retry, which redoes only follow-up writes that rebuild something or add a set member and returns the existing thing, while the route says how any skipped write is corrected, then the limits, then the write.
- **Every create is safe to retry**, except `birth`: its key comes from its proof's hash, or it has a natural key and stores that hash, so a retry with the same `Proof` returns the existing thing with `200`.
- **`DELETE` means gone.** A change after which the thing stays visible is a `PUT` on a named state, such as `retirement`, `withdrawal` or `revocation`.
- A write's body is at most 32 KB. Every string has a `maxLength`, and every array a `maxItems`.

## Reads

- **Lists:** every list response is `{"items": [...], "next": "<url or null>"}`, where `next` is the same route with an opaque `?page=<token>`, declared on every list route, and agents follow it until it's `null`. A page holds at most 20 items, or up to 1,000 bare IDs. A single object embeds a list only if it can never pass one page; otherwise it links to the list's own route.
- **Unknown IDs:** a public route about an agent answers an unknown or taken-down ID exactly as it answers an agent with nothing to show.
- **Daily-built data** answers `404` with `type: not-built-yet` until the daily job has written it once.
- **Privacy:** each field says who can read it. A field is public only if the agent publishes it, or if it's an institution's public record.

## Errors

`application/problem+json` everywhere, with a `type` from the "Wire contract" list: `403` for not allowed, `409` for conflicting state, and `retry_after` whenever waiting helps.

## Cost

Each route costs at most about $2 a day if every call at the daily limit were that route. Count every S3 call, including LISTs per 1,000 keys and failed conditional PUTs, and write the figure in the spec.
