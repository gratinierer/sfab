---
name: deploy-on-aws
description: "Deploy sfab: check `main`, push it, merge it into `production` through a pull request, check the AWS login and account, start the `sfab-deploy` CodeBuild build and watch it, and optionally run the production smoke test. Use only when the owner asks to deploy with `/deploy-on-aws`. Arguments: `auto`, `skipcheck`, `smoke`, and optional stack names."
disable-model-invocation: true
---

# Deploy on AWS

Runs the deploy the same way each time: push `main`, move `production` forward to it through a pull request, and
build it with the CodeBuild project `sfab-deploy` (`.stories/done/18-deploy-from-codebuild/spec.md` "A build"). Its
decisions are `.stories/23-deploy-on-aws-skill/spec.md` "Decisions"; `review.md` there says why each one is so.

## Arguments

Any order, all optional:

- **`auto`**: ask nothing; see "Asking".
- **`skipcheck`**: skip `scripts/check` in step 1. The report says it was skipped.
- **`smoke`**: run the smoke test after a successful build without offering it, in either mode (step 6).
- **Stack names**, such as `/deploy-on-aws SfabApi SfabWebsite`: deploy only those (step 5). Without any, the build
  deploys `--all`.

A keyword is never a stack name.

## Asking

- **Interactive** (the default): ask before the push (step 2) and before the merge (step 3), and wherever a step
  below says "stop and ask". Reads outside the machine (`git fetch`, `gh pr view`, `aws` reads) aren't asked about.
  Running the skill is the go-ahead for the build: step 5 says what it starts, and doesn't ask.
- **`auto`** asks nothing and runs push, merge, build and watch. It stops and reports only on what it can't fix:
  every stop the steps below name (among them `main` not checked out or behind `origin/main`, a red check, no story
  number for the pull request's title, a `.env` that isn't ignored, an expired login, the wrong account, a stop in
  force, an unknown stack name), a pull request that can't merge or whose checks fail, and a failed build. It never
  force-pushes, skips the check only with `skipcheck`, and runs the smoke test only with `smoke`.
- **Never**, in either mode: `git push --force` or any rewrite of `main` or `production`, a rollback, an emergency
  deploy from this machine (`scripts/deploy` with the owner's credentials; point to `docs/restart-after-a-stop.md`
  instead), or closing a story or GitHub issue.

Running `/deploy-on-aws` is the explicit instruction to push `main` and merge into `production`
(`docs/adr/0012-deploy-on-aws-may-push.md`, `docs/agents/commit-policy.md` "Exception, skills that commit"). It
makes no commits of its own, apart from the merge commit on `production`.

## No account-specific data

Nothing account-specific goes into the repo: this skill writes no file. In everything the skill shows, from `aws`
output and build logs to its own report, mask before showing:

- every 12-digit number (digits not next to another letter or digit) as `<account>`;
- every ARN (`arn:` up to the next space or quote) as `<arn>`;
- every email address as `<email>`.

The project name `sfab-deploy` and the region `us-east-1` are already public in the repo and stay as they are. Never
show the output of `aws sts get-caller-identity`.

## Steps

Run them in order. A stop ends the run with the report of step 7, saying where it stopped and why.

### 1. Ready check

1. `git fetch origin main production`, without asking: like the `sts` call in step 4 it only reads, and steps 1-3
   compare against those refs.
2. The current branch is `main`, nothing is staged (`git diff --cached --quiet`), and `main` is not behind
   `origin/main` (`git rev-list --count main..origin/main` is 0). Otherwise stop.
3. **`scripts/check`**, unless `skipcheck` was given. It builds the Rust Lambdas and runs the audit and every test,
   which takes minutes, so run it in the background with its output in a log in the session's scratchpad
   (`scripts/check > <scratchpad>/deploy-check.log 2>&1`), and wait for it to finish, without polling. Only its exit
   code decides green or red. On red, show the last ~40 lines of the log, masked, and stop.
   - It is a code check only: it runs before the login and without the profile, so `sfab_config.load_for_check` may
     fall back to the committed fixture, which is fine here. The build checks the real config itself
     (`scripts/deploy` runs `cdk/deploy_plan.py`, which loads `sfab/config`, then `scripts/build --require-secret`).
   - When the log holds the fallback warning (`check: the config doesn't load, so using …`, from `load_for_check`),
     the report says so in one line, so a green check isn't read as "config checked".

### 2. Push `main`

If `main` already equals `origin/main`, skip this step. Otherwise ask first (in interactive mode), then
`git push origin main`. Never with `--force`.

### 3. Pull request and merge

If `production` already contains `main`'s head (`git merge-base --is-ancestor main origin/production`), skip this
step. Otherwise open, or reuse, the pull request from `main` to `production`, and merge it with a merge commit, as
#21 and #22 were. Ask first, in interactive mode, showing the title.

- **Reuse:** `gh pr list --base production --head main --state open --json number,title`. With one open, use it;
  otherwise create it with `gh pr create --base production --head main --title "<title>" --body "<body>"`.
- **Title:** the story numbers from the `(#n)` endings of the commits in `origin/production..main`
  (`git log --format=%s origin/production..main`).
  - Every trailing `(#n)` is a candidate, since a squash merge appends the pull request's number too
    (`(#42) (#57)`); a number counts only when a story folder `.stories/<n>-*` or `.stories/done/<n>-*` exists.
  - One story: `Deploy story <n>: <its spec.md title> (#n)`, the title being the `# ` line of
    `.stories/<n>-*/spec.md` or, for a closed story, `.stories/done/<n>-*/spec.md`.
  - Several: `Deploy stories <a> and <b> (#a) (#b)`, in ascending order (`Deploy stories <a>, <b> and <c> (#a) (#b)
    (#c)` for three or more).
  - None: stop and ask what to call it; `auto` stops.
- **Body:** "Moves `production` forward from `main` for a deploy, so `sfab-deploy` builds …", naming the stories,
  then the commit subjects of `origin/production..main`, one per line. No `Closes #n`: `production` isn't the
  default branch, and closing issues is out of scope.
- **Checks:** read them with `gh pr view <n> --json statusCheckRollup`.
  - An empty list means none (the `production` ruleset requires none, and CI runs only on `workflow_dispatch`):
    merge straight away, because `gh pr checks` exits 1 with "no checks reported" then.
  - Otherwise wait with `gh pr checks <n> --watch`, in the background, and stop if one fails.
- **Merge:** the repo's merge commits default to "Merge pull request #N from …" (its `merge_commit_title` is
  `MERGE_MESSAGE`, as #22 and #24 show), so pass the title as the subject:
  `gh pr merge <n> --merge --subject "<title>"`. The merge commit then carries the numbers the commit policy wants.
  If it can't merge (a conflict, a ruleset refusal), stop and show why.
- Then `git fetch origin production`, and confirm `origin/production` contains `main`'s head.

### 4. AWS login

Checked just before the first AWS call.

- **`.env`:** if a `.env` exists at the repo root, first run `git check-ignore .env`, and stop if it isn't
  ignored. Then take `AWS_PROFILE` from it. It holds nothing else.
  - Read it as text, never `source` it: the one line matching `^(export )?AWS_PROFILE=`, surrounding matching
    quotes (`"…"` or `'…'`) stripped, and the value accepted only if it matches `^[A-Za-z0-9_.-]+$`.
  - No such line, several, or another value counts as no value. When the line was there but refused, say so in
    one line.
  - Without a `.env`, use the default credentials, and say in one line how to add one: a `.env` at the repo root
    with the single line `AWS_PROFILE=<profile>`, for readers with several accounts.
- **Every call carries the profile itself.** Shell state doesn't carry between the agent's commands, so every
  `aws` and `scripts/smoke` call is written `AWS_PROFILE=<value> aws …`, never after an earlier `export`. When there
  is no value (no `.env`, or one without an `AWS_PROFILE` line or with an empty one), every call, the smoke call in
  step 6 included, drops the prefix entirely: an empty `AWS_PROFILE=` fails ("config profile () could not be
  found"). Every `aws` call also carries `--region us-east-1`, so a profile without a region still finds the
  project and the stack. Below, `aws …` stands for this form.
- **Login:** `aws sts get-caller-identity --region us-east-1 > /dev/null` only has to succeed; its output is never
  shown. On an expired session, stop and ask the owner to type `! aws login`, then check again. `auto` stops.
- **The right account** is recognised by what it holds, not by a stored ID: `us-east-1` has a CodeBuild project
  `sfab-deploy` whose source is this repo's GitHub remote (`gratinierer/sfab`):
  `aws codebuild batch-get-projects --names sfab-deploy --region us-east-1 --query 'projects[0].source.location'
  --output text` is `https://github.com/<owner>/<name>.git` (`cdk/deployer_stack.py`), with `<owner>/<name>` the
  one `git remote get-url origin` names. If not, or there is no such project, stop: "this isn't the sfab
  account".
- **No stop in force:** a kill switch stop sets the API Lambda's reserved concurrency to 0, and
  `docs/restart-after-a-stop.md` step 2 puts it back to 5.
  - Read `ApiFunctionName` from the `SfabApi` outputs (`aws cloudformation describe-stacks --stack-name SfabApi
    --region us-east-1 --query "Stacks[0].Outputs[?OutputKey=='ApiFunctionName'].OutputValue" --output text`) and
    run `aws lambda get-function-concurrency --function-name <it> --region us-east-1`.
  - `ReservedConcurrentExecutions` of 0 means a stop is in force, and a deploy now can reopen part of the API.
    Interactive mode stops and asks, pointing to `docs/restart-after-a-stop.md`; if the owner deploys anyway, the
    report ends with "run `docs/restart-after-a-stop.md` from step 2 now". `auto` stops.
  - Only a reserved concurrency of exactly 0 is a stop: an empty answer (no reservation set) is not, and neither is
    a missing `SfabApi` stack, as in a fresh account, where there is no API to reopen. The report says which of
    the two it found.

### 5. Build

- **Stack names:** before starting a build, check each name given as an argument against `STACKS` and `ALL` in
  `cdk/deploy_plan.py`, so a typo stops on this machine instead of in a build. `STACKS` ends with
  `deployer_stack.STACK_NAME`, so read the names by importing the module, not from its text:
  `cd cdk && .venv/bin/python -c 'import deploy_plan as d; print(d.ALL, *d.STACKS)'`.
  - That needs `cdk/.venv`, which a `skipcheck` run on a fresh clone hasn't made: if it is missing, make it first
    as `scripts/deploy` does (`(cd cdk && uv venv -q .venv)`, then `uv pip sync -q --python cdk/.venv/bin/python
    --require-hashes cdk/requirements.txt`). Step 6 uses it too, so make it whenever it's missing.
  - An unknown name stops the run, naming the valid ones.
- **Start it:** running the skill is the go-ahead, so there is no separate question; say what it starts, then
  `aws codebuild start-build --project-name sfab-deploy --region us-east-1`.
  - With stack names, add `--environment-variables-override 'name=DEPLOY_STACKS,value=<names>'`, the names
    space-separated, such as `'name=DEPLOY_STACKS,value=SfabApi SfabWebsite'`. The shorthand value is quoted
    because `buildspec.yml` splits it unquoted; `scripts/deploy` already refuses anything else.
  - Keep the build ID (`build.id`) from the answer.
- **Waiting:** without fast polling: `aws codebuild batch-get-builds --ids <id> --region us-east-1` about once a
  minute, for `buildStatus`, `currentPhase` and `phases`. Wait between reads with a background wait, not a busy
  loop. Give up after 65 minutes (the project's `BUILD_TIMEOUT_MINUTES` of 60 plus a margin), reporting the build
  ID and its last status.
- **On failure** (`FAILED`, `FAULT`, `STOPPED` or `TIMED_OUT`): show the failed phase (the first in `phases` whose
  `phaseStatus` isn't `SUCCEEDED`, with its `contexts` message) and the last ~40 lines of the build log:
  `aws logs get-log-events --log-group-name <logs.groupName> --log-stream-name <logs.streamName>
  --no-start-from-head --limit 40 --region us-east-1`. CodeBuild and CDK print the account ID and ARNs, so the
  masking of "No account-specific data" applies to these lines before they are shown. Stop.

### 6. Smoke test

After a successful build, offer `scripts/smoke --execute-api <url> --production`; with the `smoke` argument, run it
without offering, in either mode. In `auto` without `smoke`, don't run it. The production smoke run births nothing
and cleans up its friend, block, vouch and message; it rewrites only the smoke identity's `whoiam` to a fixed text.

- **The URL** is `https://<RestApiId>.execute-api.us-east-1.amazonaws.com/v1/`, with `RestApiId` read from the
  `SfabApi` stack's CloudFormation output each time: `aws cloudformation describe-stacks --stack-name SfabApi
  --region us-east-1 --query "Stacks[0].Outputs[?OutputKey=='RestApiId'].OutputValue" --output text`
  (`cdk/kill_switch.py`, the outputs `docs/restart-after-a-stop.md` uses). No argument overrides it. It is built
  rather than read because the `Endpoint` output that `cdk/smoke.py` names has a key CDK derives from the
  construct (`<Api…>Endpoint<hash>`), while `RestApiId`'s key is fixed.
- **Running it:** a production run paces each request by `PACE_SECONDS` (`cdk/smoke.py`) and takes minutes, so run
  it in the background, its output to a log in the scratchpad, and wait for it to finish, without polling, rather
  than under a short command timeout that would cut it off and look like a failure. It runs
  `cdk/.venv/bin/python`, which step 5 made if it was missing.
- **The call:** it reads the test fixture instead of `sfab/config` when `SFAB_CONFIG` is set; the secret read names
  its region itself (`SECRET_REGION` in `cdk/sfab_config.py`). So the call is
  `env -u SFAB_CONFIG AWS_PROFILE=<value> scripts/smoke --execute-api <url> --production`, or
  `env -u SFAB_CONFIG scripts/smoke --execute-api <url> --production` when step 4 found no profile value.
- **Exit codes** (`scripts/smoke`). The build already succeeded in every case, and the report says so.
  - 0: pass.
  - 1: a failed smoke test; show the end of its log, masked.
  - 2: a usage or config problem, such as no `smoke_claim` in `sfab/config`, shown with its message.
  - 3: a plain `403`, which means WAF blocked this IP or the API is stopped. Report it as "wait at least 5
    minutes and rerun", not as a failed deploy. If a rerun exits 3 again, the report points to
    `docs/restart-after-a-stop.md`.

### 7. Report

In chat only, masked: the commits deployed (`origin/production..main` before the merge), the pull request (number
and title), the build ID and result, the smoke result if run, and what was skipped (the check with `skipcheck`, an
already-pushed `main`, an already-merged `production`, the smoke test), plus the one-line notes the steps ask for
(the fixture fallback, no `.env` or a refused `AWS_PROFILE` line, which of the no-stop cases step 4 found). On a
stop, say at which step and why. When the owner deployed during a stop, end with "run
`docs/restart-after-a-stop.md` from step 2 now".

No deploy log in the repo: GitHub and CodeBuild keep the history.
