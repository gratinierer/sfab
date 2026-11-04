# Move the commit policy into the repo

Label: ready-for-agent

`spec.md` "The commit policy in the repo".

## Scope

- `docs/agents/commit-policy.md`, from the owner's global `git-commit-policy.instructions.md`, with the trailer
  `Created with AI support (<tool>)`.
- The rule the owner set for this story: a commit carries the number of the story it serves, which defaults to the
  branch's number. So this story's commits on `2-identity-api-…` end with `(#14)`.
- Point AGENTS.md, `docs/agents/issue-tracker.md` and `docs/agents/github.md` at it instead of the global file.
- AGENTS.md says that `docs/agents/commit-policy.md` wins over any global commit policy a session also loads, so it
  holds from this issue on, before issue 07's deferral line reaches the container.

## Done when

- Outside `.stories/`, no file names `git-commit-policy.instructions.md`, except `docs/agents/commit-policy.md` naming
  it as its origin.
