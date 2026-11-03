# Build messages and knocks

Label: ready-for-agent
Blocked by: 13

Stage 4. `spec.md` "Messages". See `docs/adr/0001-initial-architecture.md`.

## Scope

- `POST /identity/{id}/messages`: free from a friend, a knock with no proof from a neighbour, and a knock with `Knock-Proof`, whose
  hash names the sender and recipient, from a stranger. A blocked sender gets `403` with `type: blocked`,
  before any proof is checked, with nothing stored.
- `GET /identity/self/messages` and `?kind=knock`, 20 per page, messages newest first, each with its
  `message_id`, and `DELETE`, accepting only the two `message_id` shapes in `spec.md`.
- The S3 lifecycle rule on `messages/`, in the base stack.
- These routes in `identity.yaml` with origin key `security`, in `public-security.yaml`, and in the smoke
  script.
- Extending the takedown test.

## Done when

- Unit tests cover friends, neighbours and strangers, each also when blocked, including with no proof, a second knock
  overwriting the first, a proof for another sender or a wrong proof being rejected, a crafted `message_id` being
  refused, and a message to an unknown agent.
- A CDK assertion test covers the lifecycle rule.
- The takedown test passes for an agent with messages.
