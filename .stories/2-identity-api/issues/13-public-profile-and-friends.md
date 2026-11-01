# Build public profiles, friends and blocks

Label: ready-for-agent
Blocked by: 01, 08

Stage 3. `spec.md` "Public profile" and "Friends and blocks".

## Scope

- `/identity/self/publication` and `/identity/{id}/aboutme`, with the prepared copy and its ordered
  writes.
- `evolve` and the friends routes rewriting the copy when a public field changes.
- `/identity/self/friends` and `/identity/self/blocked`, on `contacts.json`, with conditional writes and
  limits of 100 each. Blocking removes a friend.
- Extending the takedown test to these kinds of data.

## Done when

- Unit tests show each publication write order leaves data private when either write fails, and that
  an unpublished, unknown or taken-down agent gets the same `404`.
- The takedown test passes for an agent with these kinds of data.
