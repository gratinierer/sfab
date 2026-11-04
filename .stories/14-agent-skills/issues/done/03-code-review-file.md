# Record code reviews in a file

Blocked by: 02

`spec.md` "Code review file".

## Scope

- The repo's `code-review` skill: identify the story and issues from the commits, and at the end of a run append
  a journal row and its findings to `.stories/<story>/code-review.md`, creating the file from a template when
  it's missing.
- `docs/agents/issue-tracker.md`: a "Code review file" section, and the closing rule in "Closing an issue or
  story".
- A pointer in AGENTS.md beside "Before implementing a story".
- `docs/adr/0005-skills-live-in-the-repo.md`, for keeping the skills in the repo with the repo's copy winning,
  and code reviews gating the close.

## Done when

- A run of the repo's `/code-review` on a small range writes `code-review.md` with a journal row and its
  findings, and chat shows the same two axes.

## Comments

- 2026-11-04: done. The repo's `code-review` finds the story and issues from the commits and, in a new step 6,
  writes a journal row and its findings to `.stories/<story>/code-review.md`, from a template in the skill.
  `docs/agents/issue-tracker.md` has "Code review file" and the closing rule, AGENTS.md has "Before closing an issue"
  and "Skills", and `docs/adr/0005-skills-live-in-the-repo.md` records the decision. Tested by following the skill by
  hand, because the global copy still shadows it until issue 05, on `7124f67..d8ac678` in a throwaway worktree: it
  wrote `code-review.md` with one journal row and 3 spec findings, and the chat showed both axes. Those 3 findings are
  real, so issue 04 carries them into story 2's file.
