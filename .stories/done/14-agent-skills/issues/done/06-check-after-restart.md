# Merge the agentic_containers branches and check after a restart

Blocked by: 05, 07

The owner's step after issues 05 and 07, whose branches are in gratinierer/agentic_containers:

1. Merge both branches on GitHub.
2. Update the host checkout behind `/workspaces/agentic_container` to the merged `main`, because `poststart.sh`
   and the policy link in `~/.claude/rules` read from that checkout, not from GitHub.
3. Restart the container.

## Done when

- `~/.claude/skills` has no link for any skill in `.claude/skills/`, Claude Code lists the repo's copies, and Copilot
  CLI lists each repo skill once, as the repo's copy.
- The global policy that both tools load, in `~/.claude/rules` and `~/.copilot/instructions`, starts with issue 07's
  line deferring to `docs/agents/commit-policy.md`.

## Comments

- 2026-11-04: done. The owner asked for the merge and pull: agentic_containers PRs #6 and #7 were merged into `main`
  (`68fe775`), the host checkout behind `/workspaces/agentic_container` was fast-forwarded to it, and the owner
  restarted the container. After the restart, `~/.claude/skills` has no link for any of the 14 repo skills, Claude
  Code lists the repo's `code-review`, `copilot skill list` shows each repo skill once under "Project skills", and both
  `~/.claude/rules` and `~/.copilot/instructions` copies of the global policy start with the deferral line.
