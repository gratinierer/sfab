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
whole story can be implemented in one run without breaking them: a repo skill, `/implement-story <n>`, copied from
it and changed as "Decisions" says. The decisions came from a grilling session with the owner on 2026-11-09.

## The clashes

1. **Branch.** It creates its own integration branch. `docs/agents/commit-policy.md` wants work on the story's
   `<n>-<slug>` branch from `gh issue develop`, and, outside the review skills, never commits or creates a branch
   without the owner's instruction.
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

## Decisions

The open questions this story first recorded are answered here.

- **A repo copy, under a new name.** `/implement-spec` is copied to `.claude/skills/implement-story/` and changed
  there, rather than wrapped. `.claude/skills/VENDORED.md` gets its row and a "Changes from upstream" entry. The
  new name says it works on a `.stories/` story; it doesn't shadow the global `implement-spec`.
- **The story's branch is the integration branch.** The skill creates no branch. It runs on the story's
  `<n>-<slug>` branch, or on a branch the owner has ruled the story onto, as `docs/agents/commit-policy.md` "Which
  number a commit carries" allows, and otherwise stops and asks. Each implementer works in its own worktree on a
  branch off it, and a merger sub-agent merges that back.
- **The start gate.** Before anything else it applies `AGENTS.md` "Before implementing a story": no `review.md`,
  no `Kind = review` row, an `open` finding, or a stale review stops it and asks.
- **The start commit is recorded.** Each run appends a line, `- <date> run started at <sha>`, to the story's
  `implementation.md`, with `<sha>` the branch's `HEAD` before that line's own commit, and commits it. The file
  only grows, so it keeps every run. A resumed run counts issues in `issues/done/`, or already merged, as finished.
- **The review base.** The review starts after the newest `code-review` journal row's reviewed `HEAD` in the story's
  `code-review.md`, or at the first run-log line's commit when there is none, so a resumed run reviews only what no
  review has covered, and still reaches every issue's commits. On a shared branch, the range can hold other stories'
  commits: the skill names the story's own (subjects ending `(#<n>)`) to the review, and the rest as out of scope.
  The review names every open issue of the story whose commits fall in the range, not only this run's.
- **One implementer at a time.** The frontier is worked one ticket after another, not in parallel: `scripts/check`
  takes minutes and every checkout shares `lambda/target/`, so parallel builds would only wait on Cargo's lock or
  build cold. Each ticket still gets a fresh sub-agent, its own worktree and `/tdd`.
- **Commits follow the policy.** Implementer and merger commits end their subject with `(#<n>)`, name the issue in
  the body ("Issue NN of story N"), and carry the trailers of `docs/agents/commit-policy.md`. `/code-review` finds
  the issues and the author model from exactly these. Each ticket passes `scripts/check` before it merges.
- **A skill that commits may commit.** `docs/agents/commit-policy.md` generalizes its review-skill exception: running
  any skill whose process includes commits is the explicit ask for that skill's own commits, and for the
  short-lived branches its process makes, such as one per ticket. Pushing still needs an explicit ask. A new ADR
  amends `docs/adr/0006-review-skills-commit-by-default.md`.
- **Every dependency is in the repo.** A skill under `.claude/skills/` calls only skills that are also there, so
  `tdd` is vendored with `implement-story`, and the rule is written down next to ADR 0005's.
- **Questions up front.** Whatever the end-of-run `/code-review ... auto` would ask before its loop (reviewer model,
  branch ruling, uncommitted story documents, a failing baseline `scripts/check`) is settled in `/implement-story`'s
  own start gate, before any sub-agent runs, so the run isn't stopped at its end.
- **One auto review at the end.** After the last merge, the skill runs this repo's `/code-review` inline through
  the Skill tool, with `<review base> auto 3`: up to three review-and-address rounds, on a model other than the
  code's author, recorded in the story's `code-review.md`, with `/address-code-review-findings` treating the
  findings. This replaces upstream's "fix all issues in one sub-agent". The skill says it means the repo's skill,
  whose arguments are a fixed point and `auto`, not Claude Code's built-in `/code-review`. Neither skill is renamed.
- **Closing.** Issue files move to `issues/done/` only when the auto review ends clean. If it stops on its limit,
  without progress or on a failing check, the skill reports and leaves the issues open. When that closes the
  story's last open issue, it offers to move the story to `.stories/done/` and to run `gh issue close <n>`, and does
  neither without the owner's yes.
- **No automatic pull request.** The tracker closes work by moving files, so the skill ends by reporting the
  branch, its commits and the review outcome. It opens a pull request only when the user asks.
- **Exploration notes** go in the session's scratchpad, outside the repo, and reach each implementer as a path.
- **Workflow docs.** `AGENTS.md` and `docs/agents/issue-tracker.md` each say, where they describe implementing a
  story, that `/implement-story <n>` implements a whole story in one run.
- **This story's own commits** go on `2-identity-api-…`, by the owner's ruling, ending with `(#16)`.

## Out of scope

Changing `/implement-spec` upstream.
