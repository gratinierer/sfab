# Let the repo's skills win over the global links

Label: ready-for-agent
Blocked by: 02

`spec.md` "Skills in the repo" ("The repo's copy wins"). The change is in gratinierer/agentic_containers.

## Scope

- An issue in gratinierer/agentic_containers, labelled `AIassisted` and assigned to the owner.
- In its `poststart.sh`, skip linking a skill into `~/.claude/skills` when a repo under `/workspaces/*/.claude/skills/`
  has one of that name, with a message naming the repo, and remove such a link left by an earlier start.
- Copilot CLI registers the whole global directories with `copilot skill add`, and `skill-loading.instructions.md`
  re-registers them each session when their paths are missing. Check which copy Copilot uses when a repo skill has
  the same name. If the global one wins, change both `poststart.sh` and `skill-loading.instructions.md`, so the
  registration and its self-heal agree on a set that leaves the repo's names out, since `copilot skill add` takes
  only whole directories.
- Work in a clone of gratinierer/agentic_containers in the scratchpad, because `/workspaces/agentic_container` isn't
  writable. A branch for that issue, pushed, for the owner to merge. No merge.

## Done when

- The branch is pushed with the change, and a dry run of the new `poststart.sh` logic against this container lists
  every skill in `.claude/skills/` that also has a global copy as skipped, and links the rest as before.
- The Copilot check's result is under `## Comments`: which copy Copilot CLI uses for a shared name. If it's the
  global one, the branch also changes `skill-loading.instructions.md`.
- The owner's merge, the host checkout's update and the restart are issue 06.
