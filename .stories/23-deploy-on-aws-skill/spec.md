# A /deploy-on-aws skill: push, merge to production, deploy via CodeBuild

GitHub issue: [gratinierer/sfab#23](https://github.com/gratinierer/sfab/issues/23)
Authored: 2026-11-22 by claude-opus-5-5, from a grilling with the owner the same day

## Goal

Deploying today is a hand-run sequence: push `main`, open and merge a pull request from `main` to `production`, sign
in to AWS, find the `sfab-deploy` CodeBuild project, start it, and watch it (story 18, "A build"). Story 10's deploy
on 2026-11-22 went that way, and stalled once on an expired AWS session. A repo skill, `/deploy-on-aws`, runs the
sequence the same way each time, asks before every step that reaches outside the machine, and keeps account details
out of the repo and out of its output.

## Decisions

From the grilling of 2026-11-22. Each answer below is the owner's.

- **One skill, `/deploy-on-aws`,** in `.claude/skills/deploy-on-aws/`, running in order:
  1. **Ready check:** first `git fetch origin main production`, without asking: like the `sts` call in step 4 it
     only reads, and steps 1-3 compare against those refs. Then: on `main`, nothing staged, `main` not behind `origin/main`, and `scripts/check` green. If the
     check fails, stop. `skipcheck` skips it, and the report says it was skipped.
  2. **Push `main`:** ask first, unless `main` already equals `origin/main`, then skip.
  3. **Pull request and merge:** open, or reuse, the pull request from `main` to `production` and merge it with a
     merge commit, as #21 and #22 were. Ask first, unless `production` already contains `main`'s head, then skip.
  4. **AWS login, checked just before the first AWS call:**
     - If a `.env` exists at the repo root, check `git check-ignore .env` first and stop if it isn't ignored; then
       take `AWS_PROFILE` from it. It holds nothing else. Without a `.env`, use the default credentials and say in
       one line how to add one with `AWS_PROFILE` for readers with several accounts.
     - `aws sts get-caller-identity` only has to succeed; its output is never shown. On an expired session, stop
       and ask the owner to type `! aws login`, then check again.
     - **The right account** is recognised by what it holds, not by a stored ID: `us-east-1` has a CodeBuild
       project `sfab-deploy` whose source is this repo's GitHub remote (`gratinierer/sfab`). If not, stop:
       "this isn't the sfab account".
  5. **Build:** start `sfab-deploy` (`aws codebuild start-build --project-name sfab-deploy --region us-east-1`).
     Running the skill is the go-ahead, so there is no separate question; the skill says what it starts. Stack
     names given as arguments, such as `/deploy-on-aws SfabApi SfabWebsite`, go through
     `--environment-variables-override name=DEPLOY_STACKS,value=...`; `scripts/deploy` already refuses anything
     else. Several names are space-separated, so the shorthand value is quoted
     (`'name=DEPLOY_STACKS,value=SfabApi SfabWebsite'`), as `buildspec.yml` splits it unquoted. Before starting a
     build, check each name against `STACKS` and `ALL` in `cdk/deploy_plan.py`, read from the file rather than
     copied, so a typo stops on the machine instead of in a build. Wait for the build without fast polling. On failure, show the failed phase and the last ~40 lines of the
     build log.
  6. **Smoke test:** after a successful build, offer `scripts/smoke --execute-api <url> --production`. The URL is
     `https://<RestApiId>.execute-api.us-east-1.amazonaws.com/v1/`, with `RestApiId` read from the API stack's
     CloudFormation output each time (`cdk/kill_switch.py`, the outputs `docs/restart-after-a-stop.md` uses).
     There is no override. The production smoke run births nothing and cleans up its friend, block, vouch and
     message; it rewrites only the smoke identity's `whoiam` to a fixed text.
     - `scripts/smoke` runs `cdk/.venv/bin/python`, which a `skipcheck` run on a fresh clone hasn't made: if
       `cdk/.venv` is missing, make it first as `scripts/deploy` does (`uv venv` and `uv pip sync` of
       `cdk/requirements.txt`).
     - Its exit codes: 0 pass; 1 a failed smoke test; 2 a usage or config problem, such as no `smoke_claim` in
       `sfab/config`, shown with its message; 3 WAF blocked it, reported as "wait at least 5 minutes and rerun",
       not as a failed deploy. The build already succeeded in every case, and the report says so.
  7. **Report in chat only:** the commits, the pull request, the build ID and result, and what was skipped. No
     deploy log in the repo; GitHub and CodeBuild keep the history.
- **Asking:** interactive by default, as above. **`auto`** (`/deploy-on-aws auto`) asks nothing and runs push, merge,
  build and watch. It stops and reports only on what it can't fix: an expired login, the wrong account, a red
  check, `main` behind `origin/main`, a pull request that can't merge or whose checks fail, a failed build. It never
  force-pushes, skips the check only with `skipcheck`, and runs the smoke test only with `smoke`.
- **No account-specific data:** nothing account-specific goes into the repo. Every 12-digit number, ARN and email in
  what the skill shows is masked. The project name and the region are already public in the repo.
- **`.env`:** added to `.gitignore`. No `.env.example`; the skill itself says what `.env` may hold.
- **Commit policy:** `docs/agents/commit-policy.md` gets one line: running `/deploy-on-aws` is the explicit
  instruction to push `main` and merge into `production`, after asking in interactive mode and without asking in
  `auto`, as the review skills' right to commit is written there (ADR 0010).

## Out of scope

- An emergency deploy from a machine (`scripts/deploy` with the owner's credentials): the skill only points to
  `docs/restart-after-a-stop.md`.
- Rollbacks.
- Closing stories or GitHub issues.
