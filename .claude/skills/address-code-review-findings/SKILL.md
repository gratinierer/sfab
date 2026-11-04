---
name: address-code-review-findings
description: "Walk through a story's open code-review findings one at a time, in dialogue with the user (or, with `auto`, without interaction): decide each one, optionally fix it, and record the outcome in the story's `code-review.md`. Use when asked to go through, work through, or address code-review findings, or before closing an issue whose findings are still open. For producing the findings, use `code-review`; for a story's planning review, use `address-story-review-findings`."
---

# Address Code Review Findings

Work through the open rows of a story's `code-review.md`, one finding at a time, with the user. `code-review`
produces the findings; this skill consumes them. It's the sibling of `address-story-review-findings`, which does the
same for `review.md`, and has the same shape: one item, a decision, an optional change, an optional commit, then the
next item.

The conventions are in `docs/agents/issue-tracker.md`, sections "Code review file" and "Closing an issue or story".

## Modes

- **Interactive** (default): everything below applies as written.
- **`auto`**: the user passes `auto` (e.g. `/address-code-review-findings 2-identity-api auto`, or says "auto"). The
  agent works through the findings without asking, deciding and fixing each one itself. The ask-and-wait rules below
  are lifted *only* where "Auto mode" says so. Running in the CLI's autopilot mode is **not** `auto`: without the
  explicit `auto` argument, ask and wait as usual.

## Finding the story

1. An argument, if given: a story number (`2`), a story directory name (`2-identity-api`), or a path, optionally
   with issue numbers to limit the walk to findings against them (`2-identity-api 05`), and optionally `auto`.
2. Otherwise infer from the current branch: `2-identity-api-birth-evolve-self-and-reclaim` → `.stories/2-*/`.
3. Otherwise, or if more than one story matches, **ask**. Never guess between two stories.

If the story has no `code-review.md`, or it has no `Kind = code-review` journal rows, stop: there is nothing to
address. Recommend `/code-review` first.

## Walking the findings

Take rows with status `open`, in ascending `C<n>` order, one at a time, limited to the named issues if any. Rows that
are `resolved`, `wontfix` or `superseded` are history: skip them, unless the user names one to reopen it.

For each finding:

1. **Present it**: the row, its axis (`standards` or `spec`), and the current code or document it's about. Re-read
   the file at `Affects` before describing it: the code may have moved since the review, and the finding may already
   be dead. A `standards` finding that starts with "Possible" is a judgement call, so say so.
2. **Recommend one outcome** and say why.
3. **Wait for the user.** Even in autopilot mode, ask and wait.

In `auto` mode skip step 3: decide per "Auto mode" and carry on.

### The six outcomes

Every finding ends in exactly one of these, and every one gets a dated note in the row's `Comments`:

| Outcome | Status becomes | What you do |
|---|---|---|
| **Fix now** | `resolved` | Make the change, run the relevant check, then record what you changed and the check. |
| **File it** | `resolved` | Create a story issue `issues/NN-<slug>.md` carrying the work; the comment points at it. |
| **Already fixed** | `resolved` | Verify against the tree first, then record the evidence, such as a commit. |
| **Waive** | `wontfix` | Only on the user's explicit say-so; record their reason, not yours. |
| **Superseded** | `superseded` | Name the decision or change that killed it. |
| **Leave open** | `open` | Add a dated comment saying what is still blocking it, move on. |

A judgement-call smell is often rightly waived, but only the user can waive it. **File it** suits a finding about
work another issue will do, such as a deployed check that belongs to a smoke script.

## Making changes

This skill **may** edit the code, the tests and the story documents. The guard:

- No change without an explicit user instruction **for that specific finding**. A general "yes, fix things" is not
  consent for the next eight.
- Suggest freely; apply only when told. Even in autopilot mode, ask and wait.
- Keep each change minimal and scoped to the finding in hand.
- **Run the cheapest check that covers the change**, such as one test file, and `scripts/check` before the last
  commit. Never mark a finding `resolved` over a failing check.

## Committing

After a finding that produced a change, ask whether to commit it before moving to the next one. Even in autopilot
mode, ask and wait.

- Per-finding commits keep the history bisectable.
- Follow `docs/agents/commit-policy.md` for the issue number and trailers.
- **Never push** without an explicit instruction.

If the user prefers one commit at the end, that is fine, but ask, don't assume it.

## Finishing

When every open finding has been handled, or the user stops:

1. Append **one** journal row to `code-review.md`, with `Kind` = `walkthrough`: `Findings` = `0 new, 3 resolved,
   2 still open`; `Notes` = "walkthrough, not a review pass". A walkthrough changes code without re-reviewing it, so
   it never counts as a `code-review` row.
2. Summarise in chat: what was resolved, what was filed as issues, what is still open and why.
3. **Say what the closing gate still needs.** An issue closes only when no finding against it is open and a
   `code-review` row's range reaches its last commit outside `.stories/` and `docs/`. Fixes made here are new
   commits, so name the range a fresh `/code-review` must cover, such as `e2b740c..HEAD`.
4. If findings were filed as new issues, remind the user those issues are themselves unreviewed.

## Auto mode

`auto` is the user's standing instruction to fix findings without asking, so the "explicit instruction for that
specific finding" guard and the ask-and-wait rules (decision, change, per-finding commit prompt) are satisfied for
the outcomes below. Everything else keeps its guard.

For each open finding, in order, still re-reading the current state first:

- **Fix now / File it / Already fixed**: choose and apply the recommended outcome yourself. Keep each change minimal
  and scoped to the finding. Prefer **File it** when the work belongs to another issue, and **Already fixed** only
  with evidence from the tree.
- **Superseded**: apply when a recorded decision or later change clearly kills the finding; name it.
- **Never waive.** `wontfix` still needs the user's explicit say-so and their reason, judgement-call smells included.
  In auto mode a finding you would waive stays **Leave open**, with a dated comment saying why.
- **Leave open** anything ambiguous, anything that needs a product or design decision the story doesn't record, and
  anything whose fix would be destructive or reach beyond the story and the code it names.
- **Record each outcome** with a dated `Comments` note, and run the cheapest relevant check for any code you changed.
  Report failures; don't mark the finding `resolved` over a failing check.
- **Commits**: don't commit and never push unless the user also said to (e.g. `auto commit`). With `auto commit`,
  make one commit per finding that produced a change, following `docs/agents/commit-policy.md`. Otherwise leave the
  changes uncommitted and say so at the end.

Finish as in "Finishing", with `Notes` = "auto walkthrough, not a review pass", and add to the chat summary an
explicit list of every autonomous decision (finding, outcome, why). Findings left open are listed first.
