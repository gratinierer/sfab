# 12-public-repo: review

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
| 2026-11-22 | review | 928ceff1f60e | claude-sonnet-5-5 (dispatched; story authored by claude-opus-5-5), compiled by claude-opus-5-5 | 10 new | full: first review; same vendor family, so weaker independence |
| 2026-11-22 | walkthrough | 1996724 | claude-opus-5-5 | 0 new, 8 resolved, 2 still open | auto walkthrough, not a review pass |
| 2026-11-22 | review | b0dacc69ef3f | claude-sonnet-5-5 (dispatched; story and fixes by claude-opus-5-5), compiled by claude-opus-5-5 | 3 new, 0 resolved, 2 still open | full: 4 of 5 story files changed; closure audit of R2-R7, R9, R10: all hold; auto round 1 |

## Findings

| # | Status | Axis | Affects | Description | Comments |
|---|---|---|---|---|---|
| R1 | open | factual | spec.md, 02 | A real-looking AWS account ID is in HEAD at `.stories/done/19-deploy-build-cache/spec.md:17` (an STS ARN in a quoted error), against `SECURITY_RULES.md` rule 4, and no issue plans its removal or the history decision. | 2026-11-22 needs the user (3195fb2): an important security finding with a history-rewrite choice (scrub the file in HEAD only, or rewrite history too), the owner's to decide. 2026-11-22 needs the user (cb1c3bc): still there after round 1's review; the scrub and the history choice are still the owner's. |
| R2 | resolved | completeness | 03 | Issue 03's "tests pass in `scripts/check`" has no carrier: it names no language or test location, and `scripts/check` runs only the lambda cargo tests and the cdk pytest. | 2026-11-22: issue 03 now names Python (stdlib) and `cdk/tests/test_pii_scan.py`, run by `scripts/check`'s existing cdk pytest. |
| R3 | resolved | completeness | spec.md, 03 | The allowlist has no path or format (exact values, regexes or path:line), unlike the literal list, whose path is an explicit argument. | 2026-11-22: spec "What it ignores" and issue 03 now name `pii-allowlist.txt` at the repo root: one case-insensitive regex per line, against the whole value, with a `# reason`. |
| R4 | resolved | completeness | spec.md, 03 | "What it reads" covers blobs and commit messages only, not author/committer names and emails, tag messages or ref names; commit `0bfe291` (on `production`) has a personal name as author, so the scan as specified would miss it. | 2026-11-22: spec "What it reads"/"What it prints" and issue 03 now cover author/committer names and emails, tag messages and taggers, and ref names, with a test; the author name the scan will find goes to 02's judgement with the rest. |
| R5 | resolved | completeness | spec.md, 03 | "Never, either" lists postal addresses and support case IDs, but no generic pattern covers them and the spec doesn't say the literal list must hold them. | 2026-11-22: spec "What it looks for" now says the owner's literal list holds the name and usernames, the postal address parts, private emails and phones, and the support case IDs, which no generic pattern finds reliably. |
| R6 | resolved | consistency | spec.md, 03 | "What it ignores" names only `000000000000`, `.invalid`/`example.*` and the co-author address, but the tree also holds fixtures `123456789012`, `210987654321`, `777777777777` and zone ID `Z00000000000000000000`, so 03's first run won't be the expected short report. | 2026-11-22: spec "What it ignores" now names the fixtures' IDs found in the tree, the zone ID fixture and the relay author address, so 03's first run starts short. |
| R7 | resolved | consistency | spec.md | The last Clarifications row still says "a normal story with two tickets"; `issues/` has three. | 2026-11-22: the Clarifications row now notes three tickets since issue 03, the way the launch row notes its change. |
| R8 | open | consistency | 01, spec.md | LICENSE's "owner as copyright holder" doesn't say name or GitHub handle, while "Never, either" forbids publishing the owner's name. | 2026-11-22 needs the user (9a1bdd8): whether LICENSE names the owner or the handle `gratinierer` is the owner's decision, and the spec forbids publishing the name. 2026-11-22 needs the user (18373ac): still undecided after round 1's review; the copyright line's name or handle is the owner's. |
| R9 | resolved | factual | 01 | Issue 01 credits the Silkscreen font to Jason Kottke; `website/fonts/OFL.txt:1` says "The Silkscreen Project Authors". | 2026-11-22: issue 01 now credits "The Silkscreen Project Authors", as `website/fonts/OFL.txt:1` does. |
| R10 | resolved | staleness | spec.md, 02 | Goal and issue 02 cite `.stories/4-life-in-the-world/` and its "step 8"; the story is in `.stories/done/4-life-in-the-world/`, where step 8 is now "List sfab where agent builders look". | 2026-11-22: spec Goal and issue 02 now cite `.stories/done/4-life-in-the-world/`, and 02 points at its checklist's step 1 instead of the old step 8. |
| R11 | resolved | consistency | spec.md, 03 | "What it looks for" puts the owner's usernames on the literal list, but the handle `gratinierer` is public (repo URLs in `openapi/sfab.yaml`, `website/llms.txt`, nearly every commit's author) and literals bypass the allowlist, so the spec must say which usernames are public and stay off the list. | 2026-11-22: spec "What it looks for" now limits the list to private usernames and says the public handle `gratinierer` stays off it. |
| R12 | open | completeness | spec.md, 03 | "What it ignores" covers only `.invalid`/`example.*` and the co-author address, but the tree also holds `.test` and `.example` addresses (`abuse@example.test`, `abuse@sfab.example`), the public site's `@botrefugium.com` contacts, and history has GitHub's noreply addresses, so 03's first run still won't be short. | |
| R13 | open | staleness | spec.md | "What gets published" cites `.stories/18-deploy-from-codebuild/`, which now lives in `.stories/done/18-deploy-from-codebuild/` (step 4 of the same spec already uses that path). | |
