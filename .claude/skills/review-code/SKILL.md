---
name: review-code
description: "Review the changes since a fixed point (commit, branch, tag, or merge-base) along three axes: Standards (does the code follow this repo's documented coding standards?), Spec (does the code match what the originating issue/spec asked for?) and Security (does the code break this repo's security rules?). Runs the three reviews in parallel sub-agents, reports them side by side, and records the findings in the story's `code-review.md`. Use when the user wants to review a branch, a PR, work-in-progress changes, or asks to \"review since X\". With `auto [X]`, loops review and an unattended `address-code-review-findings` pass up to X times (default 3), committing as it goes and asking only about what needs a human."
---

Three-axis review of the diff between `HEAD` and a fixed point the user supplies:

- **Standards**: does the code conform to this repo's documented coding standards?
- **Spec**: does the code faithfully implement the originating issue / spec?
- **Security**: does the code break a rule in `SECURITY_RULES.md`?

All three axes run as **parallel sub-agents** so they don't pollute each other's context, then this skill aggregates their findings. Start every sub-agent with a fresh context window.

The issue tracker is `docs/agents/issue-tracker.md`; stories and issues live under `.stories/`.

## Process

### 1. Pin the fixed point

The fixed point is whatever the user said (a commit SHA, branch name, tag, `main`, `HEAD~5`, etc.); if none, ask. The keywords `auto`, an optional X after it, and `nocommit` are never the fixed point: `/review-code auto` has none. See "Auto mode".

Capture the diff command once: `git diff <fixed-point>...HEAD` (three-dot: against the merge-base), and the commit list via `git log <fixed-point>..HEAD --oneline`.

Confirm the fixed point resolves (`git rev-parse <fixed-point>`) and the diff is non-empty, so a bad ref or empty diff fails here, not inside three sub-agents.

### 2. Identify the spec source

The spec is a story under `.stories/`: its `spec.md`, plus the issue files the change implements, in `issues/` or
`issues/done/`. Find them in this order:

1. The story and issues the user named, such as "issues 04 and 05 of story 2".
2. The story from the commit subjects' `(#<n>)` suffix (`.stories/<n>-*/`), and its issues from the commit bodies
   ("Issue 05 of story 2") or from the issue files the range moved into `done/`.
3. If still unclear, or the range spans two stories, ask. Never guess between them.

Note the story and issue numbers for step 7. If the user says there's no spec, the **Spec** sub-agent skips and
reports "no spec available", and step 7 records the review under the story the commits name.

### 3. Identify the standards sources

Search the repo for every file that documents how code should be written, except `SECURITY_RULES.md`, which goes only to the Security reviewer so a rule is never reviewed on two axes. When `CODING_STANDARDS.md` or `CONTRIBUTING.md` exists, it must be on the list.

The Standards axis also always carries the **smell baseline** below: a fixed set of Fowler code smells (_Refactoring_, ch.3), applied even when a repo documents nothing. Two rules bind it:

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

### 3a. Gate on the two standards files

Runs in every mode, before step 4 and before any sub-agent is spawned, so a refusal costs no sub-agent tokens. The
gate is always on, with no opt-out, and both thresholds are fixed here. Check `CODING_STANDARDS.md` and
`SECURITY_RULES.md`, each at the repo root (fixed paths, no search), one at a time. Each carries one header line,
`Last reviewed: YYYY-MM-DD`. Only the user sets it; **this skill never edits `Last reviewed`**, nor either file.
A "review" is the user reading the file, fixing it if needed, and setting the date to today.

1. **Missing**: the file doesn't exist. **Stop.**
2. **Empty**: nothing in it besides headings, blank lines, HTML comments and the header line; no minimum number of
   rules. Test: `perl -0pe 's/<!--.*?-->//gs' <file> | grep -vE '^\s*(#.*|Last reviewed:.*)?$' | grep -q .`
   fails. **Stop.**
3. **Header**: read it with this one line, never the whole file for this check:
   `d=$(grep -m1 -oP '^Last reviewed:\s*\K\d{4}-\d{2}-\d{2}(?=\s*$)' <file>)`. **Stop** if there's no match (trailing text after the date counts as none), if the match
   isn't a real calendar date (`[ "$(date -d "$d" +%F)" = "$d" ]` fails: `2026-02-31` and `YYYY-MM-DD` are
   unparsable), or if the date is in the future. Git commit dates are not used.
4. **Age**, in whole days against today: `age=$(( ($(date -u -d "$(date +%F)" +%s) - $(date -u -d "$d" +%s)) / 86400 ))`.
   A negative age is the future date of rule 3.
   - `age` 0 to 30: pass.
   - `age` 31 to 40 (more than 30 days): **warn and continue**.
   - `age` 41 or more (more than 40 days): **stop.**

Run both files, then report every stop together. Messages, with `<file>` and `<date>` filled in:

- Missing: `<file> is missing. /review-code needs it. Create it with the rules for this repo and set "Last reviewed: <today>", then rerun.`
- Empty: `<file> has no content. Write this repo's rules into it and set "Last reviewed: <today>", then rerun.`
- Header: `<file> has no valid "Last reviewed: YYYY-MM-DD" line (found: <none | candidate>), or its date is in the future. Read the file, fix it if needed, and set the line to today's date, then rerun.`
- Over 40 days: `<file> was last reviewed on <date>, <age> days ago (limit 40). Read it, update it if needed, and set "Last reviewed: <today>", then rerun.`
- Over 30 days, a warning: `<file> was last reviewed on <date>, <age> days ago. It becomes a stop after 40 days.`

Stopping here makes no commit and writes nothing. Keep each warning: step 6 and the "Final report" repeat it.

### 3b. Baseline check

Run the repo's `scripts/check` once per run, not per sub-agent, before the sub-agents, so the review knows whether
the tree is red. Capture the output (`mkdir -p .scratch && scripts/check >.scratch/check.log 2>&1`) and read only the exit code and, on
failure, the last ~40 lines, so a noisy check never floods the context window. In `auto` rounds 2+ the loop's own
check (step 7) is this check, so don't run it twice. Its result is a fact about the tree, never a finding, and goes
in a `check` journal row (step 7).

- **Green** (exit 0): record `green`.
- **Missing** (no `scripts/check`): **warn**, "no scripts/check, so nothing is checked", and continue. Record `missing`.
- **Skeleton** (its output says it is an unfilled skeleton): **warn**, "scripts/check is an unfilled skeleton, so
  nothing is checked", and continue. Record `skeleton, nothing checked`. A skeleton is never green.
- **Red** (non-zero): ask, "fix it first, or review without check gating". On "fix first", stop: no commit, nothing
  written. Otherwise continue and record `red at baseline, user chose to continue`. In `auto` this is the up-front
  question.

Keep each warning: step 6 and the "Final report" repeat it.

### 4. Pick the reviewers' model

A model reviewing its own code shares the blind spots that produced it, so all three reviewers run on a model other
than the code's author, as in `review-story`.

1. **Find the author** from the reviewed commits' `Co-Authored-By:` lines: `git log <fixed-point>..HEAD --format=%b
   | grep -i '^Co-Authored-By:' | sort | uniq -c`. A line such as `Co-Authored-By: Claude Opus 5.5 <...>` names the
   model. Match the line, not `%(trailers)`: older commits end with a `Created with AI support` line without a
   colon, so git doesn't parse their last paragraph as trailers. If no line names a model, ask who wrote the code.
2. **Pick a different model** for all three sub-agents and name it in chat. Prefer a different vendor family (code by
   `claude-*` → `gpt-*` or `gemini-*`); if none is available, a different model of the same family is acceptable
   (Sonnet for code by Opus), and `Notes` in step 7 says so. With several author models, pick one that wrote none of
   them, or say which overlap remains.
3. **In `auto` rounds 2+**, the range is the orchestrator's own fix commits, so the orchestrating model counts as an
   author too: pick a model other than both. That holds even when a human wrote the original code.
4. **If the author is unknown, or no other model is available**, say which, and continue only on the user's
   go-ahead. Record the compromise in the journal's `Notes` in step 7, so a later reader knows the review wasn't
   independent. A human author needs no other model.

### 5. Spawn the three sub-agents in parallel

Issue all three sub-agent calls together, in the foreground, on the model from step 4, and aggregate the reports they
return.

**Standards sub-agent prompt** should include:

- The full diff command and commit list.
- The standards-source files from step 3, **plus the smell baseline from step 3** pasted in full (the sub-agent has no other access to it).
- The brief: "Report, per file/hunk where relevant, (a) every place the diff violates a documented standard: cite the standard (file + the rule); and (b) any baseline smell you spot: name it and quote the hunk. Distinguish hard violations from judgement calls: documented-standard breaches can be hard, but baseline smells are always judgement calls, and a documented repo standard overrides the baseline. Skip anything tooling enforces. Under 400 words."

**Spec sub-agent prompt** should include:

- The diff command and commit list.
- The reviewed issue files (full) and the parts of `spec.md` they reference: the cited decision ids and section names, in full.
- An outline of the rest of `spec.md` (headings only) with the file path, not the unreferenced bodies. The sub-agent may open any section whose heading looks relevant, so it can still report requirements no reviewed issue references, but does not read the whole spec by default.
- The brief: "Report: (a) requirements the spec asked for that are missing or partial; (b) behaviour in the diff that wasn't asked for (scope creep); (c) requirements that look implemented but where the implementation looks wrong. Quote the spec line for each finding. Under 400 words."

**Security sub-agent prompt** should include:

- The diff command and commit list.
- `SECURITY_RULES.md` (the path; the sub-agent reads it in full). It has no smell baseline and no judgement-call category.
- The brief: "Review the diff against `SECURITY_RULES.md`. Report only hard findings, each citing the rule it breaks (the rule's text or heading) and quoting the hunk. Skip anything tooling enforces. Under 400 words."

If the spec is missing, skip the Spec sub-agent and note this in the final report.

### 6. Aggregate

Present the three reports under `## Standards`, `## Spec` and `## Security` headings, verbatim or lightly cleaned. Do **not** merge or rerank findings: the three axes are deliberately separate (see _Why three axes_).

End with a one-line summary: total findings per axis, and the worst issue _within each axis_ (if any). Don't pick a single winner across axes: that's the reranking the separation prevents. Repeat any step 3a warning here, as the closing report of a single pass.

### 7. Record the review

Write the findings to `.stories/<story>/code-review.md` before discussing them, so they outlive the conversation.
Conventions: `docs/agents/issue-tracker.md` "Code review file". If the file doesn't exist, create it from "Record template".

- **One journal row** for this run: `Date` is the ISO datetime, `Kind` is `code-review`, `Reviewed` names the issues
  and the range as `<issues> @ <fixed-point short sha>..<HEAD short sha>`, such as `04, 05 @ 862ddba..e2b740c`, `By`
  names the reviewers' model and the code's author, such as `gpt-6-sol (dispatched; code by claude-opus-5-5)`,
  `Findings` is `<n> new: <s> standards, <p> spec, <x> security`, and `Notes` records any step 4 independence
  compromise, such as "reviewers on the author's model, by the user's go-ahead".
- **One `check` journal row** for the step 3b check, written with the review row: `Kind` is `check`, `Reviewed` is
  `@ <HEAD short sha>`, `By` is `scripts/check`, `Findings` is `—`, and `Notes` is the step 3b outcome. It is a fact
  about the tree: the closing gate and the delta rules read only `code-review` rows, so they ignore it. In `auto`
  rounds 2+ it records the loop's step 7 check of the fixes, at the same `<HEAD>`.
- **One findings row per finding**, appended in report order (standards, spec, security): the next free `C<n>`, status
  `open`, axis `standards`, `spec` or `security`, `Affects` as the issue and the file (`05, cdk/app.py:17`), one
  sentence of `Description` that starts with "Possible <smell>:" for a judgement call, and `Comments` of `<date>: found.`
- Leave out what the reviewers found sound: it belongs in the chat report, not the table.

Then say in chat that the file was written, and that `/address-code-review-findings` works through the open rows.

### 8. Commit `code-review.md`

Commit it without asking, unless `nocommit` was given: running this skill is the explicit ask, per the exception in
`docs/agents/commit-policy.md`.

- Check the branch per `docs/agents/commit-policy.md` before step 5, not after it. If it has no `<n>-` prefix, or
  the story isn't the branch's own and the owner hasn't ruled that its commits go here, ask then.
  If `code-review.md` already had uncommitted changes, ask then whether to commit them first.
- Stage `code-review.md` alone, by name. The subject is `Code-review issues <issues> and record C<a>-C<b> (#<n>)`,
  such as `Code-review issues 04, 05 and record C12-C15 (#2)`, or `Code-review issues <issues>, no findings (#<n>)`.
  The trailers follow `docs/agents/commit-policy.md`. Never push.

## Auto mode

`/review-code <fixed-point> auto [X]` reviews, addresses the findings with `/address-code-review-findings <story>
auto`, and reviews the fixes again, unattended, asking only about what needs the user.

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
- the gate on the two standards files, step 3a: a stop ends the run before round 1; a warning is kept for the final report;
- the story and issues, when step 2 can't settle them;
- the branch, as in step 8, including the owner's ruling when the story isn't the branch's own. Without it the loop
  doesn't start;
- the reviewers' model: one other than the code's author **and** the orchestrator's, as in step 4. If none is
  available, say so and stop;
- uncommitted story documents (`spec.md`, `issues/**`, `code-review.md`): offer to commit them as one commit,
  `Commit story <n> documents before review (#<n>)`. If the user declines, the loop doesn't start;
- the **baseline**: the step 3b check. In `auto`, a red baseline is asked here, never mid-loop. If the user chooses to run without check gating, the address passes don't revert a finding for a failure the baseline already showed.

### Caller mode

A calling skill, such as `implement-story`, can hand over the story and issues, the reviewers' model, the baseline
result, a `(#<n>)` commit filter and an extra line for the Spec reviewer. Then the loop asks nothing up front and
reviews only the filtered commits; see [caller-mode.md](caller-mode.md). Without them, no step changes.

### The loop

1. **Review**: steps 1–8 on the current range, with all three sub-agents on the chosen model. Round 1's range starts at
   the given fixed point. Later rounds review only the fix commits, `<previous review's HEAD>..HEAD`, and their
   journal rows still name round 1's issues, as `<issues> @ <prev>..<HEAD>`, so the closing gate sees a row reaching
   each issue's last commit. `<HEAD>` is the one reviewed, before the review's own commit (it touches only
   `.stories/`, so the next range including it is harmless).
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
7. **Check**: run the repo's `scripts/check`, if it has one, with output captured as in step 3b. If it fails beyond
   what the baseline showed, stop after the next review and list the failure as needing the user, rather than
   addressing on a broken tree.
8. Go to 1.

The address pass leaves open whatever needs the user (design or product decisions the story doesn't record,
ambiguity, waivers, destructive or out-of-scope fixes, important findings, files with unrelated uncommitted edits,
fixes whose check failed) and carries on.

### Final report

One chat summary, in this order:

0. Any step 3a warning, naming the file and its `Last reviewed` date, and any step 3b warning (missing or skeleton `scripts/check`).
1. Findings that need the user, each with why, including rows step 4 found still waiting.
2. Rounds run, and the stop reason: clean, limit, no progress, or a failing check.
3. The commits made, as short shas with subjects.
4. Every autonomous decision: finding, outcome, why.
5. Story issues the loop filed, as unreviewed, and, if an address pass edited `spec.md` or `issues/`, that the story's
   `review.md` is now stale for the implementation gate and wants a `/review-story`.

## Record template

```markdown
# <story>: code review

Code reviews of this story's issues. The journal lists every pass: `code-review` runs of `/review-code`,
`check` rows for the `scripts/check` run before each review, and `walkthrough` passes of
`/address-code-review-findings`. The findings table lists what each run found and what became of it. Rows are never
deleted or renumbered: a fixed finding flips to `resolved`.

Statuses: `open` / `resolved` / `wontfix` / `superseded`.
Axes: `standards` / `spec` / `security`.
Conventions: `docs/agents/issue-tracker.md`, section "Code review file".

## Review journal

| Date | Kind | Reviewed | By | Findings | Notes |
|---|---|---|---|---|---|

## Findings

| # | Status | Axis | Affects | Description | Comments |
|---|---|---|---|---|---|
```

## Why three axes

A change can pass one axis and fail another:

- Code that follows every standard but implements the wrong thing → **Standards pass, Spec fail.**
- Code that does exactly what the issue asked but breaks the project's conventions → **Spec pass, Standards fail.**
- Code that is clean and does what was asked but breaks a security rule → **Standards and Spec pass, Security fail.**

Reporting them separately stops one axis from masking another.
