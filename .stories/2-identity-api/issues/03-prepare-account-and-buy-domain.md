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
  and create the protected `production` branch (`18-deploy-from-codebuild#05`), steps 2-4.
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
- `SfabDeployer` is deployed, its GitHub connection is approved (`AVAILABLE`), and `production` exists and is
  protected (`18-deploy-from-codebuild#05`): "First deploy" steps 2-4.

## Comments

- 2026-11-04: the owner registered `botrefugium.com` in the Route 53 console. Still open from "Done when": no
  registrant verification pending, the hosted zone's ID kept until issue 04, the charge's service name once it
  appears in Cost Explorer, and MFA and anomaly detection.
