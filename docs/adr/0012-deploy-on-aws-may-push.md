# `/deploy-on-aws` may push and merge

Status: accepted. Amends ADR 0010.

ADR 0010 lets a skill whose process commits make its own commits, but says "Pushing still always needs an explicit
request". `/deploy-on-aws` is the one exception: running it is the explicit instruction to push `main` and to merge
`main` into `production` through a pull request, after asking in interactive mode and without asking in `auto`. It
never force-pushes. Every other skill still never pushes. See `.stories/23-deploy-on-aws-skill/spec.md` "Decisions".

## Why

- **A deploy is a push and a merge.** `production` is what `sfab-deploy` builds (ADR 0009), so a deploy skill that
  couldn't push `main` and move `production` would stop at its first step and leave the hand-run sequence it
  replaces.
- **The owner asks for it by name.** The skill runs only when the owner types `/deploy-on-aws`, and in interactive
  mode it still asks before the push and before the merge. `auto` is the owner choosing not to be asked.
- **One named exception, not a general rule.** Letting every committing skill push would put review and
  implementation commits on GitHub unasked; naming the one skill keeps ADR 0010's rule for all the others.

## Consequences

- `docs/agents/commit-policy.md` "Exception, skills that commit" names `/deploy-on-aws` as the one skill that may
  push and merge, and cites this ADR.
- ADR 0010's Status line reads "Amended by ADR-0012".
- The skill pushes only `main` and merges only into `production`, with a merge commit whose subject carries the
  deployed stories' numbers; it makes no other commits and never rewrites either branch.
