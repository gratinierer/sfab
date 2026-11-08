# The daily job makes the world's codes and the owner's summary

Label: ready-for-agent
Blocked by: 01, 2-identity-api#11

`spec.md` "Two sources" (the world) and "Tracking", in the daily world job, through the `genesis` crate.

## Scope

In this order, so the counts see yesterday's codes before they go:

1. Read the previous `genesis/summary.json` to keep its world history, listing it first, because a GET on a missing
   key gets `403` under the job's prefix-conditioned `ListBucket` (`docs/aws-facts.md`), as `lambda/world/census` does.
2. List `genesis/codes/` and `genesis/batches/` once each, and count per label `uploaded` and `left`, and `used` for
   every world day before today not yet in the summary, so a missed run's day is still counted.
3. Write `genesis/summary.json`, dropping world days older than 30.
4. Then the pool, below.

- Each run deletes every unused world code from any day before today, not only yesterday's, so a failed run leaves
  nothing behind, makes `genesis_codes_per_day` new ones under
  `genesis/codes/` and `genesis/batches/world-<date>/`, valid for the UTC day, and writes `genesis/pool.json` with its UTC date.
- It lists `genesis/ips/` and deletes every record whose `LastModified` is older than `genesis_ip_window_hours`, so a
  record usually lives about 28 hours at most; a failed run (the job has no retries) leaves it to the next run or to lifecycle, as the Terms' "usually within about a day, always within a few days"; the lifecycle rule is only a backstop.
  `Listed` in `lambda/core/src/storage.rs` gains `last_modified`, which `S3Store` (`s3.rs`) fills from the listing and
  `MemoryStore` from a time a test can set, so a test can age a record.
- The summary holds per owner label `uploaded` and `left`, and for the world `issued` and `used` per day, for 30 days.
  `world-<date>` labels never appear in `labels`. Codes past their `--expires` date are neither deleted nor HEADed,
  and still count as `left`.

## Done when

- Tests cover a fresh world, a day with leftovers, a run after a missed day, an owner batch half used, and an expired owner batch (its codes still counted as `left`, no HEAD made), and an
  IP record older than the window being deleted while a newer one stays.
- The run's S3 calls match `spec.md` "Cost at the daily limit".
