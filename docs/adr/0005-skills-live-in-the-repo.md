# Skills live in the repo, and code reviews gate closing

Status: accepted

The agent skills this repo's workflow uses are kept in `.claude/skills/`, as frozen copies the repo owns, and the
repo's copy wins over any global skill of the same name. Every code review is recorded in the story's
`code-review.md`, and an issue that changes code can't close until a review covers it and none of its findings is
open. See `.stories/14-agent-skills/`.

## Why

- **The repo should be enough.** The workflow in `docs/agents/` names skills such as `/review-story` and
  `/code-review`. When those lived only in the devcontainer's global links, a session's behaviour depended on what
  that container happened to link in, and a public clone couldn't follow its own docs.
- **Frozen, not linked.** The third-party set is reinstalled from its latest version on each container start, so a
  link can change under the repo. A copy changes only in a commit here, and `.claude/skills/VENDORED.md` records
  where each one came from.
- **Code reviews vanished.** `/code-review` reported only in chat, so its findings were lost with the conversation.
  A file per story keeps them, as `review.md` does for planning reviews.
- **A gate where the code is.** An open finding against an issue blocks closing it, not starting the next one, so
  the review doesn't slow planning and still can't be skipped.

## Consequences

- Claude Code shows a personal skill over a project skill of the same name, so the devcontainer's linking step skips
  any skill the project repo has (story 14, issue 05). The global commit policy, which every session also loads,
  defers to `docs/agents/commit-policy.md` (story 14, issue 07).
- Upstream improvements arrive only when someone copies them in by hand.
- Matt Pocock's skills carry his MIT notice in `.claude/skills/LICENSE-mattpocock`, which the public repo's README
  names.

## Considered and rejected

- **Leaving the skills global:** the docs would keep naming skills the repo doesn't carry.
- **Renaming the copies, such as `sfab-review-story`:** both versions would show up, which invites using the wrong one.
- **Gating the start of the next issue on open code-review findings:** that blocks unrelated work for a finding about
  code that's already written.
