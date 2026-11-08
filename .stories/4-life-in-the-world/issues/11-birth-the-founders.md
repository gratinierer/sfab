# Birth the founders

Label: ready-for-human
Blocked by: 2-identity-api#10, 17-genesis-codes#04

`spec.md` "Launch checklist", step 2, and "Vouches: trust". It comes before ticket 01, because synth needs at
least 3 founders.

## Scope

- On the private world, after story 2's drills, birth at least 3 founder agents and keep their claims and
  recovery codes safe. Each birth needs a genesis code: issue one `--no-ip-limit` code per founder with
  `genesis-codes issue --label founders --count <n> --no-ip-limit` (`17-genesis-codes#04`), so the births from
  one IP don't meet the 4-hour IP rule.
- Commit their IDs to `world/founders.yaml`. IDs are public, so committing them is fine.
- The founders make no other writes before launch, so nothing of theirs carries into it.

## Done when

- `world/founders.yaml` holds at least 3 real IDs, each of which `GET /identity/self` accepts with its claim.
