"""The kill switch and the daily limit: spec.md "Kill switch", built by issue 07.

A small Rust function that runs the same full stop whatever triggers it: the 5-minute check of
the day's calls, any message on the stop topic, or the daily spend check. Its rules live in
`lambda/killswitch/`; this is its wiring, permissions and email-only alarms.
"""

from __future__ import annotations

from typing import cast

import aws_cdk as cdk
from aws_cdk import aws_apigateway as apigw
from aws_cdk import aws_cloudwatch as cloudwatch
from aws_cdk import aws_cloudwatch_actions as cw_actions
from aws_cdk import aws_events as events
from aws_cdk import aws_events_targets as events_targets
from aws_cdk import aws_iam as iam
from aws_cdk import aws_lambda as lambda_
from aws_cdk import aws_logs as logs
from aws_cdk import aws_sns as sns
from aws_cdk import aws_sns_subscriptions as subs
from aws_cdk import aws_wafv2 as wafv2
from constructs import Construct

# API Gateway calls per UTC day. More than this stops the API until the owner restarts it. A
# public setting, so it lives here, reviewed, and changing it takes a commit and a deploy.
DAILY_CALL_LIMIT = 100_000
# Emails the owner, and can never stop anything.
DAILY_CALL_WARNING = 50_000
# A WAF update and a stage deployment take seconds each; CDK's default of 3 is too short.
TIMEOUT_SECONDS = 60


class KillSwitch(Construct):
    def __init__(
        self,
        scope: Construct,
        id: str,
        *,
        code: lambda_.Code,
        stop_topic: sns.ITopic,
        alerts_topic: sns.ITopic,
        stop_budget_name: str,
        api_function: lambda_.Function,
        job_function: lambda_.Function,
        rest_api: apigw.SpecRestApi,
        stage_name: str,
        web_acl: wafv2.CfnWebACL,
    ) -> None:
        super().__init__(scope, id)
        stack = cdk.Stack.of(self)

        self.function = lambda_.Function(
            self,
            "Function",
            runtime=lambda_.Runtime.PROVIDED_AL2023,
            architecture=lambda_.Architecture.ARM_64,
            handler="bootstrap",
            code=code,
            timeout=cdk.Duration.seconds(TIMEOUT_SECONDS),
            environment={
                "DAILY_CALL_LIMIT": str(DAILY_CALL_LIMIT),
                "API_FUNCTION_NAME": api_function.function_name,
                # The fourth stop step pauses the daily world job (world_job.py).
                "JOB_FUNCTION_NAME": job_function.function_name,
                # The web ACL's Ref is `name|id|scope`, which the function splits.
                "WEB_ACL": web_acl.ref,
                "REST_API_ID": rest_api.rest_api_id,
                "STAGE_NAME": stage_name,
                "ACCOUNT_ID": stack.account,
                "STOP_BUDGET_NAME": stop_budget_name,
                "ALERTS_TOPIC_ARN": alerts_topic.topic_arn,
            },
            log_group=logs.LogGroup(
                self,
                "Logs",
                retention=logs.RetentionDays.TWO_WEEKS,
                removal_policy=cdk.RemovalPolicy.DESTROY,
            ),
            # No retry settings: Lambda's default retries stay, because a stop must get through.
        )

        # Any message on the stop topic stops the API; only the stop budget publishes there.
        stop_topic.add_subscription(subs.LambdaSubscription(self.function))
        for name, schedule, check in (
            ("DailyLimitCheck", events.Schedule.rate(cdk.Duration.minutes(5)), "daily-limit"),
            # Once a day, because AWS sends an actual-spend alert only once per budget period.
            ("SpendCheck", events.Schedule.cron(minute="0", hour="12"), "spend"),
        ):
            events.Rule(
                self,
                name,
                schedule=schedule,
                targets=[
                    events_targets.LambdaFunction(
                        self.function,
                        event=events.RuleTargetInput.from_object({"check": check}),
                    )
                ],
            )

        self._grant(
            stack, stop_budget_name, alerts_topic, [api_function, job_function], rest_api, web_acl
        )
        self._alarms(alerts_topic, rest_api)

        # What the restart list in docs/restart-after-a-stop.md looks up, under fixed names.
        for name, value in (
            ("ApiFunctionName", api_function.function_name),
            ("JobFunctionName", job_function.function_name),
            ("RestApiId", rest_api.rest_api_id),
            ("WebAcl", web_acl.ref),
        ):
            cdk.CfnOutput(self, name, value=value).override_logical_id(name)

    def _grant(
        self,
        stack: cdk.Stack,
        stop_budget_name: str,
        alerts_topic: sns.ITopic,
        functions: list[lambda_.Function],
        rest_api: apigw.SpecRestApi,
        web_acl: wafv2.CfnWebACL,
    ) -> None:
        """Only what a stop and its checks need. It reads the budget and can never change it."""
        rest_api_arn = stack.format_arn(
            service="apigateway",
            account="",
            resource="/restapis",
            resource_name=rest_api.rest_api_id,
            arn_format=cdk.ArnFormat.SLASH_RESOURCE_NAME,
        )
        statements = [
            (["wafv2:GetWebACL", "wafv2:UpdateWebACL"], [web_acl.attr_arn]),
            (
                ["lambda:GetFunctionConcurrency", "lambda:PutFunctionConcurrency"],
                [f.function_arn for f in functions],
            ),
            # GetRestApi, GetStage and GetDeployment; UpdateRestApi; CreateDeployment.
            (
                ["apigateway:GET"],
                [rest_api_arn, f"{rest_api_arn}/stages/*", f"{rest_api_arn}/deployments/*"],
            ),
            (["apigateway:PATCH"], [rest_api_arn]),
            (["apigateway:POST"], [f"{rest_api_arn}/deployments"]),
            # GetMetricData has no resource-level permissions.
            (["cloudwatch:GetMetricData"], ["*"]),
            (
                ["budgets:ViewBudget"],
                [
                    stack.format_arn(
                        service="budgets",
                        region="",
                        resource="budget",
                        resource_name=stop_budget_name,
                    )
                ],
            ),
            (["sns:Publish"], [alerts_topic.topic_arn]),
        ]
        for actions, resources in statements:
            self.function.add_to_role_policy(
                iam.PolicyStatement(actions=actions, resources=resources)
            )

    def _alarms(self, alerts_topic: sns.ITopic, rest_api: apigw.SpecRestApi) -> None:
        """Email only, on the alerts topic: nothing here can stop the API."""
        email = cw_actions.SnsAction(alerts_topic)

        # A failed step means a stop is incomplete; a failed read means nothing was checked.
        errors = self.function.metric_errors(period=cdk.Duration.minutes(5), statistic="Sum")
        errors.create_alarm(
            self,
            "Errors",
            threshold=1,
            evaluation_periods=1,
            comparison_operator=cloudwatch.ComparisonOperator.GREATER_THAN_OR_EQUAL_TO_THRESHOLD,
            treat_missing_data=cloudwatch.TreatMissingData.NOT_BREACHING,
        ).add_alarm_action(email)

        # AWS documents Count's statistic as SampleCount. The function reads the API's name from API
        # Gateway, while this uses the name CDK gives it, which issue 10's alarm check confirms.
        warning = rest_api.metric_count(
            period=cdk.Duration.days(1), statistic="SampleCount"
        ).create_alarm(
            self,
            "DailyCallWarning",
            threshold=DAILY_CALL_WARNING,
            evaluation_periods=1,
            comparison_operator=cloudwatch.ComparisonOperator.GREATER_THAN_OR_EQUAL_TO_THRESHOLD,
            treat_missing_data=cloudwatch.TreatMissingData.NOT_BREACHING,
            alarm_description=(
                f"{DAILY_CALL_WARNING} API Gateway calls today (UTC); the kill switch stops the "
                f"API above {DAILY_CALL_LIMIT}."
            ),
        )
        warning.add_alarm_action(email)
        # Aligned to midnight UTC, like the daily limit, rather than a rolling 24 hours.
        cfn = cast(cloudwatch.CfnAlarm, warning.node.default_child)
        cfn.evaluation_window = cloudwatch.CfnAlarm.EvaluationWindowProperty(
            wall_clock_window=cloudwatch.CfnAlarm.WallClockWindowProperty(timezone="UTC")
        )

