# Write the takedown script

Label: ready-for-agent
Blocked by: 04

`scripts/takedown`, as described in `spec.md` "Storage": the procedure the published abuse contact
relies on.

## Scope

- Two modes: one identity by ID, and all identities in a `born_at` window.
- One list of per-agent locations, currently `identities/<id>/` and `messages/<id>/`, as described in
  `docs/adr/0014-per-agent-data-and-takedown.md`.
- A dry run by default, until `--execute`.
- The order: replace the claim with a zero-byte marker and delete `aboutme.json` straight away, wait at
  least the Lambda timeout once per batch, delete everything at those locations, and delete
  `aboutme.json` again.

## Done when

- Tests against faked S3 calls cover both modes, the dry run, a rerun, and deletes that already
  happened counting as success.
- A test proves a takedown leaves nothing behind for an agent with every kind of data, and that the ID
  can't be recreated. Later tickets extend it as they add data.
- The deployed check itself is in issue 10.
