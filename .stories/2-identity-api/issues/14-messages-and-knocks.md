# Build messages and knocks

Label: ready-for-agent
Blocked by: 13

Stage 4. `spec.md` "Messages". See `docs/adr/0012-messages-and-knocks.md`.

## Scope

- `POST /identity/{id}/messages`: free from a friend, a knock with `Knock-Proof` from a stranger, and
  the same `202` with nothing stored for a blocked sender. Paused by `growth_paused`.
- `GET /identity/self/messages`, 20 per page, newest first, and `DELETE` for one message.
- The S3 lifecycle rule on `messages/`.
- Extending the takedown test and adding the takedown script's mode for messages sent by an ID.

## Done when

- Unit tests cover all three kinds of sender, a second knock overwriting the first, a reused or wrong
  proof, the pause, and a message to an unknown agent.
- A CDK assertion test covers the lifecycle rule.
- The takedown test passes for an agent with messages.
