# The smoke script births with exempt codes

Label: ready-for-agent
Blocked by: 03, 04

## Scope

- The full smoke script reads `--no-ip-limit` codes from `smoke-genesis-codes.txt`, one per line and one per birth,
  and says clearly when it runs out. Add the file to `.gitignore`.
- It also checks `GET /world/genesis-code` and one birth with that world code. Because the code is checked before the
  IP rule, either `201` or `409 too-soon` proves the code was accepted, so reruns from one IP within the window
  still pass; only `201` uses up a pool code.
- `GET /world/genesis-code` is checked with the other world routes in `_world` (`cdk/smoke.py`), so the `--production`
  run reads it too. As for the census, `404 not-built-yet` passes before the daily job's first pool, and so does
  `409 try-later` with `retry_after` for an empty or stale pool; in both cases the full run skips the world-code birth
  and says so.
- The proof-less `birth` probe that learns `required_bits` and the server's clock sends a well-formed dummy
  `genesis_code`, so the gateway's validator passes it on and the Lambda answers `400` with `required_bits`.
- The `birth` replay check expects `403 invalid-genesis-code`, not `409 already-exists`.
- `cdk/tests/fakes.py` follows both changes, and `cdk/tests/test_smoke.py` gets the new labels (:41-77) and caps that
  allow the extra requests (one more in both modes for the route, one more in the full run for the world-code birth,
  so about 43 and 31, still well under the WAF limit at one every 4 seconds); story 2 `spec.md`'s "about 41 ... and 30"
  and `cdk/smoke.py`'s "near 41 requests" follow.
- Update `.stories/4-life-in-the-world/issues/10-launch-checklist.md` and `11-birth-the-founders.md`: the smoke identity
  and the founders birth with `--no-ip-limit` codes, and both gain `17-genesis-codes#04` on their `Blocked by` lines.
- Document how the owner tops up the file with `genesis-codes issue --label smoke --count <n> --no-ip-limit`.

## Done when

- `cdk/tests/test_smoke.py` passes against the fake with exempt codes from one IP and no `409 too-soon`. The run
  against the deployed world belongs to `2-identity-api#09`'s Done when, because that deploy waits for this issue.
- Story 4, whose documents this issue edits, gets a `/review-story` pass after the edit, so its review isn't
  left stale (`docs/agents/issue-tracker.md` "Before implementing a story").
