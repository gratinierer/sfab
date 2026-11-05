# Make /implement-spec fit this repo's rules

GitHub issue: [gratinierer/sfab#16](https://github.com/gratinierer/sfab/issues/16)
Authored: 2026-11-05 by claude-opus-5-5

## Goal

`/implement-spec`, a global skill from Matt Pocock's set (`~/.claude/skills/implement-spec/`), implements a whole
spec: it treats the tickets as a task graph, runs a fresh background sub-agent per ready ticket in its own git
worktree, each using `/tdd`, merges each into one integration branch through a merger sub-agent, runs one code
review at the end and fixes its findings in one more sub-agent, then closes the tickets. It doesn't call
`/implement`.

Used on a `.stories/` story today, four of its steps clash with this repo's rules. This story makes it fit, so a
whole story can be implemented in one run without breaking them. Nothing is decided yet: this records the problem
to solve later.

## The clashes

1. **Branch.** It creates its own integration branch. `docs/agents/commit-policy.md` wants work on the story's
   `<n>-<slug>` branch from `gh issue develop`, and never creates a branch without the owner's instruction.
2. **Review record.** Its end-of-run code review must be this repo's `/code-review`, so the findings land in the
   story's `code-review.md`, and its range must reach every issue's last commit, or the closing gate in
   `docs/agents/issue-tracker.md` "Closing an issue or story" isn't met. Its "fix all issues in one sub-agent"
   step must go through `/address-code-review-findings`, so each finding's outcome is recorded.
3. **Reviewer independence.** Sub-agents default to the orchestrator's model, which then wrote all the code, so
   the end review must be dispatched to another model, as `/code-review` already does.
4. **Closing.** "Resolve each ticket the way the issue tracker closes work" means moving the files to
   `issues/done/`, which is allowed only after the code-review gate passes and with no finding still `open`.

It also has to respect the start-of-story gate in `AGENTS.md` "Before implementing a story": a stale or unreviewed
`review.md`, or open findings, stop it before any sub-agent starts.

## Open questions

- Vendor a repo copy under `.claude/skills/`, which wins over the global one (`docs/adr/0005-skills-live-in-the-repo.md`),
  or wrap it in a repo skill that calls it with the rules stated?
- Integration branch: the story's branch itself, or a child branch merged back into it?
- One `/code-review` at the end for all issues, or one per issue as each merges, so issues close as they finish?
- Do worktrees and parallel sub-agents fit a container where `scripts/check` takes minutes and shares
  `lambda/target/`?

## Out of scope

Changing `/implement-spec` upstream.
