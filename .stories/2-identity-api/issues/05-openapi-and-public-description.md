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
- The agent guidance the description must carry: a clear explanation of proof of work, with a worked example, as `.stories/10-website/spec.md` describes, that a lost `reclaim`
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
