---
name: implement-story
description: "Implement the result of /to-spec and /to-tickets in code."
disable-model-invocation: true
---

You have been given a spec with associated tickets that describe how to implement it.

In this repo the issue tracker is `docs/agents/issue-tracker.md` and the triage label vocabulary is `docs/agents/triage-labels.md`. Read both.

The goal is the entire spec implemented on the story's own branch (the **integration branch**), with every ticket resolved the way the issue tracker closes work.

The tickets are not a list of steps but a **task graph** with blocking relationships, so there is always a **frontier** of tickets ready to be grabbed.

Start every subagent with a fresh context window.

Keep communication with subagents sparse, mainly through **context pointers** to the spec, tickets, research notes and previous commits. Don't duplicate what a pointer already gives.

**Implementer subagents run one at a time**, never in parallel (step 6).

## Steps

0. **Gate first**, before reading tickets or spawning any subagent: apply `review-code` step 3a to `CODING_STANDARDS.md` and `SECURITY_RULES.md` at the repo root, and **stop** where it stops, reporting every stop together and writing nothing. Keep its warnings for the final report. Then run the step 3b `scripts/check` baseline (red: ask whether to fix first or continue).

1. **Resolve the story** from the argument: a story number (`2`), a `.stories/` directory name (`2-identity-api`) or a path, matching exactly one `.stories/<story>/` directory. If it matches none or several, or there is no argument, ask. **Stop** without a story: this skill is for `.stories/` stories.

2. **Start gate and up-front answers**, still before any sub-agent. Apply "Before implementing a story" in `docs/agents/issue-tracker.md`: stop and ask in any of its three cases (no or empty `review.md`, an open finding, a stale review); write nothing. Then settle at once what `review-code`'s caller mode ("Caller mode") takes, and ask nothing of it again:
   - the **branch**, per step 3 below;
   - the **reviewers' model**: one other than the orchestrator's and the implementers'. If none is available, say so and stop;
   - **uncommitted story documents** (`spec.md`, `issues/**`, `code-review.md`): offer them as one commit, `Commit story <n> documents before review (#<n>)`; if declined, don't start;
   - the **baseline**: the step 0 `scripts/check` result, with a red one's answer kept. Hand `review-code` this result and the story, its issues and the `(#<n>)` commit filter.

3. **Branch.** Create no branch. Run on the current branch when it is the story's own (it has the story's `<n>-` prefix) or the owner has ruled the story onto it, as `docs/agents/commit-policy.md` and `review-code` step 8 describe; otherwise stop and ask. Each implementer works in its own worktree on a branch off it, and a **merger subagent** merges that back.

   **Run log.** With the branch settled, append `- <ISO date> run started at <sha>` to `.stories/<story>/implementation.md`, with `<sha>` the branch's `git rev-parse HEAD` before this line's own commit. Create a missing file with the heading `# <n>: implementation log`, and only ever append: earlier lines stay as written. Commit that file alone, by name, as `Start implementing story <n> (#<n>)`. On a resumed run (the file already has an entry line), issues in `issues/done/` or whose commits are already merged into the branch are **finished**: they leave the frontier and no longer block.

   **Review base.** The reviewed `HEAD` of the newest `code-review` row in `code-review.md` (the `Reviewed` column's `<issues> @ <fixed-point>..<HEAD>`, its `<HEAD>`) or, with no such row, the `<sha>` of the run log's first entry line (not its heading). A resumed run reviews only what no `code-review` row covers. The **open issues of earlier runs** are the story's open issues with a ticket commit since that first entry line's `<sha>`, found with `git log <sha>..HEAD --grep='^Issue [0-9]* of story <n>$'`. That body line marks a **ticket commit**; run-log, review and close commits don't carry it. Note both before the first merge.

4. Read the spec and issues for the task graph. List the story's open issues whose `Label:` is not `ready-for-agent` together with the issues they block (`Blocked by`): they are not implemented, and the final report says the story can't close yet.

5. (optional) Use an **exploration subagent** for any exploration the issues need (codebase files, external documentation). It must be able to save files: its markdown notes go under `.scratch/` in the working directory, git-ignored (add it to `.gitignore` if missing) and readable by all later subagents, so **implementer subagents** can focus on implementation.

6. **Work the frontier sequentially**: one `ready-for-agent` issue not skipped in step 4 at a time, never two implementers at once. Parallel worktrees would each build and check cold and contend for CPU and disk, for little gain. Each issue gets a fresh **implementer subagent** in its own worktree, on its own branch off the integration branch. Those branches' commits and the merger's are this skill's own, so running it is the ask for them (pushing is not). Each implementer subagent:
   - confirms its worktree is based on the integration branch before starting, and resets onto it if not;
   - reads `CODING_STANDARDS.md` and `SECURITY_RULES.md` (pointers, not copies) and follows them;
   - calls the Skill tool with `tdd` to build the issue, which it finds by the pointer to its issue file under `.stories/<story>/issues/` (the brief does not copy the issue);
   - **seams**: the issue file stands in for `tdd`'s user confirmation, which an unattended run can't get. It takes the seams the issue implies and names them in the commit body as `Seams: …`; the review's Spec reviewer checks them. When the issue leaves a seam unclear, it stops, commits nothing and reports the unclear seam, and the issue stays open;
   - **commits**: the subject ends `(#<n>)`, the body has `Issue NN of story <n>` and the `Seams:` line, and the trailers are the commit policy's plus a `Co-Authored-By: <model name> <noreply address>` line naming the committing model. `review-code` step 4 finds the author model and issues from them;
   - merges the integration branch tip into its own branch and runs `scripts/check` (if present and not a skeleton; output captured, read only the exit code and on failure the last ~40 lines) before reporting done. With a waived baseline only failures the baseline didn't show count. On a failure it fixes and re-runs once; a second failure stops the issue.

7. When an implementer completes, a **merger subagent** merges its branch to the integration branch, with commits following step 6's commit rule. Then remove that implementer's worktree and branch. A **stopped** issue (unclear seam or second failed check) is not merged: it stays open, its worktree and branch are kept for the final report to point to, and issues blocked by it wait. Report why it stopped.

8. When the frontier changes, pick the next issue that is `ready-for-agent`, not finished or stopped, and not blocked by an issue that is neither merged (in this run or an earlier one) nor in `issues/done/`, and go back to step 6. A stopped issue doesn't end the run: later issues that don't depend on it still run.

9. **Review what was merged.** Runs when the frontier is empty, even with issues still open (blocked by another story, not `ready-for-agent`, or stopped). First check for work to review: a ticket commit (found as in step 3, with the review base for `<sha>`) after the review base. A resumed run that merged nothing new still reviews earlier uncovered merges after that base. With none, report "no ticket commit after the review base", skip the review, leave every issue open and go to step 11. Otherwise call the Skill tool with `review-code <base> auto 3` on the integration branch, passing the five caller-mode inputs of step 2 (story and issues, reviewers' model, baseline result, the `(#<n>)` commit filter and the Spec reviewer's extra line: "Check each ticket commit's `Seams:` line against its issue file's What to build and Acceptance criteria (Scope and Done when in older issues), and report any seam the issue doesn't imply; run-log, review and close commits have none"), with the issues named being the run's own plus the open issues of earlier runs from step 3. It reviews, fixes the findings with `address-code-review-findings`, and reviews the fixes again.

10. **Close.** Close only after `review-code` stops clean (no finding against the reviewed issues `open`): move each merged issue with no `open` finding against it to `issues/done/` through "Closing an issue or story" in `docs/agents/issue-tracker.md`, and commit the moves alone, by name, as `Close implemented issues of story <n> (#<n>)`. After the limit, no progress or a failing check, move nothing and report what the review leaves for the user. Issues not merged (blocked, not `ready-for-agent`, stopped) stay open. When the moves close the story's last open issue, offer to move the story to `.stories/done/` and to run `gh issue close <n>`, and do each only on a yes.

11. **Report**: the branch, its commits, the review outcome, which issues closed and which stay open and why (stopped ones with their worktree and branch), plus any step 0 date warnings. Open no pull request unless asked.

12. **Clean up**: confirm no worktree or branch of a merged implementer is left; step 7 says which are kept.
