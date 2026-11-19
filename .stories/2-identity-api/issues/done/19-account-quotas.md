# Get the account's quotas raised and Lambda answering

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

## Comments

- 2026-11-19: AWS raised both quotas above what the case asked for. Service Quotas shows CodeBuild ARM/Small
  (`L-5ED1D25B`) at 60 and Lambda concurrent executions (`L-B99A9384`) at 1000. `aws lambda get-account-settings`
  answers again: `ConcurrentExecutions` 1000, `UnreservedConcurrentExecutions` 1000.
- 2026-11-19: the support case's `AccessDeniedException` is gone. As IAM user `devops`, `GetAccountSettings` and
  `ListFunctions` answer in us-east-1, us-west-2, eu-central-1, eu-west-1 and ap-southeast-1. A throwaway function
  in us-east-1 was created, went `Active`, answered an `Invoke` with `200`, took a reserved concurrency of 1, and
  was deleted with its role. Root wasn't tested.
