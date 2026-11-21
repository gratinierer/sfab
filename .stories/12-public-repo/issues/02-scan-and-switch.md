# Scan the history and make the repo public

Label: ready-for-human
Blocked by: 01, 03

`spec.md` "Before and at the switch". It runs on its own, whenever this story's other tickets are done; since 2026-11-21 it's no longer step 8 of
`.stories/4-life-in-the-world/` "Launch checklist" (the owner).

## Scope

- Run `gitleaks` over the full history, and stop and decide with the owner on any real finding.
- Run `scripts/pii-scan` (issue 03) over the full history with the owner's list of literals, and check its report
  as `spec.md` "The personal data scan" describes. Real findings stop the switch too.
- Turn on private vulnerability reporting, secret scanning and push protection.
- Switch the repo to public.
- Protect `production` with a branch ruleset: restrict deletions, block force pushes. It couldn't be done earlier,
  because GitHub Free offers rulesets only on public repos (`2-identity-api#03`).

## Done when

- The scan reports no real secret, and the result is noted under `## Comments` below.
- The personal data scan reports no real finding, and its result is noted there too, without the values.
- The repo is public, with the three settings on.
- `production` has an active ruleset that restricts deletions and blocks force pushes.
