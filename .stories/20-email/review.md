# 20-email: review

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
| 2026-11-21T10:50:39Z | review | 741d12bcb06a | claude-sonnet-5-5 (dispatched; story authored by claude-opus-5-5) | 6 new | full: first review. Same vendor family, so weaker independence. A seventh candidate (02's comments record steps done while it's blocked by 01) dropped: its own evidence says the `Blocked by` is right |

## Findings

| # | Status | Axis | Affects | Description | Comments |
|---|---|---|---|---|---|
| R1 | open | completeness | 03, spec.md "The new contact" | `abuse_contact` is also published at `GET /v1/federation/world` as `contact` (`lambda/api/src/main.rs:30`, `federation/world.yaml:2-3`), which neither the spec's list of where the address appears nor 03's Done when covers. | 2026-11-21: weightier than found: `world/constitution.md` item 7 tells agents to "reach the operator through the contact at `/v1/federation/world`". |
| R2 | resolved | consistency | 03, spec.md "The new contact" | 03 puts the address only on `website/index.html`, but `.stories/10-website/spec.md:58` requires `/` and `/agents/` to be mirrored with the same sections, and agents are the readers who need the abuse contact. | 2026-11-21: 03 and spec.md "The new contact" now name both pages, and story 10's spec gained "Contact", on the owner's ask to document the address there. |
| R3 | open | completeness | 01 | "When the field is set" is unspecified for `""`, `CHANGE_ME` or a value pasted with its `apple-domain=` prefix: `cdk/sfab_config.py`'s optional-key loop (`:180`) applies neither strip nor the placeholder check, so each would publish a broken TXT. | |
| R4 | open | completeness | 02, 03 | Both tickets say only "deploy", but `scripts/deploy` takes `--all` or stack names (`scripts/deploy:5`): 02 needs `SfabBase`, 03 the API stack and the website stack. | |
| R5 | open | consistency | spec.md "Design", 03 | The design creates `abuse@` but publishes `operator@` under the label "Abuse and takedown reports" (`cdk/generate_public_openapi.py:93`), and doesn't say why `abuse@` exists if it's never published. | |
| R6 | open | staleness | 03 | 03 edits `website/index.html`, which story 10's open tickets 01 and 02 will rewrite; with no blocking (Q10), whichever lands second has to rework the other's change. | |
