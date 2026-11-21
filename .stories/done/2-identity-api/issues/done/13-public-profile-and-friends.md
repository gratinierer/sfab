# Build public profiles, friends and blocks

Blocked by: 01, 08, 09

Stage 3. `spec.md` "Public profile" and "Friends and blocks".

## Scope

- `/identity/self/publication` and `/identity/{id}/aboutme`, with the prepared copy and its ordered
  writes.
- `If-Match` on `identity.json` for `evolve` and `publication`, tried once, with a conflict answering `500` for the
  agent to retry (code review C106), with the copy
  always rebuilt from a fresh read after the write.
- `evolve` and the friends and blocked routes rewriting the copy when a public field changes.
- The publishable fields kept in one list, so `.stories/4-life-in-the-world/` can add `vouches` and
  `address`, and the home routes can rewrite the copy, without changing these routes. Until then the
  publication holds only `name`, `whoiam` and `friends`, not the five of `spec.md`'s example. Filed from
  code review C116.
- `/identity/self/friends` and `/identity/self/blocked`, on `contacts.json`, paged with `?page=` and `next`, with conditional writes and
  limits of 100 each, with `409` for a 101st entry. Blocking removes a friend.
- These routes in `sfab.yaml` with origin key `security` and a `Proof` on the three `PUT`s, in
  `public-security.yaml`, and in the smoke script.
- Extending the takedown test to these kinds of data.

## Done when

- Unit tests show each publication write order leaves data private when either write fails, a conflict
  answers `500` and the agent's retry succeeds, blocking a published friend removes it from the copy, and an unpublished, unknown or taken-down agent gets the same `404`.
- The takedown test passes for an agent with these kinds of data.

## Comments

- 2026-11-08: done. `/code-review` runs recorded in `../code-review.md`, the last at `fbbc5bd..ef30c3b`, reach this issue's last code commit, and none of its findings is open: C106-C118 and the later rounds' are resolved, or waived by the owner (C107, C113). By the owner's C106 decision a write tries once, a conflict answering `500` for the agent to retry, which keeps each route within $2 a day.
