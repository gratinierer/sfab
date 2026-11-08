# The smoke script births with exempt codes

Label: ready-for-agent
Blocked by: 03, 04, 2-identity-api#09

## Scope

- The full smoke script reads `--no-ip-limit` codes from `smoke-genesis-codes.txt`, one per line and one per birth,
  and says clearly when it runs out. Add the file to `.gitignore`.
- It also checks `GET /world/genesis-code` and one birth with that world code. Because the code is checked before the
  IP rule, either `201` or `409 too-soon` proves the code was accepted, so reruns from one IP within the window
  still pass; only `201` uses up a pool code.
- The proof-less `birth` probe that learns `required_bits` and the server's clock sends a well-formed dummy
  `genesis_code`, so the gateway's validator passes it on and the Lambda answers `400` with `required_bits`.
- The `birth` replay check expects `403 invalid-genesis-code`, not `409 already-exists`.
- `cdk/tests/fakes.py` follows both changes.
- Update `.stories/4-life-in-the-world/issues/10-launch-checklist.md` and `11-birth-the-founders.md`: the smoke identity
  and the founders birth with `--no-ip-limit` codes, and both gain `17-genesis-codes#04` on their `Blocked by` lines.
- Document how the owner tops up the file with `genesis-codes issue --label smoke --no-ip-limit`.

## Done when

- A smoke run against a deployed world births with exempt codes from one IP without hitting `409 too-soon`.
