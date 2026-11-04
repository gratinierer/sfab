# Scan the history and make the repo public

Label: ready-for-human
Blocked by: 01

`spec.md` "Before and at the switch". It runs as step 8 of `.stories/4-life-in-the-world/` "Launch checklist",
after the URL is published.

## Scope

- Run `gitleaks` over the full history, and stop and decide with the owner on any real finding.
- Turn on private vulnerability reporting, secret scanning and push protection.
- Switch the repo to public.

## Done when

- The scan reports no real secret, and the result is noted under `## Comments` below.
- The repo is public, with the three settings on.
