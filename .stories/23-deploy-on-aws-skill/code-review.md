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
| 2026-11-22T17:46Z | walkthrough | - | claude-opus-5-5 (auto, inside /review-code auto) | 0 new, 2 resolved, 2 still open | auto walkthrough, not a review pass |
| 2026-11-22T18:01Z | check | @ 55b7da810b | scripts/check | — | green (loop check after auto pass 1, 597 passed) |
| 2026-11-22T18:01Z | code-review | 01 @ b2672bb859..55b7da810b | claude-sonnet-5-5 (dispatched; code and fixes by claude-opus-5-5) | 0 new: 0 standards, 0 spec, 0 security | auto round 2, fix commits only; C2, C3 fixes hold; cosmetic wrap notes not recorded; two reviewers resumed after a usage-limit stop |
| 2026-11-22T18:01Z | walkthrough | - | claude-opus-5-5 (auto, inside /review-code auto) | 0 new, 0 resolved, 2 still open | auto walkthrough, not a review pass; no progress, loop stops |

## Findings

| # | Status | Axis | Affects | Description | Comments |
|---|---|---|---|---|---|
| C1 | superseded | standards | 01, .claude/skills/deploy-on-aws/SKILL.md | Possible Duplicated Code: the "Asking" section's `auto` stop list repeats by hand the stops each step already names, so it drifts when a step changes; "every stop a step names" would do. | 2026-11-22: found. 2026-11-22: needs the user (e2777aaec1): a judgement-call smell, and spec.md's 'Asking' decision (from story review R24) asks for the explicit 'among them …' list, so dropping it means overriding the spec; the waiver or a spec change is the owner's. 2026-11-22: needs the user (2140b83de4): unchanged by the C3 fix; still a waiver or scope call for the owner. 2026-11-25: superseded: spec.md's 'Asking' decision, as fixed by story review R24, asks for both 'every stop the steps name' and the explicit list; the skill copies it word for word, and the general clause covers any stop a step later adds. Owner's choice in the walkthrough. |
| C2 | resolved | spec | 01, docs/agents/commit-policy.md:15 | The "Exception, skills that commit" paragraph still says the covered skills "never push" unqualified, with the `/deploy-on-aws` exception only appended after it; issue 01 asks for the paragraph to name it "instead of saying all such skills 'never push'". | 2026-11-22: found. 2026-11-22: auto: the paragraph's 'never push' now reads 'never push, except `/deploy-on-aws` below', so it no longer contradicts the exception; docs only, no test reads the file. |
| C3 | resolved | spec | 01, .claude/skills/deploy-on-aws/SKILL.md:132 | Step 4's account check compares `source.location` with `git remote get-url origin` without normalising it, so an SSH-form remote or one without `.git` reads as "this isn't the sfab account"; the spec names the repo `gratinierer/sfab`. | 2026-11-22: found. 2026-11-22: auto: step 4 now compares source.location and the origin remote as <owner>/<name>, stripping an https/git@/ssh GitHub prefix and a trailing .git; cdk/tests/test_deploy_on_aws_skill.py passes. 2026-11-22: correction: 0149e39 was marked resolved over a failing test (its example tripped the skill's email check); the rule now reads 'take what follows github.com/ or github.com: and drop a trailing .git', and cdk/tests/test_deploy_on_aws_skill.py passes (3 passed). |
| C4 | wontfix | spec | 01, .claude/skills/deploy-on-aws/SKILL.md | Possible scope creep: behaviour the spec doesn't ask for, chiefly the "Never" bullet's new "any rewrite of `main` or `production`" and its turning Out of scope items into prohibitions, plus the build status list and the ascending three-story title form. | 2026-11-22: found. 2026-11-22: needs the user (0149e39579): the extras are safety-leaning (a 'Never' list from Out of scope, 'any rewrite of main or production'), or needed by the #22 dry read (the three-story title); keeping or cutting them is a scope call, i.e. a waiver, which only the owner can make. 2026-11-22: needs the user (33884bfd3f): unchanged by the C3 fix; still a waiver or scope call for the owner. 2026-11-25: waived by the owner, who accepted the walkthrough's reasoning: the extras only add safety (the 'Never' list, no rewrite of main or production) or fill gaps the spec leaves (CodeBuild's failure statuses, the title for three or more stories), and cutting them would make the skill weaker. |
