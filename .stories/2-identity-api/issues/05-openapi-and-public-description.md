# Write the OpenAPI document and generate the public description

Label: ready-for-agent
Blocked by: 04

Write the API's single source and everything built from it: `spec.md` "API", "Wire contract",
"Identity fields", and "Architecture" (OpenAPI, public description, build).

## Scope

- `openapi/identity.yaml`: the identity routes, following `docs/adr/0009-api-path-rules.md`, which later
  tickets add to, models with `additionalProperties: false`, the `$default`
  validator model, `problem+json` gateway responses including `MISSING_AUTHENTICATION_TOKEN` as a
  `404`, an ARN placeholder, and the origin key's `security` on every operation.
- `openapi/public-security.yaml`, and the generator that removes `x-amazon-*` and the `apiKey`
  scheme, merges in Bearer for every route needing credentials, and adds `servers` and `info.contact`.
- The agent guidance the description must carry: the proof-of-work rule, calling `self` after
  `reclaim`, the plain `403` rule, HTTPS only, that operators could read `whoiam`, and how often to
  poll for messages.
- The build stamp, the synth check against it, and the `deploy` script, which takes the stack to
  deploy and refuses to run with `SFAB_CONFIG` set.

## Done when

- Changing anything compiled into the Lambda without rebuilding fails synth, including
  `abuse_contact` (R43), `constitution.md`, `worlds.yaml` and `domain_name`.
- The public copy has no `x-amazon-`, `arn:aws`, origin key, account ID or `apiKey` scheme, has
  `info.contact`, and declares Bearer on every route needing credentials.
