# Write the takedown program

Label: ready-for-agent
Blocked by: 04, 06

the `takedown` program, as described in `spec.md` "Storage": the procedure the published abuse contact
relies on.

## Scope

- A Rust command-line program in `lambda/takedown/`, using each crate's takedown function, so key formats live
  only in their crates.
- Two modes: one identity by ID, and all identities in a `born_at` window.
- One list of per-agent locations, each with its action, so story 4 can add "leave a note" and "replace
  the ID"; currently all delete: `identities/<id>/`, `messages/<id>/` and `vitals/<id>.json`, as described in
  `docs/adr/0001-initial-architecture.md`.
- A dry run by default, until `--execute`.
- The order: replace the claim with a zero-byte marker that keeps `born_at` in its S3 metadata, and delete `aboutme.json` straight away, wait at
  least the Lambda timeout once per batch, read the agent's files for pointers, handle the other
  locations, delete `identities/<id>/` last, and delete `aboutme.json` again.

## Done when

- Tests against `core`'s in-memory storage fake, filled through each crate's own write functions, cover both modes, the dry run, and deletes that already happened counting
  as success. A window run interrupted after writing markers finishes correctly on a rerun.
- A test proves a takedown leaves nothing behind for an agent with every kind of data, and that the ID
  can't be recreated. Later tickets extend it as they add data.
- The deployed check itself is in issue 10.
