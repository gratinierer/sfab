# `birth` needs a genesis code and keeps the IP rule

Label: ready-for-agent
Blocked by: 2-identity-api#06

The `genesis` crate in `lambda/world/genesis/`, which owns every `genesis/` key (ADR 0004), and the Lambda side of `spec.md` "What `birth` does", "The IP rule" and the `genesis/` keys in "Storage". `identity::birth` calls `genesis` after the proof and before the birth, so `identity` depends on `genesis` (ADR 0008).

## Scope

- `genesis_code` as a required field of `birth` in `openapi/sfab.yaml`, with the problem type
  `invalid-genesis-code`, and the lost-answer case. No `CHANGELOG.md` line before launch (`docs/api-design.md` "Changes").
- The IP rule in `openapi/sfab.yaml`'s `birth`: a `403` response for `invalid-genesis-code`, `409 too-soon` with
  `retry_after` alongside `already-exists`, the `genesis_ip_window_hours` window, the IPv6 /64 grouping and that
  births sent together all pass; story 2's `409` row gains `too-soon` for `birth`, besides moves and exams.
- The checks in `spec.md`'s order: proof first and unchanged, then one HEAD of `genesis/codes/<code>`, then the IP
  record unless the code's metadata skips it, then the birth, then delete the code, write the IP record unless the code skips the rule and, for a
  world code, remove it from `genesis/pool.json` with a conditional PUT whose failure is ignored.
- The IP fingerprint: HMAC of the IPv4 address or IPv6 /64 prefix with `ip_hash_key`, from CloudFront's viewer
  address. No raw IP in any object or log line.
- `GET /world/genesis-code`: one random code from `genesis/pool.json`, `404 not-built-yet` before the daily job's first pool, or `409 try-later` with
  `retry_after` once it's empty or from an earlier UTC day, and its
  route in `openapi/sfab.yaml`.
- The proof-of-work worked example in `openapi/sfab.yaml` gets a `genesis_code` in its `birth` body, with its body
  hash and nonce recomputed and `lambda/core/tests/worked_example.rs` updated to match.
- Story 2's `spec.md` "Wire contract" gains `invalid-genesis-code` (`403`), `try-later` (`409`, unless story 4 added it first) and the route's
  `not-built-yet`, and its `409` row, its lost-`birth` paragraph, its "Proof of work" line "`birth` is the exception: its replay gets
  `409`", and `openapi/sfab.yaml`'s `409` row and its other `birth`-replay mention say a lost
  `birth` retry now gets `403 invalid-genesis-code`, as `docs/api-design.md` asks for new types.
- That `403` row (story 2 `spec.md` "Wire contract") and `openapi/sfab.yaml`'s "Errors and retries" (the `403` line
  that says "so don't retry. The exception is `origin-key-replaced`") gain `invalid-genesis-code` as a second exception:
  get a new code from `GET /world/genesis-code` and solve again, since the proof covers the code, so the retry is a new
  proof and a new identity, as its `detail` says.
  `birth`'s `403` gets its own response saying so, not a `$ref` to the shared `Forbidden`, whose "so don't retry"
  stays true for every other route.
- `head` on the `Store` trait in `lambda/core/src/storage.rs`, returning metadata, for `MemoryStore` there and `S3Store`
  in `lambda/core/src/s3.rs`, and for the test fakes that implement `Store` (in `lambda/takedown/tests/takedown.rs`,
  `lambda/identity/tests/profile.rs` and `lambda/world/census/tests/census.rs`), unless `head` gets a default.
- The viewer address in `Call` (`lambda/core/src/gate.rs`), filled in by the router in `lambda/api/src/lib.rs`.
- Choose the bucket for `genesis/` and record it in `spec.md`.
- The existing tests that birth: a shared test helper seeds a `genesis/codes/` object and sends its code. In
  `lambda/api/tests/contract.rs`, `fn birth` sends `genesis_code`, the replay assert expects `403` (declared on `birth`),
  and `GET /world/genesis-code` is exercised for the every-`operationId` check; `lambda/identity/tests/identity.rs`,
  `profile.rs`, `lambda/messaging/tests/messages.rs`, `lambda/takedown/tests/takedown.rs` and
  `lambda/api/tests/logging.rs` use the helper. The birth-replay tests (`identity.rs`
  `a_birth_replay_gets_409_and_changes_nothing` and the takedown-marker case, `takedown.rs` `rebirth`) re-seed the used
  code before the replay, so they still prove `409 already-exists` and that a taken-down ID can't be recreated, and a
  new case proves a plain replay gets `403 invalid-genesis-code`. The helper seeds `--no-ip-limit` codes, so tests that
  birth several times in one store from one address keep passing; the IP-rule tests seed ordinary codes and set the
  viewer address themselves.

## Done when

- Tests cover each `403` reason giving the same answer, `409 too-soon` inside the window, an exempt code skipping it,
  an expired code, IPv6 addresses in one /64 sharing a record, the pool losing a used code, and an empty pool, and
  a pool from yesterday.
- A test proves no request path writes or logs the raw IP.
- Every existing test that births passes again through the shared helper, and `contract.rs` covers the new route.
- The new routes stay within `docs/api-design.md` "Cost", as `spec.md` "Cost at the daily limit" works out.
