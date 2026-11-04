# GitHub issues

This repo's GitHub issue tracker. Authentication is plain `gh` (`gh auth login`
or `GH_TOKEN`) — nothing repo-specific beyond the **repo** below.

Used for:

- Prefixing story directory names (`.stories/<n>-<slug>/`, `<n>` = the GitHub
  issue number) — see `issue-tracker.md`.
- Branch/commit issue references and issue creation — see
  `commit-policy.md`.

## Repo

- **Repo**: `gratinierer/sfab`

## Numbering and references

Issues and pull requests share one per-repo counter (`#1`, `#2`, …), so a
number is only unique within its repo; cross-repo it's `owner/repo#<n>`.

- **Branches**: created via `gh issue develop <n> --checkout`, which links
  the branch to the issue (its "Development" sidebar) and names it
  `<n>-<title-slug>`.
- **Commits**: end the subject with `(#<n>)`, e.g. `Fix login redirect (#42)`.
  GitHub autolinks it and lists the commit on the issue's timeline.
- **Pull requests**: put `Closes #<n>` in the description, so merging into
  the default branch closes the issue.

## Looking up / creating issues

Use `gh` directly (it resolves the repo from the git remote; add
`-R <OWNER/NAME>` when working outside the checkout).

- **Look up an issue**: `gh issue view <n>` (or `--json
  title,body,state,labels,comments,parent` for structured fields)
- **Create an issue**: `gh issue create --title "<summary>" --body "<body>"
  --label AIassisted --assignee @me` — always include `--label AIassisted`:
  every issue created by an agent carries this label, so humans can tell
  agent-created issues apart from human-created ones at a glance. No issue
  type. **After creating any issue, by this or any other path,
  follow `commit-policy.md`'s "Whenever a new GitHub issue is created"
  section** — it fires on issue creation itself, not only in the
  commit flow.
- **List this repo's issues**: `gh issue list`
- **Close an issue**: normally via `Closes #<n>` in the merged PR. When a
  story directory moves to `.stories/done/`, offer `gh issue close <n>` —
  never close it without asking.
