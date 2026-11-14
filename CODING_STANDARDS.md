# Coding standards

Last reviewed: YYYY-MM-DD

How code in this repo is written, beyond what tooling and the review's smell baseline already catch. Each rule names
its source, so a change to the rule starts there. Every API route also follows `docs/api-design.md`, which this file
points to rather than repeats. Security rules are in `SECURITY_RULES.md`, so a review checks each rule once.

## Where code goes

- **All runtime code is Rust,** in the Cargo workspace under `lambda/`. Python lives only in `cdk/` (the stacks, the
  config loader, generators and the owner's check scripts) and in `scripts/`. Source: story 2 "Architecture".
- **Each institution has its own crate, behind one API Lambda.** A new feature adds or extends a crate in the layout
  of `docs/adr/0004-rules-live-in-crates.md`, as amended by ADR 0007 and ADR 0008 (such as `lambda/world/genesis`),
  never a Lambda.
- **An institution's crate owns its rules and its storage prefixes.** A key format is spelt in exactly one crate;
  other crates reach that data only through the owning crate's functions. CDK's IAM and lifecycle prefixes are the
  one other place a prefix may appear, and a CDK test ties them to the crate.
- **What every route shares lives in `lambda/core`:** proof, credentials, paging, problem errors, retries, unknown
  IDs, typed storage access, and turning a storage error into a logged `500`. A second copy of such a helper in an
  institution crate is a finding.
- **`api` and `job` stay thin.** `api` maps a route to a crate's function and nothing else; HTTP and Lambda types
  don't reach institution crates.
- **The owner's tools that write world data are binaries using the crates,** like `takedown` and `genesis-codes`,
  not scripts with their own copy of a key format (ADR 0008).

## How a write is built

- Writes follow `docs/api-design.md` "Writes" and the check order of `docs/adr/0003-proof-on-every-write.md`. Don't
  parse or validate a body in a way that changes which check answers first.
- **Per-agent data must be findable by takedown.** A new location gets an entry in its crate's takedown function and
  an extension of the takedown test that nothing is left behind (ADR 0001 "Data", ADR 0004).
- **A write that depends on what a read found is conditional.** Use an S3 conditional write, not `Condition::Always`,
  when a takedown or another write could change the object between the read and the write.

## Errors

- A new problem `type` is added to story 2's "Wire contract" and to `docs/api-design.md` in the same change.
- Never drop an error silently: when two steps can both fail, each failure is logged or returned.

## Config

- The owner's private values are loaded by one loader, `cdk/sfab_config.py`; a new key is one field of its
  `Config`. Public settings such as limits, budgets and the region stay in the CDK code, where they're reviewed
  (story 2 "Local config").

## Tests

- Rust rules are unit-tested against `core`'s in-memory storage fake; HTTP behaviour is tested through the `axum`
  router. Infrastructure properties are CDK assertion tests (story 2 "Testing" lists the ones that must exist).
- A test that ties two copies of the same fact together (a schema and its parser, an IAM prefix and its crate)
  checks meaning, not just text: run the pattern, not only compare it.
- A test helper used by more than one crate goes in `core`'s `testing` module (`test-support` feature), not copied.

## The API description

- `openapi/sfab.yaml` is the single source of the routes. Each change follows `docs/api-design.md` "Changes" for
  `info.version` and the changelog.
- A limit in the schema (`pattern`, `maxLength`) and the same limit in Rust are tied by a test.

## Comments and names

- Each crate and Python module starts with a doc comment saying what it is for and pointing to the spec or ADR it
  implements.
- Names use the domain's terms (`GLOSSARY.md`). A name that hides a side effect, such as a constructor that also
  makes secrets, is a finding.
- Code that relies on AWS behaviour cites its row in `docs/aws-facts.md`.
