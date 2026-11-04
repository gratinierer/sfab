---
name: review-story
description: "Review a `.stories/` story — its `spec.md` and numbered issues — against the codebase and the tracker conventions, and record the result in the story's `review.md`. Use when asked to review a story, a story spec, or a story's issues. This reviews planning documents, not code changes: for reviewing a diff, branch or PR, use `code-review` instead."
---

# Review Story

Audit a story's planning documents — not its code. The output is always the
story's `review.md`: a journal of review passes plus a durable findings table.

The value is in the second and third pass: open findings from earlier passes
get re-verified against the current tree, so the table converges instead of
restating itself.

## Scope

- **Reads**: `.stories/<story>/spec.md`, `.stories/<story>/issues/**`
  (including `done/`), and whatever source, config, docs and ADRs the story
  makes claims about.
- **Writes**: `.stories/<story>/review.md` only.
- **Never**: edits `spec.md`, the issue files, or any source. If the review
  says `spec.md` section X is wrong, that becomes a finding — fixing it is a
  separate, explicitly requested step.

## Reviewer independence

A model reviewing a story it wrote shares the blind spots that produced it.
Before reviewing, establish who authored the story:

1. **The `Authored:` line in `spec.md`** — `Authored: <ISO date> by <model>`,
   or `by human`. Written at scaffold time; authoritative, because it travels
   with the repo.
2. **The session store**, for stories scaffolded before that convention, or
   when the line is missing. Best-effort only — it is local to one machine and
   often empty elsewhere:

   ```sql
   -- source: local
   SELECT f.session_id, f.file_path, u.model
   FROM session_files f
   JOIN assistant_usage_events u ON u.session_id = f.session_id
   WHERE f.file_path LIKE '%<story>/spec.md'
   GROUP BY f.session_id, f.file_path, u.model;
   ```

Then:

- **Author was a human** → no independence problem. Review directly.
- **Author was a different model** → review directly, and record both models
  in the journal's `By` column.
- **Author is the model about to review** → do **not** review directly.
  Dispatch the pass to a fresh sub-agent on a different model and compile its
  findings. Tell the user which model you picked and why. They may override
  with an explicit "review with the same model anyway"; a warning alone is not
  enough, because warnings get waved through.
- **Authorship unknown, or no alternate model available** → say plainly which
  of the two it is, recommend running `/review-story` from a session on a
  different model, and proceed only on the user's go-ahead. Record the
  compromise in the journal's `Notes`, so a later reader knows the pass was
  not independent.

The sub-agent must be **fresh** — it starts with no context from the session
that wrote or discussed the story, so never a fork — and run on **any model
other than the author's**. It does not need to come from a different vendor:
author `claude-opus-5-5` → `claude-fable-5-1` or `claude-sonnet-5-5` is fine.
Always name the pick.

The sub-agent **finds**; this skill **writes**. Keep one writer so the file
format stays stable and obviously-wrong findings can be caught before they are
persisted. The journal's `By` cell then records both sides, e.g.
`claude-fable-5-1 (dispatched; story authored by claude-sonnet-5-5)`.

This guard applies to `review-story` only. `address-story-review-findings`
does not need it: the findings already exist and the user is in the loop on
every one.

## Process

### 1. Read the whole story

Every file, including `issues/done/`. Closed issues carry the decisions that
explain why the open ones look the way they do.

Also read `docs/agents/issue-tracker.md` (the conventions this story is
supposed to follow) and note the repo state the story will be judged against:
current branch, `git status`, recent commits touching the story's subject.

### 2. Re-verify the existing findings first

If `review.md` exists, check the recorded findings against the *current* tree
before looking for anything new. Two halves — open rows, then closures.

**Open rows.** Take every row with status `open`:

- Fixed → `resolved`, with a dated comment saying what fixed it.
- Made irrelevant by a later decision rather than fixed → `superseded`, naming
  the decision.
- Consciously accepted → `wontfix`, only ever on the user's say-so.
- Still true → stays `open`; add a comment if the evidence changed.

**Closures since the last review.** Anyone may close a finding — an
implementing agent, a `/address-story-review-findings` walkthrough — and this
skill is what *audits* those flips. Take every row closed since the newest
`Kind = review` journal row (`resolved`, `wontfix`, `superseded`) and check the
comment's evidence holds:

- Evidence holds → leave it closed; no comment needed.
- Evidence does not hold → **reopen the original row**, dated, saying why the
  closure was unsupported. Never file the same problem under a new `R` number:
  a bogus closure that reappears as `R23` hides the fact that `R7` was wrongly
  shut, which is the thing worth knowing.
- Closed as **filed** (the work moved into a story issue) → check the issue
  exists and carries the work. Do **not** require it to be implemented;
  filing is a resolution of the *finding*, and the issue has its own lifecycle.
- Closed as `wontfix` → check the comment records the user's reason, not the
  agent's. A `wontfix` an agent talked itself into is a finding of its own.

Rows closed before that journal row were audited by the pass that followed
them; leave them alone.

Never delete a row and never renumber one. `R4` means `R4` forever.

### 3. Hunt for new findings along five axes

Tag every finding with exactly one axis. The axes exist so a review stays
substantive instead of drifting into wording nits:

- **factual** — a claim in the story contradicts the code. Counts, file paths,
  API behaviour, "X is uncommitted in the working tree". Verify numbers by
  actually counting, not by trusting the story.
- **consistency** — the story contradicts itself. A `Decisions` summary that
  still states a reverted answer, an issue whose `Blocked by` names an issue
  that does not exist, two issues claiming the same work. A `Blocked by`
  naming a *closed* issue is not a finding — that is what unblocked looks
  like, per the tracker's definition. A `Blocked by` naming something that is
  not an issue number at all (a spec section, a decision) is `convention`,
  below.
- **completeness** — work the story implies but never tickets. A decision with
  no issue carrying it out, a dependency nobody owns, an acceptance criterion
  that cannot be met by any listed task.
- **convention** — `docs/agents/issue-tracker.md` compliance: `Label:` lines,
  numbering from `01`, closed issues in `done/`, `Blocked by` referring to
  issue numbers, spec/review file placement.
- **staleness** — the story describes a world that has moved. The branch
  merged, the interim change was discarded, the referenced ADR was superseded.

Prefer findings that would cost someone a day. A finding nobody would act on
is noise in a table that has to stay readable for the life of the story.

Verify facts yourself — read the files, count the occurrences, run the
search. Where a check is self-contained and wide (e.g. "does any spec declare
`securitySchemes`?"), a sub-agent is fine; the conclusion still has to be
backed by output you saw.

### 4. Write `review.md`, then report

Persist first, discuss after — an unsaved review is lost if the session dies.
Then show the user the open rows as a table in chat and offer to act on them.

Rows are durable, not authoritative: the user can argue any finding down to
`wontfix` afterwards, and that is a normal outcome, not a failure of the
review.

## File format

Create it from this template when it does not exist (story scaffolding should
already have done so):

```markdown
# <STORY-KEY>: review

Review record for this story. The journal lists every pass over it — reviews
and walkthroughs, distinguished by `Kind`; the
findings table lists what each pass found and what became of it. Rows are
never deleted — a fixed finding flips to `resolved`, it does not disappear.

Statuses: `open` / `resolved` / `wontfix` / `superseded`.
Axes: `factual` / `consistency` / `completeness` / `convention` / `staleness`.
Conventions: `docs/agents/issue-tracker.md`, section "Review file".

## Review journal

| Date | Kind | Reviewed | By | Findings | Notes |
|---|---|---|---|---|---|

## Findings

| # | Status | Axis | Affects | Description | Comments |
|---|---|---|---|---|---|
```

**Journal** — one row per pass, newest last:

- `Date` — ISO datetime of the pass.
- `Kind` — `review` or `walkthrough`. `review` means this skill examined the
  story and the tree; `walkthrough` means `/address-story-review-findings`
  worked through existing findings and edited the documents without
  re-examining them. Only a `review` row is evidence of a review, and the
  implementation gates compare against the newest `review` row — a
  walkthrough must never make a story look freshly reviewed.
- `Reviewed` — what it was reviewed against: a commit sha, or `working tree`.
- `By` — the agent/model that ran it, or the human who did. A human skim and
  a model pass are not the same evidence.
- `Findings` — `3 new, 2 resolved, 4 still open`. Add `1 reopened` when the
  closure audit reopened a row, so an unsupported closure is visible from the
  journal alone.
- `Notes` — one clause on what changed since the last pass, if anything.

**Findings** — one row per finding, appended in discovery order:

- `#` — `R1`, `R2`, … never reused, never renumbered.
- `Status` — `open` / `resolved` / `wontfix` / `superseded`.
- `Axis` — one of the five.
- `Affects` — the story issue(s) or file the finding is about: `02`,
  `spec.md`, `01, 04`.
- `Description` — the finding in one sentence, specific enough to act on.
  Name the file and the symbol.
- `Comments` — dated notes, newest appended: what resolved it, who waived it,
  what evidence changed. Keep each short; detail that cannot fit belongs in a
  `## Notes on <Rn>` section below the table.

Keep cells to one line. A finding needing a code block or several paragraphs
gets a `## Notes on <Rn>` section under the tables, with the row's
`Description` pointing at it.

## Anyone may resolve a row

An agent that fixes `R4` while implementing issue `02` should flip `R4` to
`resolved` there and then, with a dated comment — otherwise the next review
wastes a pass re-deriving a dead finding. `/review-story` is what *audits*
those flips, not the only thing permitted to make them: step 2 re-checks
every closure made since the last review row and reopens any whose evidence
does not hold.
