# The daily job makes the world's codes and the owner's summary

Label: ready-for-agent
Blocked by: 01, 2-identity-api#11

`spec.md` "Two sources" (the world) and "Tracking", in the daily world job, through the `genesis` crate.

## Scope

In this order, so the counts see yesterday's codes before they go:

1. Read the previous `genesis/summary.json`, if any, to keep its world history.
2. List `genesis/codes/` and `genesis/batches/` once each, and count per label `uploaded` and `left`, and for
   yesterday's world batch `used`.
3. Write `genesis/summary.json`, dropping world days older than 30.
4. Then the pool, below.

- Each run deletes the previous day's unused world codes, makes `genesis_codes_per_day` new ones under
  `genesis/codes/` and `genesis/batches/world-<date>/`, valid for the UTC day, and writes `genesis/pool.json` with its UTC date.
- The summary holds per label `uploaded` and `left`, and for the world `issued` and `used` per day, for 30 days.

## Done when

- Tests cover a fresh world, a day with leftovers, an owner batch half used, and an expired owner batch.
- The run's S3 calls match `spec.md` "Cost at the daily limit".
