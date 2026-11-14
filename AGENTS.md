## Agent skills

### Issue tracker

Stories and issues as local markdown under `.stories/`, stories named
`<n>-<slug>` after their GitHub issue number. See
`docs/agents/issue-tracker.md` and `docs/agents/github.md`. New GitHub issues
are always labeled `AIassisted` and self-assigned on creation — see
`docs/agents/commit-policy.md`'s "Whenever a new GitHub issue is created"
section, which applies regardless of which skill or flow created the issue.

### Commit policy

Commits, pushes and issue creation follow `docs/agents/commit-policy.md`. It
wins over any global commit policy a session also loads, including its
issue number for the commit subject and its AI trailer, which names no tool.

Every dependency between tickets goes on the `Blocked by:` line, written as
`<story>#<NN>` when it's in another story, such as `2-identity-api#13`. See
"Blocking" in `issue-tracker.md`.

### Triage labels

Default vocabulary: `needs-triage`, `needs-info`, `ready-for-agent`,
`ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `GLOSSARY.md` plus `docs/adr/` at the repo root, both
created lazily. See `docs/agents/domain.md`.

### Reference files

Check `docs/aws-facts.md` when a story, review or change relies on AWS
behaviour, and update its row whenever you confirm or disprove a fact.

Apply `docs/api-design.md` when a story, review or change adds or changes an
API route.

Update `docs/restart-after-a-stop.md` when a change adds a kill switch step or
renames what its commands look up.

When a kind of knowledge keeps being rechecked or has no home, add a small file
under `docs/`, with a pointer here saying when to read it.

### Start-of-task: check/create an `.stories` ticket first

Before starting any **non-ad-hoc** task (i.e. not a trivial one-off like a
typo or a single-line config tweak), check whether a `.stories/<story>/issues/`
ticket already covers it. If not, create one (story spec + issue file, see
`issue-tracker.md`) before writing code — the tracker should capture the
original requirement, not just leftovers filed at the end.

If the user names a bare GitHub issue (`#42`, `42`, or an issue URL) and no
matching `.stories/` folder exists, fetch it and grill it with the user
first — see "Starting work from a bare GitHub issue" in `issue-tracker.md`.

### Before implementing a story

Before writing code for any issue in a `.stories/` story, read that story's
`review.md` and **stop and ask** if any of these hold: there is no
`review.md` or it has no `Kind = review` journal rows (never reviewed —
recommend `/review-story`); any finding has status `open`; or
`spec.md`/`issues/` were modified after the newest `Kind = review` journal row
(the review is stale — `walkthrough` rows do not count, since a walkthrough
edits the documents without re-examining them). Proceed only on
the user's explicit go-ahead. Work through open findings with
`/address-story-review-findings`. `/implement-story <n>` implements a whole
story in one run and applies this gate itself. See
`docs/agents/issue-tracker.md`, sections "Review file" and "Before
implementing a story".

### Before closing an issue

An issue whose commits touch files outside `.stories/` and `docs/` needs a
`/review-code` before it moves to `done/`: a `code-review` row in the story's
`code-review.md` that reaches its last such commit, and no finding against it
still `open`. Work through findings with `/address-code-review-findings`. See
`docs/agents/issue-tracker.md`, sections "Code review file" and "Closing an
issue or story".

### Skills

The skills this repo's workflow uses live in `.claude/skills/`, and win over
any global skill of the same name. Their sources are in
`.claude/skills/VENDORED.md`. A skill there calls only skills that are also
there. See `docs/adr/0005-skills-live-in-the-repo.md`.

### End-of-task: keep ADRs and issues current

Before ending a task, check whether what you actually did was
**architecturally significant**: a new technology/library choice, a reversal
or amendment of a prior ADR, or a convention meant to be visible/binding
across teams or contexts. If so:

- **New decision**: add a new ADR under the relevant `docs/adr/` (root for
  system-wide, `<context>/docs/adr/` for context-scoped — see `domain.md`), only
  if it's hard to reverse, surprising without context, and a real trade-off.
  Everything else belongs in the story's `spec.md`.
- **Supersedes a prior decision**: add the new ADR, then mark the old one's
  Status line `Superseded by ADR-000X`. ADRs are immutable history. Rewriting or
  merging them happens only when the owner asks for it in a special case, as in
  `docs/adr/0001-initial-architecture.md`.
- **Left work deferred/unfinished**: file it as a `.stories/<story>/issues/`
  ticket (see `issue-tracker.md`), not just a code comment or chat note —
  otherwise it's invisible to the next session/agent.

This is a trusted convention, not a CI-enforced gate — there's no automated
check for "was this architecturally significant", so judge it yourself, and
err toward filing the issue when in doubt.

### Personal overrides

@AGENTS.local.md

`AGENTS.local.md` is gitignored and optional: each contributor's own rules,
which override this file where they conflict.
