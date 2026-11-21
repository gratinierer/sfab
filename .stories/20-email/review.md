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
| 2026-11-21T11:05:00Z | walkthrough | 6 findings | claude-opus-5-5 with the owner | 0 new, 4 resolved, 1 waived, 0 still open | walkthrough, not a review pass: R1, R3, R5, R6 fixed (R6 moved the website work to story 10), R4 waived by the owner; R2 was resolved before it |
| 2026-11-21T11:07:41Z | review | 5cadeed822f0 | claude-sonnet-5-5 (dispatched; story authored by claude-opus-5-5, orchestrated by claude-opus-5-5) | 6 new, 6 closures audited and held | full: every story file changed since 741d12bcb06a; auto round 1. Same vendor family, so weaker independence. HEAD moved to 5cadeed822f0 (another session closed story 2) while the pass ran, and the reviewer saw that tree |

## Findings

| # | Status | Axis | Affects | Description | Comments |
|---|---|---|---|---|---|
| R1 | resolved | completeness | 03, spec.md "The new contact" | `abuse_contact` is also published at `GET /v1/federation/world` as `contact` (`lambda/api/src/main.rs:30`, `federation/world.yaml:2-3`), which neither the spec's list of where the address appears nor 03's Done when covers. | 2026-11-21: weightier than found: `world/constitution.md` item 7 tells agents to "reach the operator through the contact at `/v1/federation/world`". 2026-11-21 (walkthrough): fixed: spec.md "What's there now" names the federation endpoint, and 03's Scope and Done when cover its `contact`. |
| R2 | resolved | consistency | 03, spec.md "The new contact" | 03 puts the address only on `website/index.html`, but `.stories/10-website/spec.md:58` requires `/` and `/agents/` to be mirrored with the same sections, and agents are the readers who need the abuse contact. | 2026-11-21: 03 and spec.md "The new contact" now name both pages, and story 10's spec gained "Contact", on the owner's ask to document the address there. |
| R3 | resolved | completeness | 01 | "When the field is set" is unspecified for `""`, `CHANGE_ME` or a value pasted with its `apple-domain=` prefix: `cdk/sfab_config.py`'s optional-key loop (`:180`) applies neither strip nor the placeholder check, so each would publish a broken TXT. | 2026-11-21 (walkthrough): fixed: 01's Scope and Done when check the key when present (strip, not empty, no `CHANGE_ME`, `[A-Za-z0-9]{8,64}`, a hint on the prefix), with a test per refusal. |
| R4 | wontfix | completeness | 02, 03 | Both tickets say only "deploy", but `scripts/deploy` takes `--all` or stack names (`scripts/deploy:5`): 02 needs `SfabBase`, 03 the API stack and the website stack. | 2026-11-21 (walkthrough): waived by the owner: "a deploy all is standard", so a plain deploy means `scripts/deploy --all`. |
| R5 | resolved | consistency | spec.md "Design", 03 | The design creates `abuse@` but publishes `operator@` under the label "Abuse and takedown reports" (`cdk/generate_public_openapi.py:93`), and doesn't say why `abuse@` exists if it's never published. | 2026-11-21: the rename to `watchdog@` (Q21) made the label fit. 2026-11-21 (walkthrough): fixed: spec.md "Addresses" says `watchdog@` is the only published address, and why `abuse@` and `info@` exist. |
| R6 | resolved | staleness | 03 | 03 edits `website/index.html`, which story 10's open tickets 01 and 02 will rewrite; with no blocking (Q10), whichever lands second has to rework the other's change. | 2026-11-21 (walkthrough): fixed on the owner's ruling "in this ticket: only update story 10, do not change website code": 03 changes no website file and is now `ready-for-human`; story 10's spec "Contact" and its ticket 02 take the `{{contact}}` placeholder and the link (Q23). |
| R7 | open | staleness | 02, 03 | 02 step 3 and 03 say "deploys" from `scripts/deploy`, but deploys run in CodeBuild from the `production` branch (`.stories/done/2-identity-api/spec.md:899`, ADR 0009), and setting the key before 01 is on `production` fails the build on `parse`'s unknown-key check (`cdk/sfab_config.py:165`). | |
| R8 | open | staleness | 01, spec.md "What's there now" | 01 edits `.stories/2-identity-api/spec.md` "Delivery" and spec.md cites story 2 at that path, but story 2 closed into `.stories/done/2-identity-api/` (`5cadeed`), so the path is gone and the edit would change a closed story. | |
| R9 | open | factual | spec.md "What's there now" | "Story 2 'Storage' … rely on it as the abuse contact" cites the wrong section: "Storage" (`.stories/done/2-identity-api/spec.md:473`) doesn't mention it; the contact is at lines 214, 309 and 918. | |
| R10 | open | consistency | spec.md "Clarifications" | Q10 ("a later ticket in this story") and Q17 (03 "puts the address on `index.html`") contradict Q23 and 03, which changes no website file, and carry no "superseded by Q23" as Q2, Q6, Q7 and Q11 do. | |
| R11 | open | completeness | 02, spec.md "Design" | 02 step 5 says to note a limit below four in spec.md but not which address to drop, so the Design's four addresses have no fallback. | |
| R12 | open | completeness | 01, 02 | Nothing checks that the zone holds no apex `TXT` or `MX`, `sig1._domainkey` or `_dmarc` record before the first deploy with the key, though CloudFormation fails `SfabBase`'s update on an existing record. | |
