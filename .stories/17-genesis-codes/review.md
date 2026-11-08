# 17-genesis-codes: review

Review record for this story. The journal lists every pass over it — reviews
and walkthroughs, distinguished by `Kind`; the
findings table lists what each pass found and what became of it. Rows are
never deleted — a fixed finding flips to `resolved`, it does not disappear.

Statuses: `open` / `resolved` / `wontfix` / `superseded`.
Axes: `factual` / `consistency` / `completeness` / `convention` / `staleness`.
Conventions: `docs/agents/issue-tracker.md`, section "Review file".

## Review journal

| Date | Kind | Reviewed | By | Findings | Notes |
|---|---|---|---|---|---|
| 2026-11-08T11:30:00+00:00 | review | 61c1a4f | claude-fable-5-1 (dispatched; story authored by claude-opus-5-5, orchestrated by claude-opus-5-5, auto round 1) | 18 new, 0 resolved, 18 open | first review |

## Findings

| # | Status | Axis | Affects | Description | Comments |
|---|---|---|---|---|---|
| R1 | resolved | completeness | 01, 02, 03 | No issue grants the API Lambda (`API_READ_WRITE_PREFIXES`, `API_DELETE_PREFIXES` in `cdk/stacks.py`) or the daily job (`cdk/world_job.py`) access to `genesis/`, or updates story 2's asserted permission lists and their CDK tests. | 2026-11-08: found. 2026-11-08 (auto walkthrough): fixed: issue 03 scope and "Done when" now carry the `genesis/` IAM for the API Lambda and the daily job, and the update to story 2's permission lists. |
| R2 | resolved | completeness | 06, spec.md | Step 5 of "What `birth` does" writes the IP record even for exempt codes, so issue 06's world-code smoke birth from the same IP gets `409 too-soon`, and reruns within 4 hours fail; also `smoke_claim` is a config key, not a file, and 06 isn't `Blocked by` 03. | 2026-11-08: found. 2026-11-08 (auto walkthrough): fixed: exempt births write no IP record (spec step 5, issue 01); issue 06 accepts `201` or `409 too-soon` for its world-code birth, names `smoke-genesis-codes.txt` and its `.gitignore` line, and is `Blocked by` 03. |
| R3 | resolved | completeness | 06 | `scripts/smoke` relies on a proof-less `birth` getting the Lambda's `400` with `required_bits` and on a replay getting `409 already-exists`; with `genesis_code` required, the gateway answers `BAD_REQUEST_BODY` and a replay gets `403`, and `cdk/tests/fakes.py` must follow; issue 06 lists neither. | 2026-11-08: found. 2026-11-08 (auto walkthrough): fixed: issue 06 scope now covers the proof-less probe's dummy `genesis_code`, the replay now getting `403`, and `cdk/tests/fakes.py`. |
| R4 | resolved | completeness | 01 | The API description's proof-of-work worked example (`openapi/sfab.yaml` birth body, pinned by `lambda/core/tests/worked_example.rs`) and story 10's worked example and agent steps need `genesis_code` and a get-a-code step; no issue carries them. | 2026-11-08: found. 2026-11-08 (auto walkthrough): fixed: issue 01 updates the `openapi/sfab.yaml` worked example and `worked_example.rs`; issue 05 updates story 10's worked example and agent steps. |
| R5 | resolved | completeness | 01, spec.md | Story 2's "Wire contract", the one list per `docs/api-design.md`, lacks `invalid-genesis-code` and `none-left`, and still says a lost `birth` retry gets `409` (story 2 spec.md:109, :121, and `openapi/sfab.yaml` 409 row). | 2026-11-08: found. 2026-11-08 (auto walkthrough): fixed: issue 01 now updates story 2's "Wire contract" with the new types and the lost-`birth` retry, and the `openapi/sfab.yaml` `409` row. |
| R6 | resolved | consistency | spec.md, 01, 02 | Before the first daily run `GET /world/genesis-code` answers `404 none-left`, but `docs/api-design.md` "Reads" requires daily-built data to answer `404 not-built-yet` until written once. | 2026-11-08: found. 2026-11-08 (auto walkthrough): fixed: `spec.md` "Two sources" and issue 01 answer `404 not-built-yet` before the first pool, and `none-left` only after. |
| R7 | open | completeness | 01, 02, spec.md | The route can serve yesterday's `pool.json` between midnight and the job's run (delayed up to 6 hours, or a failed run with no retries), handing out expired codes; the route should check the pool's date. | 2026-11-08: found. |
| R8 | open | completeness | 02 | Issue 02 doesn't fix the order (count world `used` from batches vs `codes/` before deleting leftovers) or say it reads the previous `summary.json` to keep 30 days of history. | 2026-11-08: found. |
| R9 | open | completeness | 04, 02 | An owner label such as `world-2026-11-08` passes `[a-z0-9-]` and merges into a world day's batch and counts; issue 04 should refuse labels starting `world-`. | 2026-11-08: found. |
| R10 | open | consistency | 04, 01, 02, spec.md | ADR 0004 says one crate owns each key format, but the story names no crate for `genesis/` and has `scripts/genesis-codes` write the keys; also `takedown` is now the Rust `lambda/takedown/`, not a script. | 2026-11-08: found. |
| R11 | open | consistency | spec.md, 01 | The story asks for a `CHANGELOG.md` line, but `docs/api-design.md` says the changelog starts at launch, the file doesn't exist yet (`2-identity-api#16` creates it), and story 2's C130 was superseded on this point. | 2026-11-08: found. |
| R12 | open | factual | spec.md, 03, 05 | "A lifecycle rule deletes each record after a day" and the Terms line "for a day" overstate S3: `docs/aws-facts.md` says expiry rounds up to the next midnight UTC and deletion lags, so a record can live about 2 days or more. | 2026-11-08: found. |
| R13 | open | completeness | 05, spec.md | The Terms change and storing a keyed IP hash (pseudonymous personal data) fall under the legal check gratinierer/sfab#3, which gates launch; the story never mentions it. | 2026-11-08: found. |
| R14 | open | completeness | 03 | No issue proves on a deploy that the viewer address reaches the Lambda and that a client can't spoof it; issue 03's "Done when" covers only synth and lifecycle. | 2026-11-08: found. |
| R15 | open | completeness | 01 | `Store` in `lambda/core/src/storage.rs` has no HEAD and `Call` in `lambda/api/src/lib.rs` carries no viewer address; issue 01 should say it adds both, for `S3Store` and `MemoryStore`. | 2026-11-08: found. |
| R16 | open | completeness | spec.md | `docs/api-design.md` "Route tables" requires a route table showing each route complies; the spec has none for `GET /world/genesis-code` or the changed `birth`. | 2026-11-08: found. |
| R17 | open | completeness | spec.md | Other pre-launch births need codes too: story 4's founders (`4-life-in-the-world#11`) and the fixed smoke identity in its launch checklist (`#10`); nothing links them or says they need exempt codes. | 2026-11-08: found. |
| R18 | open | factual | spec.md | "Cost at the daily limit" prices LIST per 1,000 keys listed, but S3 charges per 1,000 LIST requests, and "prices from memory" is stale: `docs/aws-facts.md` now records them, checked 2026-11-08. | 2026-11-08: found. |
