# Write the smoke script and abuse check

Label: ready-for-agent
Blocked by: 01

The two scripts described in `spec.md` "Testing", which run against the deployed API. Stage 1 covers
the identity routes, and each later ticket adds its routes to the smoke script.

## Scope

- The paced smoke script, including the gateway validation cases, both `404`s, a direct
  `execute-api` request without the key, and treating WAF's plain `403` as "wait and rerun".
- The separate abuse check: about 10 rapid requests, then about one per second for up to 5 minutes.

## Done when

- The owner has run both against the deployed API with stage 1's routes, and they pass. This container has no AWS
  credentials, but neither script needs any.
