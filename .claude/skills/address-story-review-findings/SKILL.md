---
name: address-story-review-findings
description: "Walk through a story's open review findings one at a time, in dialogue with the user (or, with `auto`, without interaction): decide each one, optionally fix it, and record the outcome in the story's `review.md`. Use when asked to go through, work through, or address a story's review findings, or to address them automatically. For producing findings, use `review-story`; for reviewing a diff or PR, use `code-review`, and for its findings `address-code-review-findings`."
---

# Address Story Review Findings

Work through the open rows of a story's `review.md`, one finding at a time,
with the user. `review-story` produces findings; this skill consumes them.

The shape is deliberately the same as `github-pr-review`: one item, a
decision, an optional change, an optional commit, then the next item. The
conversation is the point — a batch pass that waves through nine findings has
done nothing a grep couldn't.

## Modes

- **Interactive** (default): everything below applies as written.
- **`auto`**: the user passes `auto` (e.g. `/address-story-review-findings
  2-identity-api auto`, or says "auto"). The agent works through the
  findings without asking, deciding and fixing each one itself. The ask-and-wait
  rules below are lifted *only* where "Auto mode" says so; see that section
  for what stays off-limits. Running in the CLI's autopilot mode is **not**
  `auto`: without the explicit `auto` argument, ask and wait as usual.

## Finding the story

1. An argument, if given: a story number (`2`), a story directory name (`2-identity-api`), or a path (and
   optionally `auto`, which is not a story).
2. Otherwise infer from the current branch: `2-identity-api-birth-evolve-self-and-reclaim`
   → `.stories/2-*/`.
3. Otherwise, or if more than one story matches, **ask**. Never guess between
   two stories.

If the story has no `review.md`, or it has no `Kind = review` journal rows,
stop: there is
nothing to address. Recommend `/review-story` first.

## Walking the findings

Take rows with status `open`, in ascending `#` order, one at a time. Rows that
are `resolved`, `wontfix` or `superseded` are history — skip them, unless the
user names one explicitly to re-open it.

For each finding:

1. **Present it**: the row, plus any `## Notes on <Rn>` section, plus the
   current state of whatever it is about. Re-read the file before describing
   it — the finding may be days old and already dead.
2. **Recommend one outcome** and say why.
3. **Wait for the user.** Even in autopilot mode, ask and wait.

In `auto` mode skip step 3: decide per "Auto mode" and carry on.

### The six outcomes

Every finding ends in exactly one of these, and every one gets a dated note in
the row's `Comments`:

| Outcome | Status becomes | What you do |
|---|---|---|
| **Fix now** | `resolved` | Make the change, then record what you changed. |
| **File it** | `resolved` | Create a story issue `issues/NN-<slug>.md` carrying the work; the comment points at it. |
| **Already fixed** | `resolved` | Verify against the tree first, then record the evidence. |
| **Waive** | `wontfix` | Only on the user's explicit say-so; record their reason, not yours. |
| **Superseded** | `superseded` | Name the decision that killed it. |
| **Leave open** | `open` | Add a dated comment saying what is still blocking it, move on. |

**File it** is the one people forget. A finding like "the loop must filter
path-item keys" is an implementation note that belongs in the issue that will
be implemented, not in a review table for the rest of the story's life. Moving
it is a resolution, not a dodge.

## Making changes

Unlike `review-story`, this skill **may** edit the story documents and the
code — most findings are "`spec.md` §2 contradicts the decision" or "issue
`02` is missing a step", and a walkthrough that cannot touch them is theatre.

The guard:

- No change without an explicit user instruction **for that specific
  finding**. A general "yes, fix things" is not consent for the next eight.
- Suggest freely; apply only when told. Even in autopilot mode, ask and wait.
- Keep each change minimal and scoped to the finding in hand.

## Committing

After a finding that produced a change, ask whether to commit it before
moving to the next one. Even in autopilot mode, ask and wait.

- Per-finding commits keep the history bisectable, which is most of the value
  of doing this one at a time.
- Follow `docs/agents/commit-policy.md` for the issue number and trailers.
- **Never push** without an explicit instruction.

If the user prefers one commit at the end, that is fine — but ask, don't
assume it.

## Finishing

When every open finding has been handled, or the user stops:

1. Append **one** journal row to `review.md`, with `Kind` = `walkthrough`:
   `Findings` = `0 new, 3 resolved, 6 still open`; `Notes` = "walkthrough, not
   a review pass". Marking it clearly matters — a later `/review-story` must
   not mistake a walkthrough for a review of the current tree, and the
   implementation gates ignore `walkthrough` rows when deciding whether a
   review is stale. A walkthrough edits the story documents without
   re-examining them, so it must never make a story look freshly reviewed.
2. Summarise in chat: what was resolved, what was filed as issues, what is
   still open and why.
3. If findings were filed as new issues, remind the user those issues are
   themselves unreviewed.

## Auto mode

`auto` is the user's standing instruction to fix findings without asking, so
the "explicit instruction for that specific finding" guard and the
ask-and-wait rules (decision, change, per-finding commit prompt) are
satisfied for the outcomes below. Everything else keeps its guard.

For each open finding, in order, still re-reading the current state first:

- **Fix now / File it / Already fixed**: choose and apply the recommended
  outcome yourself. Keep each change minimal and scoped to the finding. Prefer
  **File it** when the finding is an implementation note for an issue, and
  **Already fixed** only with evidence from the tree.
- **Superseded**: apply when a recorded decision clearly kills the finding;
  name the decision.
- **Never waive.** `wontfix` still needs the user's explicit say-so and their
  reason. In auto mode a finding you would waive stays **Leave open**, with a
  dated comment saying why.
- **Leave open** anything ambiguous, anything that needs a product or design
  decision the story does not already record, and anything whose fix would be
  destructive or reach beyond the story and the code it names. Do not guess to
  keep the run moving.
- **Record each outcome** with a dated `Comments` note exactly as in
  interactive mode, and run the cheapest relevant check (lint, test, synth)
  for any code you changed. Report failures; do not mark the finding
  `resolved` over a failing check.
- **Commits**: do not commit and never push unless the user also said to
  (e.g. `auto commit`). With `auto commit`, make one commit per finding that
  produced a change, following `docs/agents/commit-policy.md`. Otherwise leave the
  changes uncommitted and say so at the end.

Finish as in "Finishing", with `Notes` = "auto walkthrough, not a review
pass", and add to the chat summary an explicit list of every autonomous
decision (finding, outcome, why), since the user was not in the loop. Findings
left open in auto mode are listed first.
