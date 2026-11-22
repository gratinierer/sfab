# Commit policy

How agents commit, push and create GitHub issues in this repo. It started as a copy of the owner's global
`git-commit-policy.instructions.md`, and it wins over that file, or any other global commit policy a session
also loads. See `.stories/done/14-agent-skills/`.

Never run `git commit`, `git push`, or any command that writes to git history unless the user explicitly asks
you to commit or push. Making file edits is fine; committing is the user's decision.

**Exception, skills that commit:** running a skill whose process includes commits is the explicit ask to commit for
that skill's own commits, and to create and delete the short-lived branches and worktrees its process describes. That
covers `/review-story`, `/review-code`, `/address-story-review-findings` and `/address-code-review-findings` (the
record file after a review, and one commit per treated finding) and `/implement-story` (its run log, per-ticket
branches, merges and closing moves). They stage only the paths they touched, follow the rest of this policy for the
number and trailers, and never push. `nocommit` in a review skill's arguments turns this off. See
`docs/adr/0006-review-skills-commit-by-default.md` and `docs/adr/0010-skills-that-commit-may-commit.md`.
`/deploy-on-aws` is the one skill that may push and merge: running it is the explicit instruction to push `main` and
to merge it into `production` through a pull request, after asking in interactive mode and without asking in
`auto`, and never with a force-push. See `docs/adr/0012-deploy-on-aws-may-push.md`.

## Whenever a new GitHub issue is created

This applies every time any instruction, skill or ad-hoc request creates a GitHub issue, not only inside the
commit flow below. That includes `docs/agents/github.md`'s "Create an issue", `/to-tickets`, a new story under
`.stories/`, and any other path that runs `gh issue create`.

1. **Create the issue** with `gh issue create --title "<summary>" --body "<body>" --label AIassisted --assignee
   @me`. `--label AIassisted` marks issues created by an agent, and `--assignee @me` assigns it to the user. No
   issue type. The repo comes from the git remote, or `-R <owner/name>` when outside the checkout.
2. **If the label is missing**, create it and retry: `gh label create AIassisted --description "Created with AI
   assistance" --force`.
3. **If `gh` isn't logged in** (`gh auth status`), ask the user to run `! gh auth login`. If creation still
   fails, ask the user to create the issue in the GitHub web UI and give you the number.

When one request creates several issues, apply the same label and assignee to every one.

## Issue references

GitHub numbers issues and pull requests with one counter per repo, so use the native forms:

- **Branch**: `<n>-<title-slug>`, created by `gh issue develop <n> --checkout`, which also links the branch to the
  issue's "Development" sidebar.
- **Commit subject**: ends with `(#<n>)`, such as `Fix login redirect (#42)`. A squash merge appends the pull
  request's number too, such as `Fix login redirect (#42) (#57)`, which is expected.
- **Pull request description**: includes `Closes #<n>`, so merging into the default branch closes the issue.

## Which number a commit carries

A commit carries the number of the story it serves, which defaults to the branch's number. When the owner has
ruled that a story's work goes on another story's branch, its commits carry their own story's number there. For
example, story 14's commits on `2-identity-api-…` end with `(#14)`, while story 2's end with `(#2)`.

## When asked to commit or push

1. Check the current branch. If it matches `^(\d+)-`, such as `42-fix-login-redirect`, and the work serves that
   story, append ` (#<n>)` to the commit subject and commit on the current branch. Don't create a new branch.
2. If the branch doesn't start with an issue number, stop and give the user this choice:
   - **(a) Issue number**: ask for the GitHub issue number.
   - **(b) Create an issue**: if no suitable issue exists, offer to create one, following "Whenever a new GitHub
     issue is created". On success, use its number as in (a).

   With a number in hand, ask a second choice:
   - **New branch** (recommended): `gh issue develop <n> --checkout`, then commit there.
   - **Commit on current branch**: commit here with ` (#<n>)` appended to the subject.

   Don't create a branch, assume an issue number, or choose between a new branch and the current one without the
   user's instruction.
3. End every commit message with a trailer noting AI involvement, in the last paragraph of the body together
   with any other trailers, such as `Co-Authored-By:`. It names no tool, so it reads the same whichever agent made
   the commit, and its `Key: value` form lets git parse that whole paragraph as trailers:

   ```
   Assisted-by: AI
   Co-Authored-By: <the model, when the tool adds one>
   ```

   Commits before 2026-11-04 end with `Created with AI support` instead, which git doesn't parse as a trailer.
