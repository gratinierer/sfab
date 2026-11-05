---
name: code-review
description: "Review the changes since a fixed point (commit, branch, tag, or merge-base) along two axes: Standards (does the code follow this repo's documented coding standards?) and Spec (does the code match what the originating issue/spec asked for?). Runs both reviews in parallel sub-agents, reports them side by side, and records the findings in the story's `code-review.md`. Use when the user wants to review a branch, a PR, work-in-progress changes, or asks to \"review since X\". With `auto [X]`, loops review and an unattended `address-code-review-findings` pass up to X times (default 3), committing as it goes and asking only about what needs a human."
---

Two-axis review of the diff between `HEAD` and a fixed point the user supplies:

- **Standards**: does the code conform to this repo's documented coding standards?
- **Spec**: does the code faithfully implement the originating issue / spec?

Both axes run as **parallel sub-agents** so they don't pollute each other's context, then this skill aggregates their findings. Start every sub-agent with a fresh context window.

In this repo the issue tracker is `docs/agents/issue-tracker.md`, with stories and their issues under `.stories/`.

## Process

### 1. Pin the fixed point

Whatever the user said is the fixed point (a commit SHA, branch name, tag, `main`, `HEAD~5`, etc.). If they didn't specify one, ask for it. The keywords `auto`, an optional X after it, and `nocommit` are never the fixed point: `/code-review auto` has none. See "Auto mode".

Capture the diff command once: `git diff <fixed-point>...HEAD` (three-dot, so the comparison is against the merge-base). Also note the list of commits via `git log <fixed-point>..HEAD --oneline`.

Before going further, confirm the fixed point resolves (`git rev-parse <fixed-point>`) and the diff is non-empty. A bad ref or empty diff should fail here, not inside two parallel sub-agents.

### 2. Identify the spec source

In this repo the spec is a story under `.stories/`: its `spec.md`, plus the issue files the change implements,
in `issues/` or `issues/done/`. Find them in this order:

1. The story and issues the user named, such as "issues 04 and 05 of story 2".
2. The story from the commit subjects' `(#<n>)`, which is `.stories/<n>-*/`, and its issues from the commit bodies,
   which name them ("Issue 05 of story 2"), or from the issue files the range moved into `done/`.
3. If the story or its issues are still unclear, or the range spans two stories, ask. Never guess between them.

Note the story and the issue numbers: step 7 records them. If the user says there's no spec, the **Spec**
sub-agent skips and reports "no spec available", and step 7 records the review under the story the commits name.

### 3. Identify the standards sources

Search the repo for every file that documents how code should be written. When `CODING_STANDARDS.md` or `CONTRIBUTING.md` exists, it must be on the list.

On top of whatever the repo documents, the Standards axis always carries the **smell baseline** below: a fixed set of Fowler code smells (_Refactoring_, ch.3) that applies even when a repo documents nothing. Two rules bind it:

- **The repo overrides.** A documented repo standard always wins; where it endorses something the baseline would flag, suppress the smell.
- **Always a judgement call.** Each smell is a labelled heuristic ("possible Feature Envy"), never a hard violation. Like any standard here, skip anything tooling already enforces.

Each smell reads *what it is* → *how to fix*; match it against the diff:

- **Mysterious Name**: a function, variable, or type whose name doesn't reveal what it does or holds. → rename it; if no honest name comes, the design's murky.
- **Duplicated Code**: the same logic shape appears in more than one hunk or file in the change. → extract the shared shape, call it from both.
- **Feature Envy**: a method that reaches into another object's data more than its own. → move the method onto the data it envies.
- **Data Clumps**: the same few fields or params keep travelling together (a type wanting to be born). → bundle them into one type, pass that.
- **Primitive Obsession**: a primitive or string standing in for a domain concept that deserves its own type. → give the concept its own small type.
- **Repeated Switches**: the same `switch`/`if`-cascade on the same type recurs across the change. → replace with polymorphism, or one map both sites share.
- **Shotgun Surgery**: one logical change forces scattered edits across many files in the diff. → gather what changes together into one module.
- **Divergent Change**: one file or module is edited for several unrelated reasons. → split so each module changes for one reason.
- **Speculative Generality**: abstraction, parameters, or hooks added for needs the spec doesn't have. → delete it; inline back until a real need shows.
- **Message Chains**: long `a.b().c().d()` navigation the caller shouldn't depend on. → hide the walk behind one method on the first object.
- **Middle Man**: a class or function that mostly just delegates onward. → cut it, call the real target direct.
- **Refused Bequest**: a subclass or implementer that ignores or overrides most of what it inherits. → drop the inheritance, use composition.

### 4. Pick the reviewers' model

A model reviewing code it wrote shares the blind spots that produced it, so both reviewers run on a model other
than the code's author, as `review-story` does for planning reviews.

1. **Find the author** from the reviewed commits' `Co-Authored-By:` lines: `git log <fixed-point>..HEAD --format=%b
   | grep -i '^Co-Authored-By:' | sort | uniq -c`. A line such as `Co-Authored-By: Claude Opus 5.5 <...>` names the
   model. Match the line rather than `%(trailers)`: older commits end with a `Created with AI support` line, which has
   no colon, so git doesn't parse their last paragraph as trailers. If no line names a model, ask who wrote the code.
2. **Pick a different model** for both sub-agents and name it in chat. A different model of the same vendor is
   enough, such as Sonnet or Fable for code by Opus. If the commits have several author models, pick one that wrote
   none of them, or say which overlap remains.
3. **In `auto` rounds 2+**, the range is the orchestrator's own fix commits, so the orchestrating model counts as an
   author too: pick a model other than both. That holds even when a human wrote the original code.
4. **If the author is unknown, or no other model is available**, say which of the two it is, and continue only on
   the user's go-ahead. Record the compromise in the journal's `Notes` in step 7, so a later reader knows the
   review wasn't independent. A human author needs no other model.

### 5. Spawn both sub-agents in parallel

Issue both sub-agent calls together, in the foreground, on the model from step 4, and aggregate the reports they
return.

**Standards sub-agent prompt** should include:

- The full diff command and commit list.
- The list of standards-source files you found in step 3, **plus the smell baseline from step 3** pasted in full (the sub-agent has no other access to it).
- The brief: "Report, per file/hunk where relevant, (a) every place the diff violates a documented standard: cite the standard (file + the rule); and (b) any baseline smell you spot: name it and quote the hunk. Distinguish hard violations from judgement calls: documented-standard breaches can be hard, but baseline smells are always judgement calls, and a documented repo standard overrides the baseline. Skip anything tooling enforces. Under 400 words."

**Spec sub-agent prompt** should include:

- The diff command and commit list.
- The path or fetched contents of the spec.
- The brief: "Report: (a) requirements the spec asked for that are missing or partial; (b) behaviour in the diff that wasn't asked for (scope creep); (c) requirements that look implemented but where the implementation looks wrong. Quote the spec line for each finding. Under 400 words."

If the spec is missing, skip the Spec sub-agent and note this in the final report.

### 6. Aggregate

Present the two reports under `## Standards` and `## Spec` headings, verbatim or lightly cleaned. Do **not** merge or rerank findings, because the two axes are deliberately separate (see _Why two axes_).

End with a one-line summary: total findings per axis, and the worst issue _within each axis_ (if any). Don't pick a single winner across axes: that's the reranking the separation exists to prevent.

### 7. Record the review

Write the findings to `.stories/<story>/code-review.md` before discussing them, so they outlive the conversation. The
conventions are in `docs/agents/issue-tracker.md` "Code review file". If the file doesn't exist, create it from the
template under "Record template".

- **One journal row** for this run: `Date` is the ISO datetime, `Kind` is `code-review`, `Reviewed` names the issues
  and the range as `<issues> @ <fixed-point short sha>..<HEAD short sha>`, such as `04, 05 @ 862ddba..e2b740c`, `By`
  names the reviewers' model and the code's author, such as `claude-sonnet-5-5 (dispatched; code by claude-opus-5-5)`,
  `Findings` is `<n> new: <s> standards, <p> spec`, and `Notes` records any independence compromise from step 4,
  such as "reviewers on the author's model, by the user's go-ahead".
- **One findings row per finding**, appended in report order, standards first: the next free `C<n>`, status `open`,
  axis `standards` or `spec`, `Affects` as the issue and the file (`05, cdk/app.py:17`), one sentence of
  `Description` that starts with "Possible <smell>:" for a judgement call, and `Comments` of `<date>: found.`
- Leave out what the reviewers checked and found sound. It belongs in the chat report, not the table.

Then say in chat that the file was written, and that `/address-code-review-findings` works through the open rows.

### 8. Commit `code-review.md`

Commit it without asking, unless `nocommit` was given: running this skill is the explicit ask, per the exception in
`docs/agents/commit-policy.md`.

- Check the branch per `commit-policy.md` before step 5, not after it. If it has no `<n>-` prefix, or the story isn't
  the branch's own and the owner hasn't ruled that its commits go here, ask then. If `code-review.md` already had
  uncommitted changes, ask then whether to commit them first.
- Stage `code-review.md` alone, by name. The subject is `Code-review issues <issues> and record C<a>-C<b> (#<n>)`,
  such as `Code-review issues 04, 05 and record C12-C15 (#2)`, or `Code-review issues <issues>, no findings (#<n>)`.
  The trailers follow `commit-policy.md`. Never push.

## Auto mode

`/code-review <fixed-point> auto [X]` reviews, addresses the findings with `/address-code-review-findings <story>
auto`, and reviews the fixes again, unattended, asking the user only about what needs them. The decisions behind it
are in `docs/adr/0006-review-skills-commit-by-default.md`.

### Arguments

- **X** is the maximum number of address passes, an integer of at least 1, default 3. The loop runs
  `review, (address, review) × up to X`, so it always ends on a review: the last fixes get reviewed, and the closing
  gate can be met.
- `auto 0`, `auto -1`, `auto foo`: invalid, asked about up front.
- A branch literally named `auto` is given as `refs/heads/auto`.
- `auto nocommit` is refused up front: the fix range and the progress check need commits. Plain `nocommit` works for a
  single pass.

### Up front, then unattended

Ask everything the loop could need before round 1, in one go, and never mid-loop:

- a missing fixed point, an invalid X or `auto nocommit`;
- the story and issues, when step 2 can't settle them;
- the branch, as in step 8, including the owner's ruling when the story isn't the branch's own. Without it the loop
  doesn't start;
- the reviewers' model: one other than the code's author **and** the orchestrator's, as in step 4. If none is
  available, say so and stop;
- uncommitted story documents (`spec.md`, `issues/**`, `code-review.md`): offer to commit them as one commit,
  `Commit story <n> documents before review (#<n>)`. If the user declines, the loop doesn't start;
- the **baseline**: run `scripts/check` once. If it already fails, ask: fix it first, or run without check gating. In
  the latter case the address passes don't revert a finding for a failure the baseline already showed.

### The loop

1. **Review**: steps 1–8 on the current range, with both sub-agents on the chosen model. Round 1's range starts at
   the given fixed point. Later rounds review only the fix commits, `<previous review's HEAD>..HEAD`, and their
   journal rows still name round 1's issues, as `<issues> @ <prev>..<HEAD>`, so the closing gate sees a row reaching
   each issue's last commit. `<HEAD>` is the one reviewed, before the review's own commit; that commit touches only
   `.stories/`, so the next range including it is harmless.
2. **Stop, clean**, if no finding against the reviewed issues is `open`.
3. **Stop, limit**, if X address passes have run.
4. **Stop, no progress**, without an address pass, if every open finding against the reviewed issues would be skipped
   as still waiting (see "Skip what still waits" in `address-code-review-findings`): the pass could only repeat
   itself.
5. **Address**: run `/address-code-review-findings <story> <issues> auto` inline, through the Skill tool, so this
   session stays the only writer of `code-review.md` and of the commits. It commits per finding and hands back its
   outcomes, whether it edited `spec.md` or `issues/`, and whether it made **progress**: changed a file other than
   `code-review.md` and kept the change.
6. **Stop, no progress**, if it made none. Everything left needs the user, so another review would repeat itself.
7. **Check**: run `scripts/check`. If it fails beyond what the baseline showed, stop after the next review and list
   the failure as needing the user, rather than addressing on a broken tree.
8. Go to 1.

The address pass leaves open whatever needs the user (design or product decisions the story doesn't record,
ambiguity, waivers, destructive or out-of-scope fixes, important findings, files with unrelated uncommitted edits,
fixes whose check failed) and carries on.

### Final report

One chat summary, in this order:

1. Findings that need the user, each with why, including rows step 4 found still waiting.
2. Rounds run, and the stop reason: clean, limit, no progress, or a failing check.
3. The commits made, as short shas with subjects.
4. Every autonomous decision: finding, outcome, why.
5. Story issues the loop filed, as unreviewed, and, if an address pass edited `spec.md` or `issues/`, that the story's
   `review.md` is now stale for the implementation gate and wants a `/review-story`.

## Record template

```markdown
# <story>: code review

Code reviews of this story's issues. The journal lists every pass: `code-review` runs of `/code-review`, and
`walkthrough` passes of `/address-code-review-findings`. The findings table lists what each run found and what became
of it. Rows are never deleted or renumbered: a fixed finding flips to `resolved`.

Statuses: `open` / `resolved` / `wontfix` / `superseded`.
Axes: `standards` / `spec`.
Conventions: `docs/agents/issue-tracker.md`, section "Code review file".

## Review journal

| Date | Kind | Reviewed | By | Findings | Notes |
|---|---|---|---|---|---|

## Findings

| # | Status | Axis | Affects | Description | Comments |
|---|---|---|---|---|---|
```

## Why two axes

A change can pass one axis and fail the other:

- Code that follows every standard but implements the wrong thing → **Standards pass, Spec fail.**
- Code that does exactly what the issue asked but breaks the project's conventions → **Spec pass, Standards fail.**

Reporting them separately stops one axis from masking the other.
