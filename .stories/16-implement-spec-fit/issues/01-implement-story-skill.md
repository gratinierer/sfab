# A repo skill, `/implement-story`, implements a whole story

Label: ready-for-agent
Blocked by: 02, 03

`spec.md` "Decisions": copy the global `/implement-spec` to `.claude/skills/implement-story/` and change it so a run
keeps this repo's rules.

## Scope

- `.claude/skills/implement-story/SKILL.md` starts as a verbatim copy of
  `~/.claude/skills/implement-spec/SKILL.md` (mattpocock/skills, `skills/engineering/implement-spec/`), with `name:
  implement-story`, a description naming `.stories/` stories, and `disable-model-invocation: true` kept. Its
  `agents/openai.yaml` is copied too, with the display name changed.
- Its argument is the story: a number, directory name or path, resolving to exactly one `.stories/` directory, or
  it asks. The issue tracker is `docs/agents/issue-tracker.md`, replacing "run `/setup-matt-pocock-skills`", as in
  the other vendored skills.
- **Start gate**: before anything else it applies `AGENTS.md` "Before implementing a story" to the story's
  `review.md`, and stops and asks in any of its three cases. In the same go, before any sub-agent runs, it settles
  everything `code-review`'s auto mode asks "up front, then unattended": the reviewers' model (other than the
  orchestrator's and the implementers'; none available stops the run here), the branch ruling, uncommitted story
  documents (offered as one commit), and the baseline `scripts/check` (a failure is fixed first, or the owner
  waives check gating). It hands these answers to `code-review` at the end through issue 03's caller mode, which
  then asks nothing. It also lists the story's open issues not labelled `ready-for-agent` (`ready-for-human`,
  `needs-info`, `needs-triage`): those aren't implemented, and issues they block wait.
- **Branch**: replaces "Create the integration branch". It runs on the current branch when that is the story's
  `<n>-<slug>` branch, or one the owner has ruled the story onto (`docs/agents/commit-policy.md` "Which number a
  commit carries"), and otherwise stops and asks. It never creates the integration branch.
- **Run log**: each run appends `- <ISO date> run started at <sha>` to `.stories/<story>/implementation.md`,
  creating it with a one-line heading if missing, where `<sha>` is `HEAD` before that commit, and commits the file
  alone as `Start implementing story <n> (#<n>)`. The file is only appended to. A resumed run treats issues in
  `issues/done/`, or whose commits are already merged into the branch, as finished.
- **Review base**: the reviewed `HEAD` of the newest `code-review` journal row in the story's `code-review.md`, or,
  with no such row, the commit on the run log's first line. So a resumed run reviews only what no review covered,
  and no finding is filed twice.
- **One implementer at a time**: the frontier is worked sequentially, the reason (each worktree builds cold in its
  own `lambda/target/` and `scripts/check` takes minutes, so parallel tickets contend for CPU and disk) stated in
  the skill. Each ticket still gets a fresh sub-agent in its own
  worktree on a branch off the story branch, using `/tdd`, and a merger sub-agent merges it back. The ticket's
  branch passes `scripts/check` after merging the story branch's tip into it, before the merger takes it; a failure
  goes back to the implementer, and a second one stops that ticket, which stays open (see "When the frontier
  empties").
- **Seams**: the implementer brief says the issue file stands in for `/tdd`'s confirmation of the seams under test:
  the implementer takes the seams the issue's Scope and "Done when" imply and names them in its commit body ("Seams:
  …"), and the review's Spec reviewer checks them against the issue. When the issue doesn't make them clear, the
  implementer stops without guessing, and the ticket is reported as needing the owner. A ticket branch with no
  commit carrying a `Seams:` line goes back to the implementer, as a failed check does, before the merger takes it. `tdd` itself stays
  unchanged.
- When the owner waived a failing
  baseline, only failures the baseline didn't show count.
- **Commits**: the implementer **and merger** briefs say every commit follows `docs/agents/commit-policy.md`:
  subject ending `(#<n>)`, body naming "Issue NN of story N", and the policy's trailers including `Co-Authored-By:`
  with the model. The merger writes its merge commit's message that way, never git's default `Merge branch …`.
  Implementers don't move issue files to `done/`. The commits and per-ticket branches rest on issue 02's rule.
- **Exploration notes** go in the session's scratchpad directory, passed to each implementer as a path.
- **When the frontier empties**: the run moves on to the review once no further ticket can start, whether every
  ticket merged or some stay open (blocked by another story's open issue, not `ready-for-agent`, or stopped after a
  second failed check). It reviews and closes what was merged, and reports what stays open and why.
- **Review**: replaces upstream's step 7. After the last merge it reads `.claude/skills/code-review/SKILL.md` by
  path and follows it inline, with `<review base> auto 3` and issue 03's caller mode: the story, every open issue
  of it with a commit since the run log's first line (whether this run or an earlier one made it, and whether or
  not its commits fall in the range), the start gate's answers, the filter to the story's own `(#<n>)` commits, and the Spec brief line: "for each commit
  whose body has a `Seams:` line, check those seams against its issue; a commit without one (run log, merge,
  closing, review fixes) is not a finding". Later rounds' ranges hold only fix commits, so it finds nothing there.
  It doesn't call it by name through the Skill tool, since Claude Code ships a built-in `code-review` too. The
  skill says this is the repo
  skill in `.claude/skills/code-review/`, whose arguments are a fixed point and `auto`, not Claude Code's built-in
  `/code-review`. The "fix all issues in one sub-agent" step goes; the auto loop's address passes do that.
- **Closing**: only when the auto review stops clean, it moves the reviewed issue files to `issues/done/`,
  deleting their `Label:` line, per `issue-tracker.md` "Closing an issue or story", in one commit. On any other stop
  (limit, no progress, failing check) it leaves them open. Issues it didn't implement (not `ready-for-agent`, or
  blocked by one) are reported as keeping the story open. When that closes the story's last open issue, it offers
  to move the story to `.stories/done/` and to run `gh issue close <n>`, and does neither without the owner's yes.
- **End**: no draft pull request. It reports the branch, the commits as short shas with subjects, the review's stop
  reason and what needs the user, and opens a pull request only when asked. It cleans up the worktrees and their
  branches, except those of tickets that stopped unmerged (second failed check, unclear seams): it keeps them and
  names them in the report, so their work isn't lost.
- **`tdd` is vendored** to `.claude/skills/tdd/` unchanged (`SKILL.md`, `mocking.md`, `tests.md`,
  `agents/openai.yaml`), since `implement-story` calls it.
- `.claude/skills/VENDORED.md`: rows for `implement-story` (mattpocock/skills, `skills/engineering/implement-spec/`,
  commit `4ae2317`, hash `0ccc13c6766b`) and `tdd` (mattpocock/skills, its upstream path, hash `daf55026f456`, its
  commit checked the same way), and a "Changes from upstream" entry listing the `implement-story` changes above. Its
  intro (":3", "copied in once on 2026-11-04") and the "Matt Pocock's ten were identical" line (":26") are updated
  to say which skills came later. It also states the rule: a skill here calls only skills that are also here, and
  a skill vendored later brings its dependencies with it. `AGENTS.md` "Skills" states the same rule in one
  sentence. ADR 0005 itself stays unchanged.
- `AGENTS.md` "Before implementing a story" and `docs/agents/issue-tracker.md` "Before implementing a story" each
  gain one sentence: `/implement-story <n>` implements a whole story in one run, gates included.
- `docs/agents/issue-tracker.md` gains a short "Implementation log" section beside "Review file" and "Code review
  file": what `.stories/<story>/implementation.md` holds, that it is only appended to, and that its first line's
  commit is where a story's implementation began.

## Done when

- The skill file says all of the above, and a reader of it alone could run a story without breaking
  `docs/agents/commit-policy.md` or `docs/agents/issue-tracker.md`.
- `VENDORED.md`, `AGENTS.md` and `issue-tracker.md` carry their additions, and every skill named by a skill in
  `.claude/skills/` exists there.
- `code-review` and `address-code-review-findings` are unchanged in name.
