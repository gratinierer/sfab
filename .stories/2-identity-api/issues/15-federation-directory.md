# Build federation: directory, requests and domain proof

Label: ready-for-agent
Blocked by: 11

Stage 5. `spec.md` "Federation". See `docs/adr/0013-federation-by-mutual-approval.md`,
`docs/adr/0011-no-outbound-calls.md` and `docs/adr/0018-federation-domain-proof.md`.

## Scope

- `/federation/world`, and `/federation/worlds` from `federation/worlds.yaml`, both compiled in.
- `POST /federation/requests` with `Federation-Proof` and hostname validation, stored in
  `federation/pending/`, returning a token, and capped at 100 pending.
- In the daily world job: fetching each pending request's token under the limits in `spec.md`, moving
  matches to `federation/verified/`, and one daily email listing verified requests.
- S3 lifecycle rules in the base stack: 3 days on `federation/pending/`, 30 on `federation/verified/`.
- These routes in `identity.yaml` with origin key `security`, and in the smoke script.
- A short manual in `docs/` for checking and approving a domain, and deleting the stored request.

## Done when

- Unit tests cover the proof, invalid domains, the cap, and a repeat returning the same token.
- The job's outbound check is tested against a fake server: a match, a wrong body, a redirect, a private
  address, a slow response and an oversized one.
- Nothing that handles a request calls a server outside AWS.
- The check against a fake second world is in issue 10.
