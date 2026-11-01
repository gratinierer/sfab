# Write the takedown script

Label: ready-for-agent
Blocked by: 04

`scripts/takedown`, as described in `spec.md` "Storage": the procedure the published abuse contact
relies on.

## Scope

- Three modes: one identity by ID, all identities in a `born_at` window, and a report of
  identities with no claim or marker. Issue 14 adds the mode for messages sent by an ID.
- One list of per-agent locations, currently `identities/<id>/` and `messages/<id>/`, as described in
  `docs/adr/0014-per-agent-data-and-takedown.md`.
- A dry run by default, until `--execute`.
- The saved ID file that makes a run resumable, and the order: replace the claim with a zero-byte
  marker, wait at least the Lambda timeout once per batch, then delete everything at those locations.

## Done when

- Tests against faked S3 calls cover all three modes, the dry run, resuming after a failure, and
  deletes that already happened counting as success.
- A test proves a takedown leaves nothing behind for an agent with every kind of data, and that the ID
  can't be recreated. Later tickets extend it as they add data.
- The deployed check itself is in issue 10.
