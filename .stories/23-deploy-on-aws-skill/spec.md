# A /deploy-on-aws skill: push, merge to production, deploy via CodeBuild

GitHub issue: [gratinierer/sfab#23](https://github.com/gratinierer/sfab/issues/23)
Authored: 2026-11-22 by claude-opus-5-5, from a grilling with the owner the same day

## Goal

Deploying today is a hand-run sequence: push `main`, open and merge a pull request from `main` to `production`, sign
in to AWS, find the `sfab-deploy` CodeBuild project, start it, and watch it (story 18, "A build"). Story 10's deploy
on 2026-11-22 went that way, and stalled once on an expired AWS session. A repo skill, `/deploy-on-aws`, runs the
sequence the same way each time, asks before the push and the merge (reads outside the machine aren't asked
about, and running the skill is the go-ahead for the build), and keeps account details
out of the repo and out of its output.

## Decisions

From the grilling of 2026-11-22. Each answer below is the owner's.

- **One skill, `/deploy-on-aws`,** in `.claude/skills/deploy-on-aws/`, running in order:
  1. **Ready check:** first `git fetch origin main production`, without asking: like the `sts` call in step 4 it
     only reads, and steps 1-3 compare against those refs. Then: on `main`, nothing staged, `main` not behind `origin/main`, and `scripts/check` green. If the
     check fails, stop. `skipcheck` skips it, and the report says it was skipped.
     - `scripts/check` builds the Rust Lambdas and runs the audit and every test, which takes minutes. So it runs
       in the background, its output to a log in the session's scratchpad, and the skill waits for it to finish,
       without polling. Only its exit code decides green or red. On red, show the last ~40 lines of the log.
     - It is a code check only: it runs before the login and without the profile, so `sfab_config.load_for_check`
       may fall back to the committed fixture, which is fine here. The build checks the real config itself
       (`scripts/deploy` runs `cdk/deploy_plan.py`, which loads `sfab/config`, then `scripts/build --require-secret`).
       When the log holds the "using the fixture" warning, the report says so in one line, so a green check isn't
       read as "config checked".
  2. **Push `main`:** ask first, unless `main` already equals `origin/main`, then skip.
  3. **Pull request and merge:** open, or reuse, the pull request from `main` to `production` and merge it with a
     merge commit, as #21 and #22 were. Ask first, unless `production` already contains `main`'s head, then skip.
     - **Title:** the story numbers from the `(#n)` endings of the commits in `origin/production..main`. One story:
       "Deploy story <n>: <its spec.md title> (#n)", the title being the `# ` line of `.stories/<n>-*/spec.md` or, for
       a closed story, `.stories/done/<n>-*/spec.md`; several: "Deploy stories <a> and <b> (#a) (#b)". None: stop
       and ask, and `auto` stops. The repo's merge commits default to "Merge pull request #N from …" (its
       `merge_commit_title` is `MERGE_MESSAGE`, as #22 and #24 show), so the merge passes the title as the subject:
       `gh pr merge <n> --merge --subject "<title>"`, and the commit carries the numbers the policy wants.
     - **Body:** "Moves `production` forward from `main` for a deploy, so `sfab-deploy` builds …", then the commit
       subjects. No `Closes #n`: `production` isn't the default branch, and closing issues is out of scope.
     - **Checks:** read them with `gh pr view <n> --json statusCheckRollup`. An empty list means none (the
       `production` ruleset requires none, and CI runs only on `workflow_dispatch`): merge straight away, because
       `gh pr checks` exits 1 with "no checks reported" then. Otherwise wait with `gh pr checks <n> --watch` and
       stop if one fails.
  4. **AWS login, checked just before the first AWS call:**
     - If a `.env` exists at the repo root, check `git check-ignore .env` first and stop if it isn't ignored; then
       take `AWS_PROFILE` from it. It holds nothing else. Without a `.env`, use the default credentials and say in
       one line how to add one with `AWS_PROFILE` for readers with several accounts.
     - `.env` is read as text, never sourced: the one line matching `^(export )?AWS_PROFILE=`, surrounding
       matching quotes stripped, and the value accepted only if it matches `^[A-Za-z0-9_.-]+$`. No such line,
       several, or another value counts as no value; when the line was there but refused, say so in one line.
     - Shell state doesn't carry between the agent's commands, so every `aws` and `scripts/smoke` call carries
       the profile itself (`AWS_PROFILE=<value> aws …`), not an earlier `export`. When there is no value (no
       `.env`, or one without an `AWS_PROFILE` line or with an empty one), every call, the smoke call in step 6
       included, drops the prefix entirely: an empty `AWS_PROFILE=` fails ("config profile () could not be
       found"). Every `aws` call also carries `--region us-east-1`, so a profile without a region still finds the
       project and the stack.
     - `aws sts get-caller-identity` only has to succeed; its output is never shown. On an expired session, stop
       and ask the owner to type `! aws login`, then check again.
     - **The right account** is recognised by what it holds, not by a stored ID: `us-east-1` has a CodeBuild
       project `sfab-deploy` whose source is this repo's GitHub remote (`gratinierer/sfab`). If not, stop:
       "this isn't the sfab account".
     - **No stop in force:** a kill switch stop sets the API Lambda's reserved concurrency to 0, and
       `docs/restart-after-a-stop.md` step 2 puts it back to 5. So read `ApiFunctionName` from the `SfabApi`
       outputs and run `aws lambda get-function-concurrency` on it; 0 means a stop is in force, and a deploy
       now can reopen part of the API. Interactive mode stops and asks, pointing to the restart doc; if the
       owner deploys anyway, the report ends with "run `docs/restart-after-a-stop.md` from step 2 now".
       Only a reserved concurrency of exactly 0 is a stop: an empty answer (no reservation set) is not, and
       neither is a missing `SfabApi` stack, as in a fresh account, where there is no API to reopen; the report
       says which of the two it found.
  5. **Build:** start `sfab-deploy` (`aws codebuild start-build --project-name sfab-deploy --region us-east-1`).
     Running the skill is the go-ahead, so there is no separate question; the skill says what it starts. Stack
     names given as arguments, such as `/deploy-on-aws SfabApi SfabWebsite`, go through
     `--environment-variables-override name=DEPLOY_STACKS,value=...`; `scripts/deploy` already refuses anything
     else. Several names are space-separated, so the shorthand value is quoted
     (`'name=DEPLOY_STACKS,value=SfabApi SfabWebsite'`), as `buildspec.yml` splits it unquoted. Before starting a
     build, check each name against `STACKS` and `ALL` in `cdk/deploy_plan.py`, so a typo stops on the machine
     instead of in a build. `STACKS` ends with `deployer_stack.STACK_NAME`, so the names come from importing the
     module, not from reading its text: `cd cdk && .venv/bin/python -c 'import deploy_plan as d;
     print(d.ALL, *d.STACKS)'`. That needs `cdk/.venv`, which a `skipcheck` run on a fresh clone hasn't made: if
     it is missing, make it first as `scripts/deploy` does (`uv venv` and `uv pip sync` of
     `cdk/requirements.txt`). Wait for the build without fast polling. On failure, show the failed phase and the
     last ~40 lines of the build log.
     - **Waiting:** `aws codebuild batch-get-builds --ids <id>` about once a minute, for the status and phases.
       Give up after 65 minutes (the project's `BUILD_TIMEOUT_MINUTES` of 60 plus a margin), reporting the build ID
       and its last status.
     - **The log lines:** `aws logs get-log-events` on the build's `logs.groupName` and `logs.streamName`, with
       `--no-start-from-head --limit 40`. CodeBuild and CDK print the account ID and ARNs, so the masking rule in
       "No account-specific data" applies to these lines before they are shown.
  6. **Smoke test:** after a successful build, offer `scripts/smoke --execute-api <url> --production`; with the
     `smoke` argument, run it without offering, in either mode. The production smoke run births nothing and cleans
     up its friend, block, vouch and message; it rewrites only the smoke identity's `whoiam` to a fixed text.
     - **The URL** is `https://<RestApiId>.execute-api.us-east-1.amazonaws.com/v1/`, with `RestApiId` read from
       the `SfabApi` stack's CloudFormation output each time (`aws cloudformation describe-stacks --stack-name
       SfabApi --region us-east-1 --query "Stacks[0].Outputs[?OutputKey=='RestApiId'].OutputValue" --output
       text`; `cdk/kill_switch.py`, the outputs `docs/restart-after-a-stop.md` uses). No argument overrides it.
       It is built rather than read because the `Endpoint` output that `cdk/smoke.py` names has a key CDK
       derives from the construct (`<Api…>Endpoint<hash>`), while `RestApiId`'s key is fixed.
     - **Running it:** a production run paces each request by `PACE_SECONDS` (`cdk/smoke.py`) and takes minutes,
       so it runs in the background and the skill waits for it to finish, without polling, rather than under a
       short command timeout that would cut it off and look like a failure. It runs `cdk/.venv/bin/python`,
       which step 5 made if it was missing.
     - **The call:** it reads the test fixture instead of `sfab/config` when `SFAB_CONFIG` is set; the secret
       read names its region itself (`SECRET_REGION` in `cdk/sfab_config.py`). So the call is `env -u
       SFAB_CONFIG AWS_PROFILE=<value> scripts/smoke …`, or `env -u SFAB_CONFIG scripts/smoke …` when step 4
       found no profile value.
     - **Exit codes:** 0 pass; 1 a failed smoke test; 2 a usage or config problem, such as no `smoke_claim` in
       `sfab/config`, shown with its message; 3 a plain `403`, which means WAF blocked this IP or the API is
       stopped, reported as "wait at least 5 minutes and rerun", not as a failed deploy; if a rerun exits 3
       again, the report points to `docs/restart-after-a-stop.md`. The build already succeeded in every case,
       and the report says so.
  7. **Report in chat only:** the commits, the pull request, the build ID and result, and what was skipped. No
     deploy log in the repo; GitHub and CodeBuild keep the history.
- **Asking:** interactive by default, as above. **`auto`** (`/deploy-on-aws auto`) asks nothing and runs push, merge,
  build and watch. It stops and reports only on what it can't fix: every stop the steps above name (among them
  `main` not checked out or behind `origin/main`, a red check, no story number for the pull request's title, a
  `.env` that isn't ignored, an expired login, the wrong account, a stop in force, an unknown stack name), a pull
  request that can't merge or whose checks fail, and a failed build. It never
  force-pushes, skips the check only with `skipcheck`, and runs the smoke test only with `smoke`.
- **No account-specific data:** nothing account-specific goes into the repo. Every 12-digit number, ARN and email in
  what the skill shows is masked. The project name and the region are already public in the repo.
- **`.env`:** added to `.gitignore`. No `.env.example`; the skill itself says what `.env` may hold.
- **Commit policy:** running `/deploy-on-aws` is the explicit instruction to push `main` and merge into
  `production`, after asking in interactive mode and without asking in `auto`. That is a deliberate exception to
  ADR 0010, which says "Pushing still always needs an explicit request", so it gets its own ADR,
  `docs/adr/0012-deploy-on-aws-may-push.md`, amending ADR 0010. ADR 0010's Status line then reads "Amended by
  ADR-0012", and `docs/agents/commit-policy.md`'s "Exception, skills that commit" paragraph names `/deploy-on-aws`
  as the one skill that may push, where it now says those skills "never push".

## Out of scope

- An emergency deploy from a machine (`scripts/deploy` with the owner's credentials): the skill only points to
  `docs/restart-after-a-stop.md`.
- Rollbacks.
- Closing stories or GitHub issues.
