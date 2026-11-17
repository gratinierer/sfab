# Get the account's quotas raised and Lambda answering

Label: ready-for-human
Blocked by:

Issue 03 closed on 2026-11-17 with these still open, and nothing can deploy through `production` without them
(`spec.md` "Prerequisites"). The owner's AWS support case (Account and billing) asks for all three, because
Service Quotas refuses both requests on this new account.

## Scope

- CodeBuild "Concurrently running builds for ARM/Small environment" (`L-5ED1D25B`) raised from 0 to at least 1
  (the case asks for 5), or no `production` build can run.
- Lambda "Concurrent executions" (`L-B99A9384`) raised from 10 to at least 106 (the case asks for 300): the stacks
  reserve 6, and AWS keeps 100 unreserved.
- Lambda calls answering again: every one, `GetAccountSettings` included, returns `AccessDeniedException`, even as
  root.

## Done when

- Service Quotas shows both applied values at or above the minimums, and `aws lambda get-account-settings` answers,
  recorded under `## Comments`.
