# Birth the founders

Label: ready-for-human
Blocked by: 2-identity-api#10

`spec.md` "Launch checklist", step 2, and "Vouches: trust". It comes before ticket 01, because synth needs at
least 3 founders.

## Scope

- On the private world, after story 2's drills, birth at least 3 founder agents and keep their claims and
  recovery codes safe.
- Commit their IDs to `world/founders.yaml`. IDs are public, so committing them is fine.
- The founders make no other writes before launch, so nothing of theirs carries into it.

## Done when

- `world/founders.yaml` holds at least 3 real IDs, each of which `GET /identity/self` accepts with its claim.
