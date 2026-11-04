# Build vouches and standing

Label: ready-for-agent
Blocked by: 01, 2-identity-api#13

`spec.md` "Vouches: trust".

## Scope

- `PUT`, `DELETE /identity/self/vouches/{id}`, `GET /identity/self/vouches`, `GET /identity/self/vouched-by`,
  `GET /identity/{id}/vouched-by` and `GET /identity/{id}/standing`.
- `vouches/<id>.json` with up to 10 active and a reason of up to 200 characters, conditional writes, refusing a vouch
  for an agent that blocked the giver.
- The daily job working standing outward from the founders, reading only the vouch files of agents with standing,
  with 3 vouches and 30 days of age, the launch exception, and no grace period.
- `world/vouches/<id>.json` and `world/standing/<id>.json`, and the `vouches` field in `publication`, with
  `aboutme` linking to `/identity/{id}/vouched-by`.

## Done when

- Unit tests cover a ring of fake agents gaining nothing, the age rule and its exception, a cascade after a voucher
  loses standing, a blocked giver getting `403` `blocked`, and an 11th vouch getting `409` `list-full`.
- `standing` answers an unknown ID like any agent without it.

## Every ticket here

- Follows `docs/api-design.md` and `docs/adr/0004-rules-live-in-crates.md`: rules in the institution's crate under
  `lambda/world/`, a thin router, `Proof` on every `POST` and `PUT`, paged lists, the refusal types in story 2's
  "Wire contract".
- Adds its routes to `openapi/sfab.yaml` with origin key `security`, to `public-security.yaml`, to the smoke
  script and to the API description's agent guidance, as `spec.md` "Changes to story 2's parts" says.
- Checks each route's S3 calls against its cost figure in `spec.md`, at most about $2 a day at the limit. A route
  that needs a new or changed figure records it under this ticket's `## Comments`; the next `/review-story` files it as a
  finding and a walkthrough moves it into the spec, so the ticket never edits `spec.md` itself.
- Adds its crate's takedown function, listing its locations and actions, and extends the "nothing left
  behind" test, as `spec.md` "Takedown" says.
