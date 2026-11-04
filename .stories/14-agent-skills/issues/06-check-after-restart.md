# Merge the agentic_containers branches and check after a restart

Label: ready-for-human
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
