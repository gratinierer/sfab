# Write the takedown script

Label: ready-for-agent
Blocked by: 04

`scripts/takedown`, as described in `spec.md` "Storage": the procedure the published abuse contact
relies on.

## Scope

- Three modes: one identity by UUID, all identities in a `born_at` window, and a report of
  identities with no claim.
- A dry run by default, until `--execute`.
- The saved UUID file that makes a run resumable, and the delete order: claim, wait at least the
  Lambda timeout once per batch, then identity.

## Done when

- Tests against faked S3 calls cover all three modes, the dry run, resuming after a failure, and
  deletes that already happened counting as success.
- The deployed check itself is in issue 10.
