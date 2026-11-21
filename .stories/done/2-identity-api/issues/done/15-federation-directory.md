# Build federation: directory, requests and domain proof

Blocked by: 09, 11

Stage 5. `spec.md` "Federation". See `docs/adr/0001-initial-architecture.md`.

## Scope

- `/federation/world`, and `/federation/worlds` from `federation/worlds.yaml`, paged with `?page=`, both compiled in.
- `POST /federation/requests` with a `Proof` and hostname validation, stored in
  `federation/pending/`, returning a token, and capped at 100 pending.
- In the daily world job: fetching each pending request's token under the limits in `spec.md`, moving
  matches to `federation/verified/`, and one daily email listing verified requests.
- S3 lifecycle rules in the base stack: 3 days on `federation/pending/`, 30 on `federation/verified/`.
- These routes in `sfab.yaml` with origin key `security`, and in the smoke script.
- A short manual, `docs/federation.md`, for checking and approving a domain, and deleting the stored request, and for the
  requesting side of an sfab world: commit the token as the whole body of
  `website/.well-known/sfab-federation/<token>` and deploy, because a hand-copied file is pruned by the next deploy.
  The `202`'s instructions say the same and link the manual, which `.stories/10-website/`'s agent version links too.

## Done when

- Unit tests cover the proof, invalid domains, the cap, and a repeat returning the same token.
- The job's outbound check is tested against a fake server: a match, the token plus one trailing newline matching,
  any other extra content not matching, a wrong body, a redirect, a private
  address, a slow response and an oversized one.
- Nothing that handles a request calls a server outside AWS.
- The check against a fake second world is in issue 10.

## Comments

- 2026-11-08: done. `/code-review` runs recorded in `../code-review.md`, the last at `fbbc5bd..ef30c3b`, reach this issue's last code commit, and none of its findings is open: C138-C147 are resolved. The check against a fake second world stays in issue 10.
