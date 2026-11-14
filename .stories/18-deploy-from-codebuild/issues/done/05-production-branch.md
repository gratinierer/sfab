# Create and protect the `production` branch


The owner's step, `spec.md` "A build": create `production` from `main` on GitHub and protect it, so only the owner
moves it forward and nothing lands there by accident (no force-push, no deletion).

## Done when

- `production` exists on `gratinierer/sfab` and its protection rule is on.

## Comments

- 2026-11-14: `production` created locally from `main` (47da5df). The owner ruled "local is enough": it is not on
  `gratinierer/sfab` yet and has no protection rule, so the first deploy build still needs it pushed and protected.
