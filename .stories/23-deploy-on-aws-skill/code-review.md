# 23-deploy-on-aws-skill: code review

Code reviews of this story's issues. The journal lists every pass: `code-review` runs of `/review-code`,
`check` rows for the `scripts/check` run before each review, and `walkthrough` passes of
`/address-code-review-findings`. The findings table lists what each run found and what became of it. Rows are never
deleted or renumbered: a fixed finding flips to `resolved`.

Statuses: `open` / `resolved` / `wontfix` / `superseded`.
Axes: `standards` / `spec` / `security`.
Conventions: `docs/agents/issue-tracker.md`, section "Code review file".

## Review journal

| Date | Kind | Reviewed | By | Findings | Notes |
|---|---|---|---|---|---|
| 2026-11-22T17:45Z | check | @ b2672bb85978 | scripts/check | — | green (baseline from /implement-story 23) |
| 2026-11-22T17:45Z | code-review | 01 @ 339cbafa7b..b2672bb859 | claude-sonnet-5-5 (dispatched; code by claude-opus-5-5) | 4 new: 1 standards, 3 spec, 0 security | caller mode from /implement-story 23, commit filter (#23); reviewers on Sonnet, same vendor family as the author; auto round 1 |

## Findings

| # | Status | Axis | Affects | Description | Comments |
|---|---|---|---|---|---|
| C1 | open | standards | 01, .claude/skills/deploy-on-aws/SKILL.md | Possible Duplicated Code: the "Asking" section's `auto` stop list repeats by hand the stops each step already names, so it drifts when a step changes; "every stop a step names" would do. | 2026-11-22: found. 2026-11-22: needs the user (e2777aaec1): a judgement-call smell, and spec.md's 'Asking' decision (from story review R24) asks for the explicit 'among them …' list, so dropping it means overriding the spec; the waiver or a spec change is the owner's. |
| C2 | open | spec | 01, docs/agents/commit-policy.md:15 | The "Exception, skills that commit" paragraph still says the covered skills "never push" unqualified, with the `/deploy-on-aws` exception only appended after it; issue 01 asks for the paragraph to name it "instead of saying all such skills 'never push'". | 2026-11-22: found. |
| C3 | open | spec | 01, .claude/skills/deploy-on-aws/SKILL.md:132 | Step 4's account check compares `source.location` with `git remote get-url origin` without normalising it, so an SSH-form remote or one without `.git` reads as "this isn't the sfab account"; the spec names the repo `gratinierer/sfab`. | 2026-11-22: found. |
| C4 | open | spec | 01, .claude/skills/deploy-on-aws/SKILL.md | Possible scope creep: behaviour the spec doesn't ask for, chiefly the "Never" bullet's new "any rewrite of `main` or `production`" and its turning Out of scope items into prohibitions, plus the build status list and the ascending three-story title form. | 2026-11-22: found. |
