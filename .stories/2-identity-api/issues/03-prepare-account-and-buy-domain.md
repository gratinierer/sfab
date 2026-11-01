# Prepare the AWS account and buy the domain

Label: ready-for-human

The owner's steps before anything can deploy: `spec.md` "Prerequisites", and steps 1-3 of "First
deploy". They need the owner's AWS account and card.

## Scope

- Decide `domain_name`.
- Set up the dedicated account in `us-east-1` on the Paid plan, with a Lambda concurrency quota of
  at least 105.
- Run `aws login` and `cdk bootstrap aws://<account_id>/us-east-1`.
- Buy the domain in the Route 53 console, with WHOIS privacy and auto-renew on, and follow any
  registrant verification email.
- Once the charge appears in Cost Explorer, read its service name.

`sfab.config.toml` and its example arrive with issue 04, so keep `domain_name` and the hosted
zone's ID until then. The API stack can't deploy without the service name, because issue 07 puts
it in the budget's filter, so record it under `## Comments` below.

## Done when

- The console shows the domain registered, with no verification pending.
- The hosted zone's ID and the charge's service name are known.
