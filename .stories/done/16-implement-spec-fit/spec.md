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
  only grows, so it keeps every run. A resumed run counts issues in `issues/done/`, or whose ticket branch a run merged, as finished.
- **The review base.** The review's fixed point is the first run-log line's commit, and what a review already
  covered is left out per issue, from the ranges the story's `code-review.md` journal rows record
  (`<issues> @ <a>..<b>`): a commit naming an issue is left out when a `code-review` row naming that issue has it
  in its range, and one naming no issue (such as an `Address C<n>` fix) when any `code-review` row has it in its
  range. Rows can cover one issue, start late or end behind an older row, so no single row's `HEAD` is a base. A
  resumed run so reviews only what no review has covered, and still reaches every named issue's commits. On a shared branch, the range can hold other stories'
  commits: the review is told to take only the story's own, subjects ending `(#<n>)`, through `/code-review`'s new
  caller mode (below). The review names every open issue of the story whose ticket branch a run merged since the run log's
  first line, not only those in the range, so a resumed run still names, and can close, an earlier run's issues.
  An issue only committed to by hand isn't named, so the run never closes it, and the filter leaves its commits
  out, reported as left for a manual `/code-review`.
- **Only `ready-for-agent` issues are implemented.** Issues labelled `ready-for-human`, `needs-info` or
  `needs-triage` are listed in the start gate and left open; issues they block wait. The run then reports that the
  story can't close yet.
- **One implementer at a time.** The frontier is worked one ticket after another, not in parallel: every worktree
  builds cold in its own `lambda/target/`, and `scripts/check` takes minutes, so parallel tickets would contend for
  CPU and disk and make harder merges for little gain. Each ticket still gets a fresh sub-agent, its own worktree
  and `/tdd`.
- **The issue file confirms the seams.** `/tdd` confirms the seams under test with the user before any test, which
  an unattended implementer can't. Its brief says the issue file stands in for that: it takes the seams its Scope
  and "Done when" imply, names them in its commit body (a ticket branch without one isn't merged; an issue with nothing to test, such as docs or skill files, writes
  `Seams: none` and why), and the end-of-run review's Spec reviewer checks them. When
  the issue doesn't make the seams clear, the ticket stops and is reported rather than guessed.
- **Commits follow the policy.** Implementer and merger commits end their subject with `(#<n>)`, name the issue in
  the body ("Issue NN of story N"; the merger's `--no-ff` merge also says `Merged by /implement-story`, above the trailers, which is how
  a later run knows a run merged it), and carry the trailers of `docs/agents/commit-policy.md`. `/code-review` finds
  the issues and the author model from exactly these. Each ticket passes `scripts/check` before it merges; when
  the owner waived a failing baseline, only failures the baseline didn't show count.
- **A skill that commits may commit.** `docs/agents/commit-policy.md` generalizes its review-skill exception: running
  any skill whose process includes commits is the explicit ask for that skill's own commits, and for the
  short-lived branches its process makes, such as one per ticket. Pushing still needs an explicit ask. A new ADR
  amends `docs/adr/0006-review-skills-commit-by-default.md`.
- **Every dependency is in the repo.** A skill under `.claude/skills/` calls only skills that are also there, so
  `tdd` is vendored with `implement-story`, and the rule is written in `.claude/skills/VENDORED.md` and `AGENTS.md`
  "Skills", where ADR 0005's rule already lives. The ADR itself isn't edited.
- **A run that can't finish every ticket still reviews.** When the frontier empties with issues still open
  (blocked by another story, not `ready-for-agent`, or stopped after a failed check), the run goes on to the
  review and closing for what was merged, and reports what stays open. A ticket that stopped unmerged is relabelled
  `ready-for-human` with a note, in one commit; that makes the review stale, so the next run asks first.
- **Questions up front.** Whatever the end-of-run `/code-review ... auto` would ask before its loop (reviewer model,
  branch ruling, uncommitted story documents, a failing baseline `scripts/check`) is settled in `/implement-story`'s
  own start gate, before any sub-agent runs, so the run isn't stopped at its end.
- **`/code-review` gains a caller mode.** A calling skill can hand it the story and issues, the reviewers' model,
  the baseline check's result, the answer on uncommitted story documents, a filter to the story's own `(#<n>)` commits, and an extra line for the Spec
  reviewer's brief, with the full commit messages, which carries the seam check. Given those, its auto loop asks
  nothing up front and reviews only the filtered commits. Without them it behaves as today.
- **One auto review at the end.** After the last merge, the skill runs this repo's `/code-review` inline, with
  `<run log's first commit> auto 3` and the caller mode's answers: up to three review-and-address rounds, on a model other than
  the code's author, recorded in the story's `code-review.md`, with `/address-code-review-findings` treating the
  findings. This replaces upstream's "fix all issues in one sub-agent". Claude Code also ships a built-in
  `code-review`, so the skill reads and follows `.claude/skills/code-review/SKILL.md` by path rather than calling it
  by name through the Skill tool. Neither skill is renamed.
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

## Outcome

Closed on 2026-11-14 without implementing issues 01-03 as written: the owner added `/implement-story`, `tdd`, the
`review-code` caller mode and the general commit-policy exception (ADR 0010) directly. See `.claude/skills/VENDORED.md`.
