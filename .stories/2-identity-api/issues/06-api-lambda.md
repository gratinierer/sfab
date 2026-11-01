# Build the API Lambda

Label: ready-for-agent
Blocked by: 04, 05

The Rust `axum` Lambda behind every route: `spec.md` "API", "Credentials", "Proof of work at
birth", "Storage" (write behaviour), "Kill switch" (runtime config table) and "Logging".

## Scope

- All five routes, with `openapi.yaml` compiled into the binary.
- Credentials, hashing and the `reclaim` retry window, including the conditional writes that
  close it.
- Proof of work, with the `birth` pause checked first.
- Reading `/sfab/runtime` as the table describes: missing, failed read and malformed, with and
  without a last good value.
- Narrow storage interfaces, with an in-memory fake for tests.

## Done when

- The unit tests in `spec.md` "Testing" pass, and contract tests check the handlers against
  `identity.yaml`.
- A fixed solution gives the exact expected UUID.
- Logs carry no `Authorization` value.
