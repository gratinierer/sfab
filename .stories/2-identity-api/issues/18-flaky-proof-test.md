# Make the wrong-proof test deterministic

Label: ready-for-agent
Blocked by:

`every_write_without_a_valid_proof_gets_400_with_nothing_stored` in `lambda/identity/tests/identity.rs` fails now
and then: on 2026-11-17 a `scripts/check` run after code review C175 failed with
`evolve: Reply { status: 200, … }` at `identity.rs:468`, and the test then passed 8 runs alone and a second full
run.

The likely cause: the test reuses a solved proof for another body, another route and another caller, and expects
each to be refused. With `BITS = 12`, a reused nonce still meets the difficulty for the changed request by chance
1 time in 4,096. Seven such cases per run (other body and other route for birth, evolve and reclaim, plus b's
proof under a's credentials) make about one failure in 585 runs. The server is right to accept such a proof: it
does meet the difficulty, so this is the test's flaw, not the API's.

## Scope

- Confirm the cause: count how often a reused proof meets `BITS` for the changed request, for example by running
  the case's check over many nonces in a small test.
- Make each "made for another …" case certain to fail, for example by searching nonces until the reused proof
  doesn't meet the difficulty for the changed request (`lambda/core/src/proof.rs` `solve` shows the check), and
  asserting that before calling the route.
- Keep the stale-proof and no-proof cases as they are: they can't pass by chance.

## Done when

- The test can't pass a wrong proof by chance, and a comment says why the nonce is chosen.
- The test passes 1,000 runs in a row (`cargo test -p sfab-identity --test identity
  every_write_without_a_valid_proof` in a loop), recorded under `## Comments`.

## Comments

- 2026-11-17: Cause confirmed: over 40,000 solved evolve proofs, the reused proof met `BITS = 12` for the other
  body 10 times and for the other route 7 times (expected about 9.8 each, 1 in 4,096). The test now searches for a
  nonce that meets `BITS` for the request it was made for and not for the changed one, and asserts both before
  calling the route. 1,000 runs in a row of `cargo test -q -p sfab-identity --test identity
  every_write_without_a_valid_proof`: 0 failures.
