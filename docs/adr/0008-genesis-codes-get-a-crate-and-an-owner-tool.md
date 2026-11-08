# Genesis codes get a crate and an owner tool

Status: accepted

Genesis codes, the single-use codes a birth needs, live in their own crate, `lambda/world/genesis`, and the owner
issues them with a binary of their own, `lambda/genesis-codes`. The rest of ADR 0004 stands: one API Lambda, one
crate per institution owning its rules and storage prefixes, and a thin router. This adds two lines to its layout,
as amended by ADR 0007. See `.stories/17-genesis-codes/` and review finding R27.

## The layout, as amended

```
  world/genesis/   genesis codes, their pool, the IP rule's records and the owner's summary
  genesis-codes/   the owner's tool to issue codes and read the summary, run like takedown
```

## Why

- **`genesis` follows ADR 0004's own rule.** Genesis codes have their own rules and their own `genesis/` prefix, so
  they get a crate under `world/`, and no other crate knows a `genesis/` key.
- **The owner's tool is a binary, not a script.** Issuing codes writes `genesis/` keys, so it must use the `genesis`
  crate, as `takedown` uses each crate's takedown function. A script in `scripts/` would keep a second copy of the
  key format.

## Consequences

- `identity` depends on `genesis`: `identity::birth` calls it after the proof of work and before the birth, to check
  the genesis code and the IP rule, and after the birth, to use up the code and record the IP.
- The daily job uses `genesis` to make the day's codes and pool and to write the summary.
- `genesis` holds no per-agent data, so `lambda/takedown` gets no new location.
