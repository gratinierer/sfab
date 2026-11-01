## Agent skills

### Issue tracker

Stories and issues as local markdown under `.stories/`, stories named
`<n>-<slug>` after their GitHub issue number. See
`docs/agents/issue-tracker.md` and `docs/agents/github.md`. New GitHub issues
are always labeled `AIassisted` and self-assigned on creation — see
`git-commit-policy.instructions.md`'s "Whenever a new GitHub issue is
created" section, which applies regardless of which skill or flow created
the issue.

### Triage labels

Default vocabulary: `needs-triage`, `needs-info`, `ready-for-agent`,
`ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `GLOSSARY.md` plus `docs/adr/` at the repo root, both
created lazily. See `docs/agents/domain.md`.

### Reference files

Check `docs/aws-facts.md` when a story, review or change relies on AWS
behaviour, and update its row whenever you confirm or disprove a fact.

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
`/address-story-review-findings`. See `docs/agents/issue-tracker.md`, sections
"Review file" and "Before implementing a story".

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
