# Run code reviews on a model other than the code's author

Label: ready-for-agent

The repo's `code-review` skill starts each reviewer with a fresh context but on the session's own model, which is
usually the one that wrote the code, so a review shares the author's blind spots. `review-story` already guards
against this for planning reviews ("Reviewer independence"). Filed on 2026-11-04 at the owner's request, before
issue 05's second code review.

## Scope

- In `.claude/skills/code-review/SKILL.md`, a "Reviewer independence" rule like `review-story`'s:
  - Find the code's author from the reviewed commits' `Co-Authored-By:` trailers, or ask when they name no model.
  - Run both reviewers on a model other than the author's, and name the pick in chat. A different model of the
    same vendor is enough.
  - If the commits have several author models, use one that wrote none of them, or say which overlap remains.
  - If the author is unknown, or no other model is available, say so, and continue only on the owner's go-ahead,
    recording the compromise in the journal's `Notes`.
  - The journal's `By` cell records both, such as `claude-sonnet-5-5 (dispatched; code by claude-opus-5-5)`.
- `docs/agents/issue-tracker.md` "Code review file" says the `By` column names both models.
- `.claude/skills/VENDORED.md` lists the change from upstream.

## Done when

- The skill states the rule, and the next `/code-review`, of issue 05, records an author and a different reviewer
  model in its journal row.
