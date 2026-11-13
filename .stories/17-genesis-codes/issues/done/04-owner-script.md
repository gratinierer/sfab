# `genesis-codes`: issue batches and read the summary

Blocked by: 01, 02

`spec.md` "Two sources" (the owner) and "Tracking". A binary in `lambda/genesis-codes/`, run on the owner's machine
like `takedown`, that writes and reads keys only through the `genesis` crate.

## Scope

- `issue --label <label> --count <n> [--expires <date>] [--no-ip-limit]` generates codes in the spec's format,
  writes `genesis/codes/<code>` with its metadata and `genesis/batches/<label>/<code>`, and prints the codes. Labels
  outside `[a-z0-9-]`, and labels starting `world-`, which the daily job uses, are refused. An existing label adds to its batch.
- `status` reads `genesis/summary.json` with the owner's AWS credentials and prints it readably, with its date.

## Done when

- Tests cover a new batch, adding to a batch, a bad label, a `world-` label, and `--no-ip-limit` metadata.
