---
name: address-story-review-findings
description: "Walk through a story's open review findings one at a time, in dialogue with the user (or, with `auto`, without interaction): decide each one, optionally fix it, and record the outcome in the story's `review.md`. Use when asked to go through, work through, or address a story's review findings, or to address them automatically. For producing findings, use `review-story`; for reviewing a diff or PR, use `review-code`, and for its findings `address-code-review-findings`."
---

# Address Story Review Findings

Work through the open rows of a story's `review.md`, one finding at a time, with the user. `review-story` produces
findings; this skill consumes them. Same shape as `github-pr-review`: one item, a decision, an optional change, a
commit, then the next; the conversation is the point, not a batch pass.

## Modes

- **Interactive** (default): everything below applies as written.
- **`auto`**: the user passes `auto` (e.g. `/address-story-review-findings 2-identity-api auto`, or says "auto"). The agent
  decides and fixes each finding itself, without asking. The ask-and-wait rules below are lifted *only* where "Auto
  mode" says so. The CLI's autopilot mode is **not** `auto`: without the explicit `auto` argument, ask and wait.
- **`nocommit`**: in either mode, make no commits; leave every change uncommitted and say so at the end. Without it,
  the skill commits per finding (see "Committing").
- **Inside a review loop**: `/review-story <story> auto [X]` runs this skill as its address pass, with `auto`.
  Everything below applies, except the in-loop finish in "Finishing".

## Finding the story

1. An argument, if given: a story number (`2`), a story directory name (`2-identity-api`), or a path, and optionally the keywords `auto` and
   `nocommit`, which are never a story.
2. Otherwise infer from the current branch: `2-identity-api-birth-evolve-self-and-reclaim` → `.stories/2-*/`.
3. Otherwise, or if more than one story matches, **ask**. Never guess between two stories.

If the story has no `review.md`, or no `Kind = review` journal rows, stop: nothing to address. Recommend
`/review-story` first.

## Walking the findings

Take rows with status `open`, in ascending `#` order, one at a time. Rows that are `resolved`, `wontfix` or
`superseded` are history: skip them, unless the user names one to reopen it.

For each finding, re-read what it is about first (it may be days old and already dead; a `Possible` finding is a
judgement call, so say so). Read only what the finding needs: when `Affects` names `spec.md` or an issue, read the
section its description cites (heading or quoted line) plus its enclosing section, and the whole file only when the
finding cites no locatable part.
Then write **one plain chat message** in which nothing hides anything else, with these blocks in this fixed order:

1. **Header**: the finding's ID (`R<n>`) and `of <open count>`, the axis, and the affected files.
2. **Finding**: the problem in plain words, with the current state of the file quoted.
3. **Why it matters**: the cost if ignored.
4. **Recommendation**: one of the six outcomes, the concrete change that would be made, and why. The Options block repeats it, marked `(recommended)`.
5. **Options**: the numbered answer options, as the `decision-form` skill defines them (load it with the Skill tool):
   the six outcomes below, in the order of the table, each listed only when its precondition holds.

Show no overview table before or between findings; the count in the header is the only overview. In an interactive
pass, even in autopilot mode, ask and wait. After a decision, make the change, commit, report the change in one or two
lines plus the commit sha (unless `nocommit` is set), then present the next finding as a separate message.

`auto` is unchanged: decide per "Auto mode" and carry on, without this message.

### The six outcomes

Every finding ends in exactly one of these, and every one gets a dated note in the row's `Comments`:

| Outcome | Status becomes | What you do |
|---|---|---|
| **Fix now** | `resolved` | Make the change, then record what you changed. |
| **File it** | `resolved` | Only when the work belongs to another issue. Create a story issue `issues/NN-<slug>.md` carrying the work; the comment points at it. |
| **Already fixed** | `resolved` | Only with evidence from the tree. Verify against the tree first, then record the evidence. |
| **Superseded** | `superseded` | Only when a recorded decision kills the finding. Name the decision that killed it. |
| **Waive** | `wontfix` | Only on the user's explicit say-so; record their reason, not yours. |
| **Leave open** | `open` | Add a dated comment saying what is still blocking it, move on. |

**File it** moves an implementation note (e.g. "the loop must filter path-item keys") to the issue that will do the
work; that is a resolution, not a dodge.

## Making changes

Unlike `review-story`, this skill **may** edit the story documents and the code: most findings are "`spec.md` §2
contradicts the decision" or "issue `02` is missing a step".

The guard:

- No change without an explicit user instruction **for that specific finding**. A general "yes, fix things" is not
  consent for the next eight.
- Suggest freely; apply only when told. Even in autopilot mode, ask and wait.
- Keep each change minimal and scoped to the finding in hand.
- **Run the cheapest relevant check** (the repo's `scripts/check` if it exists, otherwise lint, test or synth) after
  any finding that changed code. If it fails, interactive mode shows the failure and asks; `auto` follows "Auto mode".
  Never mark a finding `resolved` over a failing check.

## Committing

Commit after every treated finding, without asking, in both modes, unless `nocommit` was given. Running this skill is
the explicit ask to commit, per the exception in `docs/agents/commit-policy.md`.

- **Before the first finding**, check once:
  - the branch, per `docs/agents/commit-policy.md`. If it has no `<n>-` prefix, or the story isn't
    the branch's own and the owner hasn't ruled that its commits go here, ask then, not mid-walk;
  - the story's documents (`spec.md`, `issues/**`, `review.md`). If any has uncommitted changes, offer to commit them
    first as `Commit story <n> documents before review (#<n>)`. If the user declines, carry on as `nocommit`.
- **One commit per treated finding.** Every outcome writes a dated comment, so every treated finding gets a commit,
  including those that only edit `review.md` (filed, waived, superseded, left open). A finding the user stopped
  before treating gets none, nor does one `auto` skips as still waiting (see "Auto mode"). This keeps the history
  bisectable.
- **Stage only the paths this finding touched**, by name. Never `git add -A` or `git commit -a`. If a code file the fix
  needs already has unrelated uncommitted edits, don't fix it: **Leave open** with a comment saying so, and list it at
  the end as needing the user.
- **Subject** `Address R<n>: <short outcome> (#<story>)`, such as `Address R181: file the path-item filter in
  issue 04 (#2)`. The body carries the finding; trailers follow `docs/agents/commit-policy.md`.
- **Never push** without an explicit instruction.

## Finishing

When every open finding has been handled, or the user stops:

1. Append **one** journal row to `review.md`, with `Kind` = `walkthrough`: `Findings` = `0 new, 3 resolved, 6 still
   open`; `Notes` = "walkthrough, not a review pass". Mark it clearly: a later `/review-story` must not mistake a
   walkthrough for a review of the current tree, and the implementation gates ignore `walkthrough` rows when deciding
   whether a review is stale. A walkthrough edits the story documents without re-examining them, so it must never
   make a story look freshly reviewed. Commit the row on its own, as `Record walkthrough of story <n> findings (#<n>)`,
   unless `nocommit`.
2. Summarise in chat: what was resolved, what was filed as issues, what is still open and why.
3. If findings were filed as new issues, remind the user those issues are themselves unreviewed.

**Inside a review loop**, do step 1 only, then hand back to `/review-story` with: each finding's outcome and why, which
findings need the user (rows skipped as still waiting included), which issues were filed, and whether the pass made
**progress**, meaning it changed a file other than `review.md` and kept the change. The loop runs the next review
itself, so don't recommend one.

## Auto mode

`auto` is the user's standing instruction to fix findings without asking, so the "explicit instruction for that
specific finding" guard and the ask-and-wait rules (decision, change) are satisfied for the outcomes below. Everything
else keeps its guard.

For each open finding, in order, still re-reading the current state first:

- **Fix now / File it / Already fixed**: choose and apply the recommended outcome yourself, each change minimal and
  scoped to the finding. Prefer **File it** when the finding is an implementation note for an issue, and **Already
  fixed** only with evidence from the tree.
- **Superseded**: apply when a recorded decision clearly kills the finding; name the decision.
- **Never waive.** `wontfix` still needs the user's explicit say-so and their reason. In auto mode a finding you would
  waive stays **Leave open**, with a dated comment saying why.
- **Leave open** anything ambiguous, anything that needs a product or design decision the story does not already
  record, and anything whose fix would be destructive or reach beyond the story and the code it names. Do not guess to
  keep the run moving.
- **Leave open important findings**, too, even when the fix looks clear: a security or cost risk, a fix that would
  change an ADR, or one that would rewrite a story's scope. The comment says which, and why the user should decide.
- **Mark what waits on the user.** A comment that leaves a finding open for the user (ambiguity, a design decision, a
  waiver, an important finding) starts, after its date, with `needs the user (<HEAD short sha>):`. One left open
  because its file had unrelated edits, or its check failed, starts `blocked:` instead: that can clear without anyone
  touching the row.
- **Skip what still waits.** A row whose newest comment is a `needs the user (<sha>):` note, and whose `Affects` files
  have no commits since `<sha>` (`git log <sha>..HEAD -- <files>` is empty), is not treated again: no comment, no
  commit. List it as still waiting. Interactive passes never skip: the user settles these rows there.
  - **The files**: split `Affects` at commas and drop any `:line`. Read an issue number as that issue's file in
    `.stories/<story>/issues/` or `issues/done/`, and `spec.md` as `.stories/<story>/spec.md`. Read any other part as
    a path, and resolve it to the one tracked file it names, in full or as a path suffix (`git ls-files` matching
    `*<part>`). Every part must be unchanged for the row to be skipped.
  - **Fallback**: a part that resolves to no tracked file or to several (prose, a glob, an ambiguous partial path)
    counts as changed, so the row is treated as usual.
- **Record each outcome** with a dated `Comments` note exactly as in interactive mode, and run the cheapest relevant
  check for any code you changed.
- **A failing check**: revert that finding's change by restoring only the paths it touched (`git restore -- <paths>`,
  and delete files it created), never a blanket reset. Leave the finding open with a comment quoting the failure,
  commit only that comment, and list it as needing the user. A reverted change is not progress. If the loop's baseline
  check already failed and the user chose to run without check gating, don't revert for a failure the baseline already
  showed.
- **Commits**: as in "Committing", one per treated finding, unless `nocommit`. Never push.

Finish as in "Finishing", with `Notes` = "auto walkthrough, not a review pass", and add to the chat summary an
explicit list of every autonomous decision (finding, outcome, why), findings left open first, each with why it needs
the user. Inside a review loop, hand back as "Finishing" says instead.
