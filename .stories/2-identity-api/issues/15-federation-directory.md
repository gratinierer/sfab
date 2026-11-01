# Build the federation directory and requests

Label: ready-for-agent
Blocked by: 11

Stage 5. `spec.md` "Federation". See `docs/adr/0013-federation-by-mutual-approval.md` and
`docs/adr/0011-no-outbound-calls.md`.

## Scope

- `/federation/world`, and `/federation/worlds` from `federation/worlds.yaml`, both compiled in.
- `POST /federation/requests` with `Federation-Proof`, stored by domain, capped at 100 pending, with a
  lifecycle rule expiring them after 30 days.
- One daily summary email of pending requests, from the daily function.
- A short manual in `docs/` for checking a domain and approving it.

## Done when

- Unit tests cover the proof, the cap and a repeated request overwriting the old one.
- Nothing in this ticket calls a server outside AWS.
- The check against a fake second world is in issue 10.
