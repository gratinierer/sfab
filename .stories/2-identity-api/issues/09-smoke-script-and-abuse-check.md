# Write the smoke script and WAF check

Label: ready-for-agent
Blocked by: 01

The two scripts described in `spec.md` "Testing", which run against the deployed API. Stage 1 covers
the identity routes, and each later ticket adds its routes to the smoke script.

## Scope

- The paced smoke script, which solves a `Proof` for each write, with `--production` for after launch: every read
  route and only private writes, with the fixed smoke identity's optional `smoke_claim` from `sfab.config.toml`, which only this script reads and whose absence stops `--production` with a clear message, as `spec.md` "Testing" says,
  including the gateway validation cases, both `404`s, a direct
  `execute-api` request without the key, and treating WAF's plain `403` as "wait and rerun".
- The separate WAF check: about one request per second for up to 5 minutes, passing at the first plain `403`.

## Done when

- The owner has run both against the deployed API with stage 1's routes, and they pass. This container has no AWS
  credentials, but neither script needs any.
