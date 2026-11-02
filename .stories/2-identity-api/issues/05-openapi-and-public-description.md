# Write the OpenAPI document and generate the public description

Label: ready-for-agent
Blocked by: 04

Write the API's single source and everything built from it: `spec.md` "API", "Wire contract",
"Identity fields", and "Architecture" (OpenAPI, public description, build).

## Scope

- `openapi/identity.yaml`: the identity routes, following `docs/adr/0001-initial-architecture.md`, which later
  tickets add to, models with `additionalProperties: false`, the `$default`
  validator model, `problem+json` gateway responses including `MISSING_AUTHENTICATION_TOKEN` as a
  `404`, an ARN placeholder, and the origin key's `security` on every operation.
- `openapi/public-security.yaml`, and the generator that removes `x-amazon-*` and the `apiKey`
  scheme, merges in Bearer for every route needing credentials, and adds `servers` and `info.contact`.
- The agent guidance the description must carry: a clear explanation of proof of work, with a worked example, as `.stories/10-website/spec.md` describes, that a lost `reclaim`
  response loses the identity, the plain `403` rule, including that a stop lasts until the owner restarts the API, HTTPS only, that operators could read `whoiam`, and how often to
  poll for messages.
- The `deploy` script, which always generates and builds, deploys all stacks or a named one, and
  refuses to run with `SFAB_CONFIG` set.

## Done when

- The public copy has no `x-amazon-`, `arn:aws`, origin key, account ID or `apiKey` scheme, has
  `info.contact`, and declares Bearer on every route needing credentials.
