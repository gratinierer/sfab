# Skills whose process commits may commit

Status: accepted. Amends ADR 0006.

ADR 0006 let the four review skills commit their own work without asking. The rule now covers any skill whose
process includes commits: running it is the explicit ask for that skill's own commits, and for the short-lived
branches and worktrees its process creates and deletes. The first new case is `/implement-story`, which commits a run
log, one branch per ticket in its own worktree, the merges back onto the story's branch and the closing moves.
Pushing still always needs an explicit request.

## Why

- **An unattended run can't stop for a yes per commit.** `/implement-story` works a story's tickets one after another
  through sub-agents; each ticket's branch, commit and merge is part of its process, not a separate decision.
- **A rule per skill doesn't scale.** Listing each committing skill as its own exception would grow the policy with
  every skill; one rule, bounded to the skill's own commits and branches, keeps it short.

## Consequences

- `docs/agents/commit-policy.md` "Exception, skills that commit" states the general rule and names the skills it
  covers today.
- A skill's commits still follow the policy's number and trailers and stage only the paths it touched.
- Branches a skill creates are its own to delete; a ticket that stopped keeps its worktree and branch for the owner.
