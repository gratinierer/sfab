# Make the global commit policy defer to a repo's own

Blocked by: 01

`spec.md` "The commit policy in the repo". The change is in gratinierer/agentic_containers.

## Scope

- An issue in gratinierer/agentic_containers, labelled `AIassisted` and assigned to the owner.
- One line at the top of its `.devcontainer/general_devcontainer/resources/ai/instructions/git-commit-policy.instructions.md`:
  if the repo has its own commit policy, `docs/agents/commit-policy.md`, follow that instead of this file.
- Work in a clone in the scratchpad, because `/workspaces/agentic_container` isn't writable, and push a branch for the
  owner to merge. No merge.

## Done when

- The branch is pushed with that line. The owner's merge, the host checkout's update and the restart are issue 06.

## Comments

- 2026-11-04: done. gratinierer/agentic_containers#5, branch `5-make-the-global-commit-policy-defer-to-a-repos-own` at
  `ac3b0cb`, pushed and not merged. The global policy's first line now says to follow `docs/agents/commit-policy.md`
  when the repo has one.
