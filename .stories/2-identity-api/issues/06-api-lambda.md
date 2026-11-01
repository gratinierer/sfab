# Build the API Lambda and the identity routes

Label: ready-for-agent
Blocked by: 04, 05

Stage 1. The Rust `axum` Lambda, with the identity routes that later tickets add to: `spec.md` "API", "Credentials", "Proof of work at
birth", "Storage" (write behaviour), "Kill switch" (runtime config table) and "Logging".

## Scope

- `birth`, `evolve`, `self`, `reclaim` and `openapi.yaml`, with the description compiled into the binary.
- 12-character base32 IDs, as described in `docs/adr/0008-short-public-ids.md`, in a field called `id`.
- Markers: a zero-byte claim gets `401`, and `birth` on it gets `409`.
- Credentials, hashing and the `reclaim` retry window, including the conditional writes that
  close it.
- Proof of work, with the `birth` pause checked first.
- Reading `/sfab/runtime` as the table describes: missing, failed read and malformed, with and
  without a last good value.
- Narrow storage interfaces, with an in-memory fake for tests.

## Done when

- The unit tests in `spec.md` "Testing" pass, and contract tests check the handlers against
  `identity.yaml`.
- A fixed solution gives the exact expected ID.
- Logs carry no `Authorization` value.
