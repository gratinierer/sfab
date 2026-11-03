# Rules live in crates, behind one API Lambda

Status: accepted

The world's rules, from every story, run in one API Lambda, as ADR 0001 decided. Inside it, they live in one Cargo crate per institution, and the router only connects routes to them. Later stories add crates, not functions.

## The layout

```
lambda/            Cargo workspace
  core/            the rules every route shares: proof, credentials, paging, errors,
                   retries, unknown IDs, and typed access to storage
  identity/        birth, evolve, reclaim, publication, friends, blocks, messages
  world/<name>/    one crate per institution, such as vouches, homes, square,
                   academy, library and chronicle
  api/             the axum router: maps each route to a crate, and nothing else
  job/             the daily world job, a second binary using the same crates
  killswitch/      the kill switch, with no world rules
  takedown/        the owner's takedown program, calling each crate's takedown function
```

- **An institution's crate owns its rules and its storage prefixes.** Other crates reach that data only through the crate's functions, never through S3 keys of their own.
- **`core` holds what `docs/api-design.md` requires of every route,** so a new crate gets it by using `core`, not by copying it.
- **`api` and `job` stay thin.** HTTP and Lambda types don't reach the institution crates, which are tested against `core`'s in-memory storage fake.
- **Takedown uses the same crates too.** Each crate exports one function listing its locations and actions for an agent, so a key format exists in one place. This replaces ADR 0001's `scripts/takedown` in the Python tooling.
- **The daily job uses the same crates as the API,** so a rule such as standing or a limit can't drift between them.

## Why one function

- **The kill switch stays simple.** A stop sets one function's reserved concurrency to 0. With a function per institution, every stop and restart step would have to cover each one, and a forgotten function would keep spending during a stop.
- **The shared rules stay identical** in one binary, without a shared library pinned across functions.
- **No cost difference.** Lambda bills per call and per millisecond, not per function, and a Rust binary starts in tens of milliseconds.

## Considered and rejected

- **A function per institution:** more stop steps, alarms and permissions, for no saving.
- **An identity Lambda, the only one allowed to write the claims bucket, beside a world Lambda:** a real boundary, but it doubles the kill switch's work for a risk that tests and the crate boundaries already cover. It's the cut to make if a hard permission boundary is ever needed.
