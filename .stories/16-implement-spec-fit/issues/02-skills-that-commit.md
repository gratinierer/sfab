# A skill whose process commits may commit

Label: ready-for-agent

`spec.md` "A skill that commits may commit": the review skills' exception in `docs/agents/commit-policy.md`
becomes a general rule, so `/implement-story` (issue 01) can commit and make its per-ticket branches unattended.

## Scope

- `docs/agents/commit-policy.md`: the "Exception, the review skills" paragraph becomes a general one. Running a
  skill whose process includes commits is the explicit ask to commit for that skill's own commits, and to create
  and delete the short-lived branches its process describes. It stages only the paths it touched, follows the rest
  of the policy for the number and trailers, never pushes, and `nocommit` turns it off where the skill offers it.
  The four review skills stay named as examples. The "Don't create a new branch" rule in "When asked to commit or
  push" points to the exception for such branches.
- A new ADR, `docs/adr/0010-skills-that-commit-may-commit.md` (or the next free number), records the decision and
  why it generalizes 0006; `docs/adr/0006-review-skills-commit-by-default.md`'s status line says it is amended by
  it. ADR 0006 is otherwise unchanged.
- `AGENTS.md` "Commit policy" says nothing contrary; adjust it if it names only the review skills.

## Done when

- `commit-policy.md` states the general rule, and a skill that commits as part of its process no longer conflicts
  with "never commit unless asked".
- The new ADR exists and ADR 0006 points to it.
