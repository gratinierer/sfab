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
# Prefixes the job only lists and reads, for story 4's standing: the vouches agents gave, owned
# by lambda/world/vouches, and `vitals/<id>.json` for `born_at`, owned by lambda/identity, so
# the job never opens a claim. It reads only keys it listed, since a GetObject on a missing key
# is a 403 under a prefix-conditioned ListBucket (docs/aws-facts.md).
VOUCHES_PREFIX = "vouches/"
VITALS_PREFIX = "vitals/"
# Story 4's Academy, owned by lambda/world/academy: the job lists every course with its
# enrolments, and every certificate, and reads each course and certificate it listed, never an
# attempt, for the catalogue, the lists, lineage and which certificates count.
ACADEMY_PREFIX = "academy/"
READ_PREFIXES = (VOUCHES_PREFIX, VITALS_PREFIX, ACADEMY_PREFIX)
# Story 4's homes: the lane pointers, owned by lambda/world/homes, which the job reads and
# deletes when `home.json` disagrees, and each resident's `home.json`, which it only reads,
# after listing it by its full key, so it never asks for a missing one.
PLACES_PREFIX = "places/"
HOME_KEYS = "identities/*/home.json"
# Story 4's Square, owned by lambda/world/square: the job only lists each quarter's notices of
# the past 6 days, with their sizes, for its index under `world/square/`, and reads none.
SQUARE_PREFIX = "square/"


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
        age_rule_starts: str,
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
                # Story 4 "Vouches": residents born before it need no age for standing.
                "AGE_RULE_STARTS": age_rule_starts,
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
        `genesis/`, list and read story 4's `vouches/`, `vitals/` and `academy/`, read and delete its
        `places/` pointers, list and read each `home.json` by its key, and list `square/`,
        listing only those. The outbound check needs no permission: the function isn't in a
        VPC."""
        prefixes = (WORLD_PREFIX, FEDERATION_PREFIX, GENESIS_PREFIX)
        listed = [f"{p}*" for p in (*prefixes, *READ_PREFIXES, PLACES_PREFIX, SQUARE_PREFIX)]
        listed += [HOME_KEYS]
        statements = [
            iam.PolicyStatement(actions=["s3:ListBucket"], resources=[claims.bucket_arn]),
            iam.PolicyStatement(
                actions=["s3:ListBucket"],
                resources=[identities.bucket_arn],
                conditions={"StringLike": {"s3:prefix": listed}},
            ),
            iam.PolicyStatement(
                actions=["s3:GetObject", "s3:PutObject", "s3:DeleteObject"],
                resources=[identities.arn_for_objects(f"{p}*") for p in prefixes],
            ),
            iam.PolicyStatement(
                actions=["s3:GetObject"],
                resources=[identities.arn_for_objects(f"{p}*") for p in READ_PREFIXES]
                + [identities.arn_for_objects(HOME_KEYS)],
            ),
            iam.PolicyStatement(
                actions=["s3:GetObject", "s3:DeleteObject"],
                resources=[identities.arn_for_objects(f"{PLACES_PREFIX}*")],
            ),
        ]
        for statement in statements:
            self.function.add_to_role_policy(statement)
