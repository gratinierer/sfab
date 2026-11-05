# Review skills: an auto loop, and commits by default

GitHub issue: [gratinierer/sfab#15](https://github.com/gratinierer/sfab/issues/15)
Authored: 2026-11-05 by claude-opus-5-5

## Goal

A review and the work on its findings should run as one unattended loop, so the owner is asked only about what
needs a human: design decisions and important findings. Today `/review-story` and `/code-review` stop after the
review, `/address-*-review-findings auto` is started by hand, nothing re-reviews the fixes, and every commit needs
a separate yes.

## The auto loop

- **Syntax:** `/review-story <story> auto [X]` and `/code-review <fixed-point> auto [X]`. X is a positive integer,
  default 3. Only the two review skills take X. An address skill run on its own with `auto` stays a single pass,
  as it is today. `auto` and `nocommit` are keywords, never a story or ref: `/code-review auto` is a missing fixed
  point (a branch literally named `auto` is given as `refs/heads/auto`). An X that isn't an integer of at least 1
  (`auto 0`, `auto -1`, `auto foo`) is an invalid input, asked about up front like any other.
- **Shape:** `review, (address, review) × up to X`. X counts address passes, and the loop always ends on a
  review, so the last fixes are reviewed, `review.md` isn't left stale for the implementation gate, and the
  code-review closing gate can be met.
- **Early stop:** the loop stops before X when a review leaves no open rows (for `code-review`, none against the reviewed issues), or when an address pass makes no
  progress: it changes no file outside the record file (`review.md` or `code-review.md`), so everything left needs
  a human and another review would repeat itself. Comments, status flips to `open`, and the `walkthrough` row
  don't count as progress. The final report names the stop reason.
- **Up front, then unattended:** everything the loop could need to ask is asked before round 1: the commit-policy
  branch check, a missing fixed point, an unresolvable story or reviewer model, a record file with uncommitted
  changes. When the story isn't the branch's own (story 8 reviewed from `2-…`), the branch check asks for the
  owner's ruling that its commits go on this branch with their own number, as commit-policy "Which number a commit
  carries" requires, and the loop doesn't start without it. When the story's own documents (`spec.md`, `issues/**`,
  the record file) have uncommitted changes, it offers to commit them first as one commit, `Commit story <n>
  documents before review (#<n>)`, and doesn't start if the owner declines; otherwise every finding on them would
  stay open under "Staging". Code files keep the Q8 rule. After that it never asks mid-loop.
- **Reviews:** every round dispatches the review to a fresh sub-agent on a model other than the story's or code's
  author **and** the orchestrating model, because from round 2 the orchestrator wrote the fixes. This holds even
  for a human-authored story, which review-story would otherwise review directly. For `code-review`, step 4's
  author detection therefore includes the orchestrator's fix commits in rounds 2+. The `By` column names the
  reviewer. Address passes need no independence guard, as today.
- **Code-review range:** round 1 uses the given fixed point. Later rounds review only the fix commits,
  `<previous review's HEAD>..HEAD`, and their journal rows still name the round-1 issues, as
  `<issues> @ <prev>..<HEAD>`, so the closing gate sees a row reaching each issue's last commit.
- **Which HEAD is recorded:** a review's `Reviewed` names the HEAD it reviewed, before its own record-file commit.
  The next round's range starts there; the record-file commit touches only `.stories/`, so including it is
  harmless. For review-story, the loop ends on a `review` row, so the staleness gate sees a review newer than the
  last walkthrough's edits; committing changes no file dates the gate compares.
- **Address passes:** run inline in the orchestrating session through the Skill tool, with `auto`, so one session
  writes the record file and makes the commits.
- **What needs a human:** everything `auto` already leaves open (design or product decisions the story doesn't
  record, ambiguity, waivers, destructive or out-of-scope fixes), plus **important findings**: a security or cost
  risk, a fix that would change an ADR, or one that would rewrite a story's scope. These stay open with a dated
  comment saying why, the loop carries on, and they are presented together at the end. So is a finding left open
  only because its file had unrelated uncommitted edits (see "Staging").
- **`auto nocommit` is refused:** the loop needs commits for its ranges and its progress check, so that
  combination is asked about up front and the loop doesn't start with it. `nocommit` alone still works for a
  single review or address pass.
- **Baseline check:** a code-review loop runs `scripts/check` once before round 1. If it already fails, that is
  asked up front: fix it first, or run without check gating, in which case no finding is reverted for a failure
  the baseline already showed. A review-story loop changes only documents and runs no check.
- **Failing checks:** an address pass runs the cheapest relevant check after every finding that changes code, and
  `scripts/check` before each round's closing review. When a finding's check fails, `auto` reverts that finding's
  change by restoring only its own paths (never a blanket reset), leaves it open with a comment quoting the
  failure, commits only that comment, and lists it as needing the owner. A reverted change isn't progress.
  Interactive mode shows the failure and asks instead of reverting.
  If the `scripts/check` before a closing review fails beyond the baseline, the loop stops after that review and
  lists the failure as needing the owner, rather than addressing on a broken tree. A standalone code-review
  walkthrough runs `scripts/check` once at the end, before its walkthrough commit, and reports a failure.
- **Final report:** in order: findings needing the owner, each with why; rounds run and the stop reason; the
  commits made as short shas; every autonomous decision. It also names every story issue the loop filed, as
  unreviewed, and, for the code-review loop, says when an address pass edited `spec.md` or `issues/`, since that
  leaves the story's `review.md` stale for the implementation gate and wants a `/review-story`.

## Commits by default

In every mode, interactive and `auto`, in all four skills:

- **Review skills** commit the record file once, after the review is written.
- **Address skills** commit once per treated finding, including outcomes that only edit the record file
  (`wontfix`, leave open with a comment, `superseded`, filed). Every outcome writes a dated comment, so every
  treated finding gets a commit; only a finding skipped untouched (the user stopped) gets none. The closing
  `walkthrough` journal row gets its own commit.
- **Staging:** only the paths that finding or review touched, never `git add -A`. A finding whose file already has
  unrelated uncommitted edits stays open with a comment.
- **Never push.** Pushing stays a separate request.
- **Opt-out:** `nocommit` in any of the four skills. `auto commit` is removed, since `auto` now commits.
- **Subjects**, as agreed in Q13: `Review story 2 and record R180-R183 (#2)`, `Code-review issues 04, 05 and
  record C12-C15 (#2)`, `Address R181: <short outcome> (#2)`, `Address C13: <short outcome> (#2)`. The review
  subjects follow the history's `Review story 2 … (#2)` style; the per-finding `Address …:` subjects are new, since
  findings were committed in batches before ("work through R177-R179"). The body carries the finding and the
  trailers from `docs/agents/commit-policy.md`.
- **Policy:** `docs/agents/commit-policy.md` gets an exception to its opening paragraph ("Never run `git commit`…
  unless the user explicitly asks"): running one of these skills is the explicit ask to commit for that skill's own
  commits. Every other commit and every push stays as the policy says.
- **Skill texts that go stale:** review-story's `Scope` ("Writes: `review.md` only", "Never: edits…") and its
  `Reviewer independence` note ("the user is in the loop on every one") get an auto-mode exception, and so do
  issue-tracker.md "Review file" lines "Only `/review-story` writes findings…" and "by default only on a
  per-finding go-ahead". `.claude/skills/VENDORED.md` needs no edit: it says later changes are recorded in git
  history.
- This reverses the repo-wide "commit only when asked" default for these skills, so it is recorded in
  [ADR 0006](../../docs/adr/0006-review-skills-commit-by-default.md).

## Where the work happens

On `2-identity-api-birth-evolve-self-and-reclaim`, by the owner's ruling, so its commits end with `(#15)` per
`commit-policy.md` "Which number a commit carries". The uncommitted `lambda/` work on that branch is untouched.

## Clarifications

Grilled with the owner on 2026-11-05.

- Q1: Does commit-by-default apply only in `auto`? — No, in every mode.
- Q2: Does the loop always end on a review, with X counting address passes? — Yes.
- Q3: Stop early on a clean review or an address pass that changes nothing? — Yes.
- Q4: Important findings (security/cost risk, ADR change, scope change) stay open and are presented at the end
  rather than pausing the loop? — Yes.
- Q5: Every review round runs on a fresh sub-agent on a model other than the orchestrator's? — Yes.
- Q6: Code re-reviews cover only the fix range? — Yes.
- Q7: Branch check once, up front? — Yes.
- Q8: Stage explicit paths only; a finding whose file is dirty with unrelated edits stays open? — Yes.
- Q9: Never push? — Yes.
- Q10: Add an exception to `commit-policy.md`? — Yes.
- Q11: Drop `auto commit`, add `nocommit`? — Yes.
- Q12: One commit per treated finding, table-only included? — Yes.
- Q13: Commit subjects in the existing history's style? — Yes.
- Q14: Syntax as above; X only on the review skills; standalone address `auto` stays one pass? — Yes.
- Q15: Ask everything missing up front, then run unattended? — Yes.
- Q16: Address passes inline through the Skill tool? — Yes.
- Q17: Final report in the order above? — Yes.
- Q18: New story or reopen 14? — New story, but on the current branch.
- Q19: An ADR for commits by default? — Yes.

Decided later, in the review walkthroughs recorded in `review.md`:

- R9 (owner): a failing check in `auto` reverts that finding's own paths, leaves it open, commits only the comment
  and lists it as needing the owner; interactive asks. See "Failing checks".
- R10 (owner): uncommitted story documents are offered for one commit before round 1, and the loop doesn't start
  without it. This narrows Q8: its leave-open rule now covers code files only.
- R14 (owner): `auto nocommit` is refused up front.
- R16 (owner): a code-review loop runs a baseline `scripts/check` before round 1, a failing baseline is asked up
  front (fix first, or run without check gating); review-story loops run no check.
- C10 (owner, from the code review): a `scripts/check` failure beyond the baseline stops the loop after the next
  review; a standalone code-review walkthrough runs `scripts/check` once at the end.
- R1–R8, R11–R13 (auto walkthroughs, audited by the following review): progress excludes the record file, every
  round uses a different model even for human-authored stories, recorded ranges and HEADs, the foreign-story
  ruling, `auto`/`nocommit` as keywords and invalid X asked up front, final-report additions, subjects marked as a
  new style.
