# Keep the agent skills in the repo, and record code reviews

GitHub issue: [gratinierer/sfab#14](https://github.com/gratinierer/sfab/issues/14)
Authored: 2026-11-04 by claude-opus-5-5

## Goal

The repo should carry everything an agent needs to work in it, so a session doesn't depend on what the
devcontainer happens to link in, and a code review should leave a record that later sessions can act on.
Today the skills the workflow relies on live outside the repo, symlinked into `~/.claude/skills` from
agentic_containers and a third-party set, and `/code-review` reports only in chat, so its findings vanish
with the conversation.

## Skills in the repo

- **Which:** the 14 skills the workflow uses: `review-story`, `address-story-review-findings`,
  `github-pr-review`, `grilling`, `wayfinder`, `research`, `prototype`, `domain-modeling`, `grill-with-docs`,
  `improve-codebase-architecture`, `codebase-design`, which it calls, `to-tickets`, `code-review`, and the new
  `address-code-review-findings`. Others are added when a doc or a vendored skill starts to rely on one.
- **Where:** `.claude/skills/<name>/SKILL.md`, which Claude Code and GitHub Copilot CLI both read for a repo.
- **Frozen copies:** each is copied once and becomes this repo's own. `.claude/skills/VENDORED.md` records each
  skill's source and version, and upstream changes are pulled in by hand when wanted. For agentic_containers that's
  the commit. For Matt Pocock's skills, `skills-lock.json` records no commit, so it's the lock file's `computedHash`
  and the copy date, plus the mattpocock/skills commit if one matches.
- **Licence:** Matt Pocock's skills are MIT ("Copyright (c) 2026 Matt Pocock"), so his licence text sits
  beside them in `.claude/skills/LICENSE-mattpocock`, and `VENDORED.md` names it. The skills from
  agentic_containers are the owner's.
- **Adapted:** references to `/setup-matt-pocock-skills` point to `docs/agents/issue-tracker.md` instead,
  example keys use this repo's story names such as `2-identity-api`, and "the repo's commit policy" means
  `docs/agents/commit-policy.md`.
- **The repo's copy wins:** Claude Code shows a personal skill over a project skill of the same name, so
  agentic_containers' `poststart.sh` skips linking any skill a project repo under `/workspaces/*/.claude/skills/`
  already has. In a container with several project repos, one repo's copy hides the global skill for the
  others too, which is accepted, because this container opens only sfab. Which copy Copilot CLI prefers isn't
  documented.

## The commit policy in the repo

`docs/agents/commit-policy.md` holds the commit and issue rules that today live in the owner's global
`git-commit-policy.instructions.md`, and AGENTS.md, `docs/agents/*.md` and the skills point to it. Its
AI trailer names the tool, as `Created with AI support (<tool>)`, such as `(Claude Code)`. The global policy stays
loaded in every session, so its first line says to follow a repo's own `docs/agents/commit-policy.md` when there is
one.

## Code review file

- **One per story:** `.stories/<story>/code-review.md`, beside `review.md`, written by the repo's copy of
  `code-review` at the end of each run. Chat still shows both axes.
- **Journal:** `| Date | Kind | Reviewed | By | Findings | Notes |`, where `Kind` is `code-review` or
  `walkthrough`, and `Reviewed` names the issues and the commit range, such as `04, 05 @ 862ddba..e2b740c`.
- **Findings:** `| # | Status | Axis | Affects | Description | Comments |`, with IDs `C1`, `C2` and so on,
  `Axis` as `standards` or `spec`, and the statuses of `review.md`. The two axes stay apart and aren't ranked
  against each other.
- **Closing an issue:** an issue that changes code, meaning any of its commits touch files outside `.stories/`
  and `docs/`, needs a `code-review` journal row that names it and whose range reaches its last such commit.
  While any finding against it is `open`, it can't move to `done/`. Marking a finding `wontfix` or filing it as
  a ticket unblocks it. This doesn't block starting work, which stays `review.md`'s gate.
- **From now on:** the rule applies to issues closed after it lands, starting with `2-identity-api#05`.
  `2-identity-api#04` stays closed, and its findings from the first review are tracked like any other. This
  story's own issues are exempt, because they build the rule, so they close without a code review.
- **`address-code-review-findings`:** works through the open rows of `code-review.md` one at a time, as
  `address-story-review-findings` does for `review.md`: one recommended outcome, the user's decision, an
  optional fix and commit, and a `walkthrough` journal row at the end, with an `auto` mode.
- **Documented** in `docs/agents/issue-tracker.md` "Code review file" and "Closing an issue or story", with a
  pointer in AGENTS.md and `docs/adr/0005-skills-live-in-the-repo.md`.

## Changes to other stories

- `.stories/12-public-repo/`: the README's "Third-party" line also names the vendored skills and
  `.claude/skills/LICENSE-mattpocock`, the `LICENSE` sentence in `spec.md` and the `LICENSE` bullet in issue 01 except
  the vendored skills, and issue 01 gets `Blocked by: 14-agent-skills#02`.
- `.stories/2-identity-api/`: the first `code-review.md` records the review of issues 04 and 05 at
  `862ddba..e2b740c`, with 14 findings, which `address-code-review-findings` then works through before issue
  05 closes.

## Clarifications

### Grilling, 2026-11-04

| Question | Answer |
|---|---|
| One code-review file per story, rather than per issue or in issue comments? | Yes, per story, naming the issues and commit range in each journal row |
| Do open findings block closing the issue they're against? | Yes |
| Is a code review required before closing any issue that changes code? | Yes |
| A separate `address-code-review-findings` skill mirroring the existing one? | Yes |
| Does the repo's `code-review` write the file itself? | Yes, since the MIT licence allows a modified copy |
| Vendor only the 13 skills the workflow uses? | Yes; 14 after review R1 added `codebase-design`, which `improve-codebase-architecture` calls |
| Make the repo's copy win by skipping the global link in `poststart.sh`? | Yes |
| Move the commit policy into the repo, with the tool named in the trailer? | Yes |
| Frozen copies, with sources in `VENDORED.md` and the MIT notice? | Yes |
| Adapt the copies to this repo's docs and story names? | Yes, and remove other projects' example story keys and tracker names from agentic_containers, history included; done on 2026-11-04 by rewriting its 6 commits and force-pushing `main` |
| The file's columns, copied from `review.md`, with `C` IDs and `standards`/`spec` axes? | Yes |
| "Changes code" means touching files outside `.stories/` and `docs/`, and the review's range must reach the last such commit? | Yes |
| Issue 04 stays closed, and the rule starts with issue 05? | Yes |
| Today's review becomes story 2's first `code-review.md`, worked through with the new skill? | Yes |
| The convention in `issue-tracker.md`, a pointer in AGENTS.md, and ADR 0005? | Yes |
| A GitHub issue and a small story, reviewed before building? | Yes |
| An issue and a branch in agentic_containers for the `poststart.sh` change, with no merge? | Yes |
| Do this story's own issues 02 to 04 need a code review before closing (review R5)? | No: the rule starts once issue 03 lands, so the issues that build it are exempt |
| How does the repo's policy beat the global one, which every session still loads (review R11)? | The global policy's first line defers to a repo's own policy, rather than `poststart.sh` skipping it |
| Which branch? | All on story 2's branch, `2-identity-api-birth-evolve-self-and-reclaim`, with `(#14)` on this story's commits |
