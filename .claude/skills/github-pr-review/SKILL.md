---
name: github-pr-review
description: Workflow for checking or reviewing a GitHub pull request's review comments together with the user, one comment at a time. Use when asked to check, review, or go through the review comments on a PR.
---

When asked to check or review a PR's comments:

1. Fetch the PR's review comments with `gh` CLI. If this fails due to authentication, abort
   and ask the user — do not attempt other workarounds.
2. Detect the corresponding local repository and branch for the PR. If unsure, ask the user
   to provide the local repo path — even in autopilot mode, ask and wait for the user.
3. Go through the comments one by one, in dialogue with the user. Do not make changes to the
   PR's code without explicit user instruction for that specific comment. Present each comment
   (who wrote it, where, what it asks), why it matters, and your recommendation, then end with the numbered
   Options block of the `decision-form` skill (load it with the Skill tool), always in this
   order:
   1. Fix now (recommended when the comment asks for a code change)
   2. Reply only: answer or explain, no change
   3. Decline with a reply and a reason
   4. Leave unresolved
4. You may suggest changes to the code, but do not commit or push them without explicit user
   instruction — even in autopilot mode, ask and wait for the user.
5. If you make a code change in response to a comment, ask the user whether to commit that
   change before moving on to the next comment — even in autopilot mode, ask and wait for the
   user. Ask it as a second numbered question in the same form: 1. Commit (recommended),
   2. Don't commit. Commit only when they choose 1 or say so in their own words; never push without explicit instruction.
6. After a comment is resolved (outcomes 1 to 3; outcome 4 leaves it unresolved), mark it as resolved on the PR, then move on to the next
   comment. Continue until every comment is resolved or the user wants to stop.
