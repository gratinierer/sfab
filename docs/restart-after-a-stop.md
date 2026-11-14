# Restarting the API after a stop

The kill switch stops the API and emails you, naming the trigger: daily limit, budget alert or daily
spend check. Restarting is never automatic. This is the restart list from
`.stories/2-identity-api/spec.md` "Kill switch" as AWS CLI commands, in order. Every step can safely be
repeated. Read "After a stop" under "Operations" in the same spec first.

Don't deploy during a stop, by a build of `sfab-deploy` or `scripts/deploy` from a machine, without running this
list afterwards, from step 2, because a deploy can reopen part of the API.

## 0. Look up the names

Everything is in `us-east-1`, and WAF's `CLOUDFRONT` scope only works there.

```bash
export AWS_REGION=us-east-1
out() {
  aws cloudformation describe-stacks --stack-name SfabApi \
    --query "Stacks[0].Outputs[?OutputKey=='$1'].OutputValue" --output text
}
API_FUNCTION=$(out ApiFunctionName)
JOB_FUNCTION=$(out JobFunctionName)
REST_API_ID=$(out RestApiId)
# The web ACL's Ref is name|id|scope.
IFS='|' read -r WEB_ACL_NAME WEB_ACL_ID _ <<<"$(out WebAcl)"
```

## 1. Look at what happened

```bash
# Today's calls, since midnight UTC, against the daily limit in cdk/kill_switch.py.
API_NAME=$(aws apigateway get-rest-api --rest-api-id "$REST_API_ID" --query name --output text)
aws cloudwatch get-metric-statistics --namespace AWS/ApiGateway --metric-name Count \
  --dimensions Name=ApiName,Value="$API_NAME" --statistics SampleCount --period 86400 \
  --start-time "$(date -u +%Y-%m-%dT00:00:00Z)" --end-time "$(date -u +%Y-%m-%dT%H:%M:%SZ)"

# This month's spend against the stop budget.
aws budgets describe-budget --account-id "$(aws sts get-caller-identity --query Account --output text)" \
  --budget-name sfab-stop --query 'Budget.[CalculatedSpend.ActualSpend,BudgetLimit]'
```

If today's total is over the limit and you want the API up today, raise `DAILY_CALL_LIMIT` in
`cdk/kill_switch.py` and deploy, or the next check stops it again within 5 minutes. If spend is at or
above the budget, raise `STOP_BUDGET_USD` in `cdk/stacks.py` and deploy, and lower it the same way
later. A deploy is a build: commit the change, move `production` forward to it, and run
`aws codebuild start-build --project-name sfab-deploy`, then wait for the build to succeed before step 2. Staying down until an attack ends is a valid choice too.

## 2. The API Lambda's reserved concurrency back to 5

This ends the stop, so it comes first: otherwise the next check would put the other steps back.

```bash
aws lambda put-function-concurrency --function-name "$API_FUNCTION" \
  --reserved-concurrent-executions 5
```

## 3. The daily world job's reserved concurrency back to 1

After step 2, or the next check would set it back to 0. If the stop began before the job's run
just after 00:00 UTC, Lambda may still hold that run, for up to 6 hours, and starts it once this
step is done, which is safe because every step of the job can be repeated.

```bash
aws lambda put-function-concurrency --function-name "$JOB_FUNCTION" \
  --reserved-concurrent-executions 1
```

## 4. The `execute-api` endpoint back, and the stage deployed

CloudFront's requests go through this endpoint, so it has to be back before WAF opens.

```bash
aws apigateway update-rest-api --rest-api-id "$REST_API_ID" \
  --patch-operations op=replace,path=/disableExecuteApiEndpoint,value=false
aws apigateway create-deployment --rest-api-id "$REST_API_ID" --stage-name v1 \
  --description "sfab restart"
```

## 5. WAF's default action back to Allow

This comes last, so agents keep getting the plain `403` until everything behind it works.
`update-web-acl` replaces the whole web ACL, so it sends back the rules and every setting it read that
the kill switch also keeps (`Step::Waf` in `lambda/killswitch/src/main.rs`), which needs jq 1.6 or later for `IN`.

```bash
aws wafv2 get-web-acl --scope CLOUDFRONT --name "$WEB_ACL_NAME" --id "$WEB_ACL_ID" > web-acl.json
jq '.LockToken as $lock | .WebACL
    | {Name, Id, Scope: "CLOUDFRONT", LockToken: $lock, DefaultAction: {Allow: {}},
       Rules, VisibilityConfig}
      + with_entries(select(.value != null and (.key | IN("Description", "DataProtectionConfig",
          "CustomResponseBodies", "CaptchaConfig", "ChallengeConfig", "TokenDomains",
          "AssociationConfig"))))' web-acl.json > web-acl-update.json
aws wafv2 update-web-acl --cli-input-json file://web-acl-update.json
rm web-acl.json web-acl-update.json
```

## 6. Run the smoke script

The full one before launch, and `smoke --production` after it, which writes nothing public.
