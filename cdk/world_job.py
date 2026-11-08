"""The daily world job: spec.md "Daily world job", built by issue 11.

The third Rust function, beside the API Lambda and the kill switch, with its own permissions. Its
steps live in `lambda/job/` and the crates it calls; this is its wiring, permissions and
email-only errors alarm. Issue 15 added the federation checks, with the job's one outbound call
and its email through the alerts topic, and story 4 adds its daily work, each with the rights
they need.
"""

from __future__ import annotations

import aws_cdk as cdk
from aws_cdk import aws_cloudwatch as cloudwatch
from aws_cdk import aws_cloudwatch_actions as cw_actions
from aws_cdk import aws_events as events
from aws_cdk import aws_events_targets as events_targets
from aws_cdk import aws_iam as iam
from aws_cdk import aws_lambda as lambda_
from aws_cdk import aws_logs as logs
from aws_cdk import aws_s3 as s3
from aws_cdk import aws_sns as sns
from constructs import Construct

# Lambda's maximum; CDK's default of 3 seconds would cut every run short.
TIMEOUT_SECONDS = 900
# So two runs can never overlap. The kill switch sets it to 0 during a stop, and the restart
# list sets it back to this.
RESERVED_CONCURRENCY = 1
# Just after 00:00 UTC, because the Chronicle needs a complete day.
SCHEDULE = events.Schedule.cron(minute="5", hour="0")
# The identities bucket's prefixes the job reads, writes and deletes in.
WORLD_PREFIX = "world/"
FEDERATION_PREFIX = "federation/"
# Story 17: the day's world codes, the pool, the summary, and old codes and IP records.
GENESIS_PREFIX = "genesis/"


class WorldJob(Construct):
    def __init__(
        self,
        scope: Construct,
        id: str,
        *,
        code: lambda_.Code,
        identities_bucket: s3.IBucket,
        claims_bucket: s3.IBucket,
        alerts_topic: sns.ITopic,
        genesis_codes_per_day: int,
        genesis_ip_window_hours: int,
    ) -> None:
        super().__init__(scope, id)

        self.function = lambda_.Function(
            self,
            "Function",
            runtime=lambda_.Runtime.PROVIDED_AL2023,
            architecture=lambda_.Architecture.ARM_64,
            handler="bootstrap",
            code=code,
            timeout=cdk.Duration.seconds(TIMEOUT_SECONDS),
            reserved_concurrent_executions=RESERVED_CONCURRENCY,
            environment={
                "IDENTITIES_BUCKET": identities_bucket.bucket_name,
                "CLAIMS_BUCKET": claims_bucket.bucket_name,
                # Where the daily federation email goes.
                "ALERTS_TOPIC_ARN": alerts_topic.topic_arn,
                # Story 17: the day's world genesis codes, and the IP records it deletes once
                # older than the window. The settings live in stacks.py.
                "GENESIS_CODES_PER_DAY": str(genesis_codes_per_day),
                "GENESIS_IP_WINDOW_HOURS": str(genesis_ip_window_hours),
            },
            log_group=logs.LogGroup(
                self,
                "Logs",
                retention=logs.RetentionDays.TWO_WEEKS,
                removal_policy=cdk.RemovalPolicy.DESTROY,
            ),
            # Lambda would otherwise run a failed job twice more, which would mean up to 300
            # outbound fetches and three federation emails. A failure emails the owner instead.
            # A run throttled by a stop, at reserved concurrency 0, isn't a failure: Lambda keeps
            # it queued for up to 6 hours, so a restart in that time runs it once, late, which
            # is safe because every step can be repeated (docs/aws-facts.md).
            retry_attempts=0,
        )

        events.Rule(
            self,
            "Schedule",
            schedule=SCHEDULE,
            targets=[events_targets.LambdaFunction(self.function)],
        )

        self._grant(identities_bucket, claims_bucket)
        alerts_topic.grant_publish(self.function)

        errors = self.function.metric_errors(period=cdk.Duration.minutes(5), statistic="Sum")
        errors.create_alarm(
            self,
            "Errors",
            threshold=1,
            evaluation_periods=1,
            comparison_operator=cloudwatch.ComparisonOperator.GREATER_THAN_OR_EQUAL_TO_THRESHOLD,
            treat_missing_data=cloudwatch.TreatMissingData.NOT_BREACHING,
        ).add_alarm_action(cw_actions.SnsAction(alerts_topic))

    def _grant(self, identities: s3.IBucket, claims: s3.IBucket) -> None:
        """spec.md "Daily world job" ("Permissions"): list the claims bucket, without ever
        reading a claim, and read, write and delete in `world/` and `federation/`, and story 17's
        `genesis/`, listing only those. The outbound check needs no permission: the function
        isn't in a VPC."""
        prefixes = (WORLD_PREFIX, FEDERATION_PREFIX, GENESIS_PREFIX)
        statements = [
            iam.PolicyStatement(actions=["s3:ListBucket"], resources=[claims.bucket_arn]),
            iam.PolicyStatement(
                actions=["s3:ListBucket"],
                resources=[identities.bucket_arn],
                conditions={"StringLike": {"s3:prefix": [f"{p}*" for p in prefixes]}},
            ),
            iam.PolicyStatement(
                actions=["s3:GetObject", "s3:PutObject", "s3:DeleteObject"],
                resources=[identities.arn_for_objects(f"{p}*") for p in prefixes],
            ),
        ]
        for statement in statements:
            self.function.add_to_role_policy(statement)
