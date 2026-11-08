# `birth` needs a genesis code and keeps the IP rule

Label: ready-for-agent
Blocked by: 2-identity-api#06

The Lambda side of `spec.md` "What `birth` does", "The IP rule" and the `genesis/` keys in "Storage".

## Scope

- `genesis_code` as a required field of `birth` in `openapi/sfab.yaml`, with the problem type
  `invalid-genesis-code`, the lost-answer case, `info.version` and a `CHANGELOG.md` line.
- The checks in `spec.md`'s order: proof first and unchanged, then one HEAD of `genesis/codes/<code>`, then the IP
  record unless the code's metadata skips it, then the birth, then delete the code, write the IP record unless the code skips the rule and, for a
  world code, remove it from `genesis/pool.json` with a conditional PUT whose failure is ignored.
- The IP fingerprint: HMAC of the IPv4 address or IPv6 /64 prefix with `ip_hash_key`, from CloudFront's viewer
  address. No raw IP in any object or log line.
- `GET /world/genesis-code`: one random code from `genesis/pool.json`, or `404 none-left` with `retry_after`, and its
  route in `openapi/sfab.yaml`.
- The proof-of-work worked example in `openapi/sfab.yaml` gets a `genesis_code` in its `birth` body, with its body
  hash and nonce recomputed and `lambda/core/tests/worked_example.rs` updated to match.
- Story 2's `spec.md` "Wire contract" gains `invalid-genesis-code` (`403`), `none-left` (`404`) and the route's
  `not-built-yet`, and its `409` row, its lost-`birth` paragraph and the `409` row in `openapi/sfab.yaml` say a lost
  `birth` retry now gets `403 invalid-genesis-code`, as `docs/api-design.md` asks for new types.
- Choose the bucket for `genesis/` and record it in `spec.md`.

## Done when

- Tests cover each `403` reason giving the same answer, `409 too-soon` inside the window, an exempt code skipping it,
  an expired code, IPv6 addresses in one /64 sharing a record, the pool losing a used code, and an empty pool.
- A test proves no request path writes or logs the raw IP.
- The new routes stay within `docs/api-design.md` "Cost", as `spec.md` "Cost at the daily limit" works out.
