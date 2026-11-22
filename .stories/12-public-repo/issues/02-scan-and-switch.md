# Scan the history and make the repo public

Label: ready-for-human
Blocked by: 01, 03

`spec.md` "Before and at the switch". It runs on its own, whenever this story's other tickets are done; since 2026-11-21 it's no longer a step of
`.stories/done/4-life-in-the-world/` "Launch checklist", whose step 1 now says so (the owner).

## Scope

- Run `gitleaks` over the full history, and stop and decide with the owner on any real finding.
- Run `scripts/pii-scan` (issue 03) over the full history with the owner's list of literals, and check its report
  as `spec.md` "The personal data scan" describes. Real findings stop the switch too.
- Decide with the owner whether to rewrite history to drop the account ID that
  `.stories/done/19-deploy-build-cache/spec.md` quoted until 2026-11-22 (review R1): HEAD has `<account>` instead, but
  the commit that added it still holds it, and the scan reports it. Decided 2026-11-22: it stays (see Comments).
- Turn on private vulnerability reporting, secret scanning and push protection.
- Switch the repo to public.
- Protect `production` with a branch ruleset: restrict deletions, block force pushes. It couldn't be done earlier,
  because GitHub Free offers rulesets only on public repos (`2-identity-api#03`).

## Done when

- The scan reports no real secret, and the result is noted under `## Comments` below.
- The personal data scan reports no real finding other than the two the owner accepted on 2026-11-22 (Comments),
  and its result is noted there too, without the values.
- The repo is public, with the three settings on.
- `production` has an active ruleset that restricts deletions and blocks force pushes.

## Comments

- 2026-11-22, early `gitleaks` run (8.30.1, `gitleaks git --log-opts="--all" --redact`), before 01 and 03 were done:
  1623 commits, 5.3 MB, 2 findings, both `generic-api-key` on made-up test fixtures, so no real secret:
  `cdk/tests/test_sfab_config.py:154` at `678f267` (`ICLOUD_TOKEN`, a dummy domain verification value) and
  `lambda/world/federation/tests/daily.rs:180` at `e0b913f` (a hex token in a matcher test). Rerun at the switch,
  since 01 and 03 add commits. The personal data scan waits for `scripts/pii-scan` (03).
- 2026-11-22, early `scripts/pii-scan` run with the owner's list (6 literals), before 01 was done: 2 real findings,
  so the switch stays stopped until the owner decides.
  - The owner's name as the author of `0bfe291`, GitHub's web merge of PR #21, which is `production`'s tip. GitHub
    writes the account's display name into web merges, so a fix needs both a new `production` tip without it
    (force push, before the ruleset) and a display name or merge habit that doesn't bring it back.
    Accepted by the owner on 2026-11-22: it stays, so a rerun still reports this one line (`commit author`, literal 1),
    since literals bypass the allowlist.
  - The account ID of R1 in `.stories/19-deploy-build-cache/spec.md`, first in `6c218cf`, and in 9 later
    commits until HEAD replaced it on 2026-11-22: only a history rewrite of `main` removes it.
    Accepted by the owner on 2026-11-22: no rewrite (it would also leave the ID in PR #21's and #22's refs, which only
    GitHub Support removes). A rerun still reports this one line, and it stays out of `pii-allowlist.txt`, which
    would publish it again in HEAD.
