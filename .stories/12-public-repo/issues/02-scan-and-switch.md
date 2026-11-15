# Scan the history and make the repo public

Label: ready-for-human
Blocked by: 01

`spec.md` "Before and at the switch". It runs as step 8 of `.stories/4-life-in-the-world/` "Launch checklist",
after the URL is published.

## Scope

- Run `gitleaks` over the full history, and stop and decide with the owner on any real finding.
- Turn on private vulnerability reporting, secret scanning and push protection.
- Switch the repo to public.
- Protect `production` with a branch ruleset: restrict deletions, block force pushes. It couldn't be done earlier,
  because GitHub Free offers rulesets only on public repos (`2-identity-api#03`).

## Done when

- The scan reports no real secret, and the result is noted under `## Comments` below.
- The repo is public, with the three settings on.
- `production` has an active ruleset that restricts deletions and blocks force pushes.
