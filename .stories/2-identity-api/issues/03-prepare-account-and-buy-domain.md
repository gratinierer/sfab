# Prepare the AWS account and buy the domain

Label: ready-for-human

The owner's steps before anything can deploy: `spec.md` "Prerequisites", and steps 1-3 of "First
deploy". They need the owner's AWS account and card.

## Scope

- `domain_name` is `botrefugium.com`, decided 2026-11-04. If Route 53 can't register it, stop and
  choose again, because it's also this world's federation ID.
- Set up the dedicated account in `us-east-1` on the Paid plan, with a Lambda concurrency quota of
  at least 105.
- Secure it: MFA on the root and your user, no long-lived access keys, and only `aws login`.
- Switch on AWS Cost Anomaly Detection, emailing an address you read on your phone.
- Run `aws login` and `cdk bootstrap aws://<account_id>/us-east-1` from the repo root, not `cdk/`,
  as `spec.md` "First deploy" step 1 explains.
- Buy the domain in the Route 53 console, with WHOIS privacy and auto-renew on, and follow any
  registrant verification email.
- Once the charge appears in Cost Explorer, read its service name.

Write `domain_name` and `hosted_zone_id` into `sfab.config.toml`, copied from
`sfab.config.example.toml`, which issue 04 added. The base stack can't deploy without the service name, because issue 01 puts
it in the stop budget's filter, so record it under `## Comments` below.

## Done when

- The console shows the domain registered, with no verification pending.
- The hosted zone's ID and the charge's service name are known.
- MFA and anomaly detection are on.

## Comments

- 2026-11-04: the owner registered `botrefugium.com` in the Route 53 console. Still open from "Done when": no
  registrant verification pending, the hosted zone's ID kept until issue 04, the charge's service name once it
  appears in Cost Explorer, and MFA and anomaly detection.
