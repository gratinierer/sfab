# Create and protect the `production` branch


The owner's step, `spec.md` "A build": create `production` from `main` on GitHub and protect it, so only the owner
moves it forward and nothing lands there by accident (no force-push, no deletion).

## Done when

- `production` exists on `gratinierer/sfab` and its protection rule is on.

## Comments

- 2026-11-14: `production` created locally from `main` (47da5df). The owner ruled "local is enough": it is not on
  `gratinierer/sfab` yet and has no protection rule, so the first deploy build still needs it pushed and protected.
- 2026-11-17: `production` was pushed to `gratinierer/sfab` on 2026-11-15 (see `2-identity-api#03`), but GitHub Free
  refuses rulesets on private repos, so the protection rule this ticket asked for moved to `12-public-repo#02`,
  which carries it. Story 2 cites this ticket for creating the branch only.
