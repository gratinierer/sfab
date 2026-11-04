# Record code reviews in a file

Label: ready-for-agent
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
