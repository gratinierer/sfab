# 17-genesis-codes: code review

Code reviews of this story's issues. The journal lists every pass: `code-review` runs of `/code-review`, and
`walkthrough` passes of `/address-code-review-findings`. The findings table lists what each run found and what became
of it. Rows are never deleted or renumbered: a fixed finding flips to `resolved`.

Statuses: `open` / `resolved` / `wontfix` / `superseded`.
Axes: `standards` / `spec`.
Conventions: `docs/agents/issue-tracker.md`, section "Code review file".

## Review journal

| Date | Kind | Reviewed | By | Findings | Notes |
|---|---|---|---|---|---|
| 2026-11-09T03:00:00+00:00 | code-review | 01, 02, 03, 04, 05, 06 @ 15accf8..6262c50 | claude-fable-5-1 (dispatched; code by claude-opus-5-5) | 10 new: 7 standards, 3 spec | no hard standards violations; spec reviewer called three extras defensible scope (bracketed/IPv4-mapped viewer addresses, smoke drops a `403` code, `status`'s extra LIST) |

## Findings

| # | Status | Axis | Affects | Description | Comments |
|---|---|---|---|---|---|
| C1 | resolved | standards | 01, 02, 04, lambda/world/genesis/src/daily.rs:172 | Possible Duplicated Code: "make N codes and issue each" appears in `daily.rs:172-177`, `genesis-codes/src/lib.rs:72-77` and `genesis/src/lib.rs:393-398`; "list the summary key, then GET it" in `read_summary` (`daily.rs:79-94`) and `previous_summary` (`daily.rs:217-231`); the unused-code filter at `daily.rs:119` and `:153`; and `window_hours * 3_600` at `ip.rs:35` and `daily.rs:198`, where `IpRule::window_seconds` would serve. | 2026-11-09: found. 2026-11-08: fixed: `issue_new` (make a code and issue it) now serves `daily`, `genesis-codes::issue_codes` and `testing`, each keeping its own loop (parallel, partial-on-error, seeding); `summary_object` is the one LIST-then-GET under `read_summary` and `previous_summary`, which differ only in how they treat a bad parse; `still_unused` is the one filter. The `window_hours * 3_600` pair goes with C2. Checked: clippy, `cargo test --workspace`. |
| C2 | open | standards | 01, 02, lambda/world/genesis/src/daily.rs:29 | Possible Data Clumps: the window lives in both `IpRule.window_hours` (`ip.rs:14`) and `Daily.ip_window_hours`, and `GENESIS_IP_WINDOW_HOURS` is parsed inline in `api/src/main.rs:32-34` and through a `number()` helper in `job/src/main.rs:16-20`. | 2026-11-09: found. |
| C3 | open | standards | 01, lambda/identity/src/lib.rs:214 | Possible Middle Man: `Identity::new` takes an `IpRule` only to build its own `Genesis`, while `app` builds a second `Genesis` from a clone of the same rule (`api/src/lib.rs:70,75`); one shared `Genesis` would do. | 2026-11-09: found. |
| C4 | open | standards | 01, lambda/world/genesis/src/lib.rs:63 | Possible Primitive Obsession: the genesis code and label travel as `String`/`&str` with `is_code` re-checked at the edges (`lib.rs:63,102,188`), and the `no-ip-limit` metadata is string-compared to `"true"` (`lib.rs:94,211`). | 2026-11-09: found. |
| C5 | open | standards | 01, lambda/world/genesis/src/lib.rs:168 | Possible Duplicated Code: `storage_failure` now exists in `genesis`, `identity` (`lib.rs:203`) and `messaging` (`lib.rs:176`), though ADR 0004 puts what every route shares in `core`; and `not-built-yet` is an inline literal (`genesis/src/lib.rs:321-325`) where census has a `NOT_BUILT_YET` constant. | 2026-11-09: found. |
| C6 | open | standards | 03, cdk/stacks.py | Possible Shotgun Surgery: `cdk/stacks.py` spells `genesis/codes/`, `genesis/ips/` and `genesis/pool.json` for IAM and lifecycle, while ADR 0008 says no other crate knows a `genesis/` key; CDK isn't a crate and lists `messages/` the same way, so this is soft. | 2026-11-09: found. |
| C7 | open | standards | 01, lambda/world/genesis/src/lib.rs:155 | Possible Mysterious Name: `Admission.world: bool` means "came from the daily pool" (`from_pool` would say so), and `random_below`'s doc "which is never 0" (`lib.rs:343`) means `n`, not the result. | 2026-11-09: found. |
| C8 | open | spec | 05, openapi/sfab.yaml | Issue 05's "the API description's text agrees with both lines" is partial: the constitution promises the IP hash is deleted "usually within about a day and always within a few days", but `openapi/sfab.yaml` "Genesis codes" gives no deletion period. | 2026-11-09: found. |
| C9 | open | spec | 06, cdk/smoke.py:486 | The world-code birth accepts only `201` or `409 too-soon`, but the pool is "only mostly fresh" (`spec.md` "What `birth` does" step 5), so a code a racing birth already used gets `403 invalid-genesis-code` and the smoke run fails although the world behaves as designed (`cdk/smoke.py:486-495`, `cdk/tests/test_smoke.py`). | 2026-11-09: found. |
| C10 | open | spec | 02, lambda/job/src/lib.rs:34 | The genesis step runs after the census (`job/src/lib.rs:34-36`), so a failed census step skips the day's codes and pool and `GET /world/genesis-code` answers `try-later` all day; the spec is silent on the order, but codes shouldn't hinge on the census. | 2026-11-09: found. |
