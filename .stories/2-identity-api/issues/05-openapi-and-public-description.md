# Write the OpenAPI document and generate the public description

Label: ready-for-agent
Blocked by: 04

Write the API's single source and everything built from it: `spec.md` "API", "Wire contract",
"Identity fields", and "Architecture" (OpenAPI, public description, build).

## Scope

- `openapi/sfab.yaml`: the one API document for every story, starting with the identity routes, following `docs/api-design.md`, which later
  tickets add to, with the optional `Proof` header on every `POST` and `PUT`, `?page=` on every list route, and
  the `required_bits` `400` problem schema, models with `additionalProperties: false`, the `$default`
  validator model, `problem+json` gateway responses including `MISSING_AUTHENTICATION_TOKEN` as a
  `404` whose `detail` points to `/v1/openapi.yaml`, and `INVALID_API_KEY` as a `403` with `type: origin-key-replaced`, an ARN placeholder, and the origin key's `security` on every operation.
- `openapi/public-security.yaml`, and the generator that removes `x-amazon-*` and the `apiKey`
  scheme, merges in Bearer for every route needing credentials, and adds `servers` and `info.contact`.
- The agent guidance the description must carry: a clear explanation of proof of work, with the one worked example, a `POST /v1/identity/birth` with a concrete
  body, timestamp, nonce and resulting hash, which `.stories/10-website/`'s pages reuse, as its `spec.md` describes, that a lost `reclaim`
  response loses the identity, the plain `403` rule, including that a stop lasts until the owner restarts the API, HTTPS only, that operators could read `whoiam`, and how often to
  poll for messages: both kinds at the start of a session, and at most every 15 minutes when running all
  the time, because the API can't push and messages wait 30 days.
- The `deploy` script, which always generates and builds, deploys all stacks or a named one, and
  refuses to run with `SFAB_CONFIG` set.

## Done when

- The public copy has no `x-amazon-`, `arn:aws`, origin key, account ID or `apiKey` scheme, has
  `info.contact`, and declares Bearer on every route needing credentials.
- A contract test shows a request with a wrong origin key gets the `problem+json` `403` with
  `type: origin-key-replaced`.
- Contract tests fail if any string lacks a `maxLength` or any array a `maxItems`, if a `POST` or `PUT` lacks the `Proof` header, a list route lacks `?page=`, or a write
  route lacks the `required_bits` `400`.
- A Rust test in `core` runs the worked example through the API's own proof verifier and checks that the public
  description holds exactly those values, so a wrong or drifted example fails CI.

## Comments

- 2026-11-04: built in the working tree, at the owner's go-ahead to skip re-reviewing after issue 04's
  progress notes. `openapi/sfab.yaml` has `birth`, `evolve`, `self`, `reclaim` and `openapi.yaml`, with
  their response shapes for issue 06 to match: `birth` returns `id`, `claim`, `recovery`, a `notice` and the
  `constitution` path, and `reclaim` the same without the path. Gateway errors without a listed `type` use
  `about:blank`. The generator is `cdk/generate_public_openapi.py`, and `scripts/check` runs it from the
  active config. `core` has the proof verifier and `agent_id`, which issue 06 builds on. The worked example
  is nonce `614135`, giving 22 zero bits and ID `mfyk2nkunooc`. The wrong origin key test checks the
  `INVALID_API_KEY` gateway response in the document. Seeing it on the deployed API belongs to issue 09's
  smoke script. The `$default` content type and the inlined integration are `memory` rows in
  `docs/aws-facts.md`, which issue 01's first import proves. The `deploy` script from issue 04 already
  generates, builds, deploys all stacks or a named one, and refuses `SFAB_CONFIG`.
