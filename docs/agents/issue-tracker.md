# Issue tracker: Stories (Local Markdown)

Work is organised in **stories** (each tied to a GitHub issue) containing
numbered **issues**, all tracked as markdown files under `.stories/`.

## Conventions

- One story per directory: `.stories/<n>-<slug>/`, e.g.
  `.stories/42-improve-onboarding/`. The `<n>` prefix is the number of the
  GitHub issue this story corresponds to (see `github.md`); `<slug>` is
  short, kebab-case, and meaningful on its own. It may differ from the
  `<n>-<title-slug>` branch `gh issue develop` creates — both share the
  `<n>-` prefix, which is what everything matches on. Create that issue the
  same way as any other (`gh issue create --title "<summary>" --body
  "<body>" --label AIassisted --assignee @me`, per
  `commit-policy.md`'s "Whenever a new GitHub issue is created"
  section — applies here too, not just inside the commit flow) — a
  `.stories/` directory is purely a local organisational grouping, the
  GitHub issue backing it is a plain issue like any other. The numbered
  `issues/NN-*.md` files below stay local; they don't get GitHub issues.
- The story spec is `.stories/<story>/spec.md`.
- Implementation issues are one file per ticket at
  `.stories/<story>/issues/<NN>-<slug>.md`, numbered from `01`, never a single
  combined tickets file.
- **State is folder location, not an in-file field:**
  - Open issues sit directly in `.stories/<story>/issues/`.
  - Closed issues — whether completed or `wontfix` — move into
    `.stories/<story>/issues/done/`. There is no separate "won't fix" folder;
    the reason lives in the file's triage label, not the folder.
  - A whole story closes the same way: once it's finished (all issues done,
    or the story itself abandoned), move the entire `.stories/<story>/`
    directory into `.stories/done/<story>/`.
- While an issue is open, its triage role is recorded as a `Label:` line near
  the top of the file (see `triage-labels.md` for the role strings). This is
  separate from open/closed state, which is folder-based only.
- Comments and conversation history append to the bottom of the file under a
  `## Comments` heading.

## When a skill says "publish to the issue tracker"

Create a new story directory under `.stories/` (with a `spec.md`) if none
already covers this work, and/or a new numbered issue file under its
`issues/`.

## When a skill says "fetch the relevant ticket"

Read the file at the referenced path. The user will normally pass the path,
or the story/issue identifiers, directly.

## Starting work from a bare GitHub issue

If the user names a bare GitHub issue — `#42`, an issue URL, or
`owner/repo#42`; a plain `42` only when they're clearly asking to start work
on it ("work on 42") — and no `.stories/<n>-*/` directory exists yet,
scaffold one before proceeding:

1. **Existing folder check first.** If `.stories/<n>-*/` already exists,
   stop here — reference its `spec.md`/`issues/` instead of fetching or
   grilling again. Match on the full number plus the trailing `-`, so `4`
   never matches `42-…`.
2. **Repo mismatch.** `#42` and `42` always mean this repo (see
   `github.md`). If a URL or `owner/repo#42` points at a different repo,
   fetch it anyway — cross-repo references happen — but warn the user which
   repo it resolved to and confirm before scaffolding.
3. **Fetch the issue** with `gh issue view <n> [-R owner/repo] --json
   title,body,state,labels,comments,parent`: title, body, labels, state, the
   last 5 comments, and — if it has a parent issue — that parent's number and
   title only (a one-line reference; don't fetch the parent's own
   body/comments).
4. **Derive the slug** as a short kebab-case form of the summary, show it to
   the user, and let them confirm or edit it before creating
   `.stories/<n>-<slug>/`.
5. **Seed `spec.md`** from the fetched content (GitHub issue link, Goal from
   the title/body), and create an empty `review.md` beside it from the
   template in the `review-story` skill (see "Review file" below). An absent
   `review.md` is the signal "never reviewed", so scaffolding always writes
   the empty one rather than leaving the file out. Add an
   `Authored: <ISO date> by <model or "human">` line to `spec.md`'s header —
   `review-story` uses it to avoid having a story reviewed by the model that
   wrote it (see "Review file").
6. **Grill it.** Use the `grilling` skill's design-tree/round format to work
   through ambiguities in the ticket with the user, starting from the
   fetched facts. This is a conversation, not a solo pass — never answer the
   user's own questions for them.
7. **Persist the result** as a `## Clarifications` section appended to
   `spec.md`, recording the resolved questions and answers (not just the
   conclusions) so later sessions see the original requirement, not just
   where it landed.
8. **Stop there.** This scaffolds `spec.md` only — slicing settled decisions
   into numbered `issues/NN-*.md` files is a separate, later step, not part
   of this flow.

## Review file

Every story directory has a `review.md` beside its `spec.md`, created empty
when the story is scaffolded. It is written by the `review-story` skill
(`/review-story`) and holds two tables:

- a **review journal** (`Date`, `Kind`, `Reviewed`, `By`, `Findings`,
  `Notes`), one row per pass, newest last. `Kind` is `review` or
  `walkthrough`;
- a **findings table** (`#`, `Status`, `Axis`, `Affects`, `Description`,
  `Comments`), one row per finding.

Rules:

- **Rows are never deleted and never renumbered.** A fixed finding flips to
  `resolved` with a dated comment; it does not disappear. The table is the
  audit trail, the journal is the history of passes.
- **Statuses**: `open`, `resolved`, `wontfix` (consciously accepted, only on
  the user's say-so), `superseded` (made irrelevant by a later decision
  rather than fixed).
- **Axes**: `factual`, `consistency`, `completeness`, `convention`,
  `staleness` — see the `review-story` skill for what each one covers.
- **Anyone may flip a row to `resolved`.** An agent that fixes finding `R4`
  while implementing issue `02` updates `R4` there and then, dated.
  `/review-story` audits those flips on its next pass — re-checking every
  closure made since the last `Kind = review` row and reopening the original
  row if the evidence does not hold — but it is not the only thing allowed to
  make them.
- Only `/review-story` writes findings; it never edits `spec.md` or the issue
  files. A wrong `spec.md` becomes a finding, and fixing it is a separate,
  explicitly requested step.
- **Acting on findings** is `/address-story-review-findings`: it walks the
  open rows one at a time with the user, like `github-pr-review` does with PR
  comments. Each finding ends as fixed, filed as a new story issue, waived,
  superseded, or explicitly left open — never silently. That skill *may* edit
  the story and the code, but by default only on a per-finding go-ahead:
  a blanket "yes, fix things" is not consent for the next eight findings.
  - **`auto` exception**: when the user passes an explicit `auto` argument,
    that is standing consent and the skill works through the findings
    unattended. The guards stay: it never waives (a `wontfix` always needs
    the user's own reason, so a finding it would waive stays open), never
    commits unless the user also said `auto commit`, never pushes, and
    leaves anything ambiguous or destructive open rather than guessing.
    Running under the CLI's own autopilot is **not** `auto` — only the
    typed argument is. See the skill for the full rules.
- **Reviewer independence**: `spec.md` carries an
  `Authored: <ISO date> by <model or "human">` line. A model may not review a
  story it wrote — `/review-story` dispatches the pass to a different model
  instead, and records both in the journal's `By` column. Human-authored
  stories are exempt.

## Code review file

A story's code reviews are recorded in `.stories/<story>/code-review.md`, beside `review.md`. The repo's
`code-review` skill (`/code-review`) creates it on its first run and writes every run's findings to it, so they
outlive the conversation. It holds the same two tables as `review.md`:

- a **review journal**, one row per pass, newest last. `Kind` is `code-review` for a `/code-review` run or
  `walkthrough` for a pass of `/address-code-review-findings`. `Reviewed` names the issues and the commit range,
  such as `04, 05 @ 862ddba..e2b740c`, and `By` names the reviewers' model and the code's author, which differ:
  `/code-review` runs its reviewers on a model other than the one that wrote the code.
- a **findings table**, with IDs `C1`, `C2` and so on, never reused, and `Axis` as `standards` or `spec`. The two
  axes stay apart and aren't ranked against each other.

The statuses and the rules for rows are those of the review file: rows are never deleted or renumbered, `wontfix`
needs the user's own reason, and anyone may flip a row to `resolved` with a dated comment. Acting on the findings is
`/address-code-review-findings`, which walks the open rows one at a time with the user, or unattended with `auto`.

## Before implementing a story

Before writing code for any issue in a story, check that story's `review.md`
and **stop** — surface the problem and wait for the user's explicit go-ahead —
in any of these three cases:

1. **No `review.md`, or an empty one** (no journal rows with `Kind` =
   `review`): the story has never been reviewed. Tell the user and recommend
   `/review-story` first.
2. **Any row with status `open`**: list them and ask whether to address them
   first, waive them, or proceed anyway.
3. **Stale review**: `spec.md` or any file under `issues/` was modified after
   the newest `Kind = review` journal row's date. The story moved since it was
   last reviewed; recommend a fresh `/review-story` pass. Ignore
   `walkthrough` rows here — a walkthrough edits the documents without
   re-examining them, so counting it would let its own edits certify
   themselves.

This is a stop, not a log line. Do not proceed on the assumption that a
warning the user may never read counts as consent — but do proceed
immediately once they say so, including a blanket "proceed anyway".

## Closing an issue or story

Move the file (or whole directory, for a story) into the sibling `done/`
folder. The move itself is the record — don't also add a status field. When
a story moves to `.stories/done/` and its GitHub issue is still open (it
normally closes via `Closes #<n>` in the merged PR), offer to run
`gh issue close <n>` — never close it without asking. Delete
the `Label:` line when moving a completed issue; only `wontfix` keeps its
label, since it records why the issue was closed. A file in `done/` without a
`Label:` line is therefore completed.

**An issue that changes code needs a code review before it closes.** It changes
code when any of its commits touch files outside `.stories/` and `docs/`.
Before moving it to `done/`:

- the story's `code-review.md` has a `code-review` journal row naming the
  issue, whose range reaches the issue's last such commit, so code added after
  a review needs another run; and
- no finding against the issue is `open`. Marking one `wontfix`, on the user's
  say-so, or filing it as an issue unblocks it.

This applies to issues closed from 2026-11-04 on, starting with
`2-identity-api#05`. Story `14-agent-skills`, which builds the rule, is exempt.
It gates closing, not starting: starting work is the review file's gate.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a file with one **child** file per
ticket, scoped to a story.

- **Map**: `.stories/<story>/map.md` (the Notes / Decisions-so-far / Fog body).
- **Child ticket**: `.stories/<story>/issues/NN-<slug>.md`, numbered from `01`.
  A `Type:` line records the ticket type
  (`research`/`prototype`/`grilling`/`task`).
- **Blocking**: a `Blocked by: NN, NN` line near the top. A ticket is
  unblocked when every file it lists is in `issues/done/`. A ticket that
  depends on another story's ticket names it as `<story>#<NN>`, such as
  `Blocked by: 01, 2-identity-api#13`, and that one counts as done once
  `.stories/<story>/issues/done/<NN>-*.md` exists, or
  `.stories/done/<story>/issues/done/<NN>-*.md` once the whole story is done. Every dependency goes on
  this line, never only in prose, so a frontier scan can check it.
- **Frontier**: scan `.stories/<story>/issues/` (excluding `done/`) for files
  that are unblocked and unclaimed; first by number wins.
- **Claim**: set a `Claimed: yes` line near the top and save before any work.
- **Resolve**: append the answer under an `## Answer` heading, move the file
  into `issues/done/`, then append a context pointer (gist + link) to the
  map's Decisions-so-far in `map.md`.
