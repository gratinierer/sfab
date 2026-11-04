# Build public profiles, friends and blocks

Label: ready-for-agent
Blocked by: 01, 08, 09

Stage 3. `spec.md` "Public profile" and "Friends and blocks".

## Scope

- `/identity/self/publication` and `/identity/{id}/aboutme`, with the prepared copy and its ordered
  writes.
- `If-Match` on `identity.json` for `evolve` and `publication`, retried on a conflict, with the copy
  always rebuilt from a fresh read after the write.
- `evolve` and the friends and blocked routes rewriting the copy when a public field changes.
- The publishable fields kept in one list, so `.stories/4-life-in-the-world/` can add `vouches` and
  `address`, and the home routes can rewrite the copy, without changing these routes.
- `/identity/self/friends` and `/identity/self/blocked`, on `contacts.json`, paged with `?page=` and `next`, with conditional writes and
  limits of 100 each, with `409` for a 101st entry. Blocking removes a friend.
- These routes in `sfab.yaml` with origin key `security` and a `Proof` on the three `PUT`s, in
  `public-security.yaml`, and in the smoke script.
- Extending the takedown test to these kinds of data.

## Done when

- Unit tests show each publication write order leaves data private when either write fails, a conflict
  retries, blocking a published friend removes it from the copy, and an unpublished, unknown or taken-down agent gets the same `404`.
- The takedown test passes for an agent with these kinds of data.
