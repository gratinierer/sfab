# Prepare the AWS account and buy the domain

Label: ready-for-human
Blocked by: 18-deploy-from-codebuild#02, 18-deploy-from-codebuild#05

The owner's steps before anything but `SfabDeployer` can deploy: `spec.md` "Prerequisites", and steps 1-7 of
"First deploy". They need the owner's AWS account and card.

## Scope

- `domain_name` is `botrefugium.com`, decided 2026-11-04. If Route 53 can't register it, stop and
  choose again, because it's also this world's federation ID.
- Set up the dedicated account in `us-east-1` on the Paid plan, with a Lambda concurrency quota of
  at least 106 and a CodeBuild quota of at least 1 concurrent Arm small build.
- Secure it: MFA on the root and your user, no long-lived access keys, and only `aws login`.
- Switch on AWS Cost Anomaly Detection, emailing an address you read on your phone.
- Run `aws login` and `cdk bootstrap aws://<account>/us-east-1` from the repo root, not `cdk/`,
  as `spec.md` "First deploy" step 1 explains, then `scripts/deploy SfabDeployer`, approve the GitHub connection
  and create the `production` branch (`18-deploy-from-codebuild#05`), steps 2-4. Its protection waits for the
  repo to go public (`12-public-repo#02`), because GitHub Free refuses rulesets on private repos.
- Buy the domain in the Route 53 console, with WHOIS privacy and auto-renew on, and follow any
  registrant verification email.
- Once the charge appears in Cost Explorer, read its service name.

Fill the config secret `sfab/config`, which `SfabDeployer` created, with every entry but `hosted_zone_id` ("First
deploy" step 5), and add `hosted_zone_id` once the domain is bought. There's no config file:
`.stories/18-deploy-from-codebuild/` replaced `sfab.config.toml` with the secret. The base stack can't deploy
without the service name, because issue 01 puts it in the stop budget's filter, so record it under `## Comments`
below.

## Done when

- The console shows the domain registered, with no verification pending.
- `sfab/config` holds every entry, `hosted_zone_id` included, and the charge's service name is known.
- MFA and anomaly detection are on.
- `SfabDeployer` is deployed, its GitHub connection is approved (`AVAILABLE`), and `production` exists
  (`18-deploy-from-codebuild#05`): "First deploy" steps 2-4. Protecting it isn't part of this issue: that comes
  with `12-public-repo#02`.

## Comments

- 2026-11-04: the owner registered `botrefugium.com` in the Route 53 console. Still open from "Done when": no
  registrant verification pending, the hosted zone's ID kept until issue 04, the charge's service name once it
  appears in Cost Explorer, and MFA and anomaly detection.
- 2026-11-15: `production` is pushed to `gratinierer/sfab` (47da5df) but **unprotected**: the repo is private and
  GitHub Free refuses rulesets on private repos (HTTP 403). It gets its ruleset when the repo goes public,
  `12-public-repo#02`. The owner is the only collaborator, so the gap is an accidental force-push or deletion,
  which is recoverable.
- 2026-11-15, walkthrough with the owner:
  - **Account:** dedicated, `us-east-1`, Paid plan. MFA on root and on the owner's IAM user, which has
    `AdministratorAccess`; no access keys on either; the CLI uses `aws login` as that user.
  - **Quotas, still open:** Lambda concurrent executions is 10 and CodeBuild ARM/Small is 0, and every Lambda call
    returns `AccessDeniedException`, even for root, which looks like a new account AWS hasn't fully unrestricted.
    Service Quotas refuses both requests (Lambda only accepts values above the 1000 default; CodeBuild says "contact
    AWS Support"), so the owner opened an Account and billing case asking for Lambda 300 and CodeBuild 5. This
    issue stays open until both are raised and Lambda answers.
  - **Anomaly detection:** AWS's default monitor and daily subscription, emailing the owner's phone address; the
    threshold lowered from "≥ $100 and ≥ 40%" to an impact of ≥ $10.
  - **Deploys:** `cdk bootstrap` and `scripts/deploy SfabDeployer` succeeded; `sfab-github` is `AVAILABLE`.
  - **Domain:** registrant verification `DONE`, auto-renew and privacy on all four contacts.
  - **`sfab/config`:** every required entry, `hosted_zone_id` included; `smoke_claim` waits for launch. The
    abuse contact is a separate address from the alert address, because it gets published.
  - **Service name:** the domain charge appears as **"Amazon Registrar"**, so `DOMAIN_SERVICE_NAME` stays and
    loses its placeholder mark. Its VAT appears separately under "Tax", which the stop budget still counts; the
    owner accepts that.
  - **`aws login` and boto3:** boto3 needs the `crt` extra to use `aws login` credentials, so
    `cdk/requirements.in` now pins `boto3[crt]` (`docs/aws-facts.md`).
