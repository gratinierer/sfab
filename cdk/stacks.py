"""Story 2's stacks: the base stack, the API stack and the website stack (website_stack.py).

See .stories/2-identity-api/spec.md "Architecture" and "Abuse and cost controls".
"""

from __future__ import annotations

import json
import re
from dataclasses import dataclass
from pathlib import Path

import aws_cdk as cdk
import yaml
from aws_cdk import aws_apigateway as apigw
from aws_cdk import aws_budgets as budgets
from aws_cdk import aws_certificatemanager as acm
from aws_cdk import aws_cloudfront as cloudfront
from aws_cdk import aws_cloudfront_origins as origins
from aws_cdk import aws_cloudwatch as cloudwatch
from aws_cdk import aws_cloudwatch_actions as cw_actions
from aws_cdk import aws_pricingplanmanager as pricingplanmanager
from aws_cdk import aws_route53 as route53
from aws_cdk import aws_route53_targets as targets
from aws_cdk import aws_iam as iam
from aws_cdk import aws_lambda as lambda_
from aws_cdk import aws_logs as logs
from aws_cdk import aws_s3 as s3
from aws_cdk import aws_sns as sns
from aws_cdk import aws_sns_subscriptions as subs
from aws_cdk import aws_wafv2 as wafv2
from constructs import Construct

import sfab_config
from kill_switch import KillSwitch
from sfab_config import Config
from website_stack import WebsiteStack

REGION = "us-east-1"
OPENAPI = sfab_config.REPO_ROOT / "openapi" / "sfab.yaml"
# What generate_website.py writes, which scripts/check runs first.
WEBSITE_BUILD = sfab_config.REPO_ROOT / "build" / "website"

# Budgets, in USD per month. Public settings, so they live here, reviewed, not in the config.
STOP_BUDGET_USD = 35
# The kill switch's daily spend check reads this budget by name.
STOP_BUDGET_NAME = "sfab-stop"
ALERT_BUDGET_USD = (29, 40, 50, 70, 100)
# The check budget's one alert, at $0.01, written as a percentage of a $1 limit.
CHECK_BUDGET_PERCENT_OF_1_USD = 1

# PLACEHOLDER, not yet confirmed: the service name domain charges appear under, which the
# stop budget excludes. "Amazon Registrar" is spec.md "First deploy" step 3's guess from
# memory. Issue 03 records the real name from the first purchase's bill; put it here then.
DOMAIN_SERVICE_NAME = "Amazon Registrar"

# The proof of work's difficulty, which the API Lambda reads (spec.md "Proof of work").
REQUIRED_BITS = 20
# Enforced by AWS at no charge, and the kill switch sets it to 0 to stop the API.
API_RESERVED_CONCURRENCY = 5
# CDK's default is 3 seconds; a request makes a few S3 calls. The takedown waits at least this.
API_TIMEOUT_SECONDS = 10
# The identities bucket's prefixes the API Lambda reads and writes, and those it only reads.
API_READ_WRITE_PREFIXES = ("identities/", "messages/", "vitals/", "federation/pending/")
API_READ_PREFIXES = ("world/",)


STAGE = "v1"
THROTTLE_RATE = 10
THROTTLE_BURST = 20


# Per IP per 5 minutes, across all routes; WAF's lowest is 10 (spec.md "WAF limit").
WAF_RATE_LIMIT = 100
WAF_RATE_WINDOW_SECONDS = 300


class SynthError(Exception):
    """The stacks can't be built; the message says why."""


def api_host(config: Config) -> str:
    return f"api.{config.domain_name}"


def fill_openapi(text: str, *, api_lambda_arn: str, api_host: str) -> dict:
    """sfab.yaml with its two placeholders filled in memory, so no real ARN reaches the
    repo. A placeholder left over deploys fine and shows agents the raw text, so it fails."""
    filled = text.replace("{{api_lambda_arn}}", api_lambda_arn).replace("{{api_host}}", api_host)
    left = sorted(set(re.findall(r"\{\{[^}]*\}\}", filled)))
    if "{{" in filled:
        raise SynthError(
            f"{OPENAPI.name}: unfilled placeholder(s): {', '.join(left) or '{{'}"
        )
    return yaml.safe_load(filled)


def outside_v1_function(host: str) -> str:
    """A CloudFront viewer-request function: a path outside `/v1/` gets a problem+json
    404 pointing to the API description, so it never reaches API Gateway."""
    body = json.dumps(
        {
            "type": "about:blank",
            "title": "No such route",
            "status": 404,
            "detail": f"Start at https://{host}/v1/openapi.yaml",
        }
    )
    return f"""function handler(event) {{
  var request = event.request;
  if (request.uri.indexOf('/v1/') === 0) {{
    return request;
  }}
  return {{
    statusCode: 404,
    statusDescription: 'Not Found',
    headers: {{
      'content-type': {{ value: 'application/problem+json' }},
      'cache-control': {{ value: 'no-store' }}
    }},
    body: {{ encoding: 'text', data: {json.dumps(body)} }}
  }};
}}
"""


def _waf_visibility(metric_name: str) -> wafv2.CfnWebACL.VisibilityConfigProperty:
    # Metrics give blocked counts for free; samples stay off (spec.md "Logging").
    return wafv2.CfnWebACL.VisibilityConfigProperty(
        cloud_watch_metrics_enabled=True,
        metric_name=metric_name,
        sampled_requests_enabled=False,
    )


class BaseStack(cdk.Stack):
    """What must exist before anything can be spent, and outlive any API rollback."""

    def __init__(self, scope: Construct, id: str, *, config: Config, **kwargs) -> None:
        super().__init__(scope, id, **kwargs)
        # The domain is bought by hand, so no stack can buy, change or lose it; its zone is imported.
        self.hosted_zone = route53.HostedZone.from_hosted_zone_attributes(
            self, "Zone", hosted_zone_id=config.hosted_zone_id, zone_name=config.domain_name
        )
        # CloudFront needs it in us-east-1, which is where every stack is.
        self.certificate = acm.Certificate(
            self,
            "Certificate",
            domain_name=config.domain_name,
            subject_alternative_names=[api_host(config)],
            validation=acm.CertificateValidation.from_dns(self.hosted_zone),
        )

        self.identities_bucket = self._bucket("Identities")
        self.claims_bucket = self._bucket("Claims")

        # The only thing the owner's email subscribes to. Nothing on it can stop the API.
        self.alerts_topic = sns.Topic(self, "Alerts")
        self.alerts_topic.add_subscription(subs.EmailSubscription(config.alert_email))
        self._let_publish(self.alerts_topic, "budgets.amazonaws.com", "cloudwatch.amazonaws.com")
        # Receives only the stop budget's alert; any message on it stops the API (kill_switch).
        self.stop_topic = sns.Topic(self, "Stop")
        self._let_publish(self.stop_topic, "budgets.amazonaws.com")

        # Credits are excluded so sign-up credits can't hide abnormal usage. The stop budget
        # also excludes domain charges, so a purchase or renewal can't stop the API.
        no_credits = budgets.CfnBudget.ExpressionProperty(
            dimensions=budgets.CfnBudget.ExpressionDimensionValuesProperty(
                key="RECORD_TYPE", values=["Credit"]
            )
        )
        domain_charges = budgets.CfnBudget.ExpressionProperty(
            dimensions=budgets.CfnBudget.ExpressionDimensionValuesProperty(
                key="SERVICE", values=[DOMAIN_SERVICE_NAME]
            )
        )
        self._budget(
            STOP_BUDGET_NAME,
            STOP_BUDGET_USD,
            budgets.CfnBudget.ExpressionProperty(
                not_=budgets.CfnBudget.ExpressionProperty(or_=[domain_charges, no_credits])
            ),
            [(100, "PERCENTAGE")],
            self.stop_topic,
        )
        self._budget(
            "sfab-alert",
            ALERT_BUDGET_USD[-1],
            budgets.CfnBudget.ExpressionProperty(not_=no_credits),
            [(amount, "ABSOLUTE_VALUE") for amount in ALERT_BUDGET_USD],
            self.alerts_topic,
        )
        self._budget(
            "sfab-check",
            1,
            budgets.CfnBudget.ExpressionProperty(not_=no_credits),
            # 1% of $1 is $0.01: CloudFormation's schema refuses a threshold below 0.1.
            [(CHECK_BUDGET_PERCENT_OF_1_USD, "PERCENTAGE")],
            self.alerts_topic,
        )

    def _budget(
        self,
        name: str,
        limit: float,
        filter_expression: budgets.CfnBudget.ExpressionProperty,
        thresholds: list[tuple[float, str]],
        topic: sns.Topic,
    ) -> None:
        budgets.CfnBudget(
            self,
            name,
            budget=budgets.CfnBudget.BudgetDataProperty(
                budget_name=name,
                budget_type="COST",
                time_unit="MONTHLY",
                budget_limit=budgets.CfnBudget.SpendProperty(amount=limit, unit="USD"),
                metrics=["UnblendedCost"],
                filter_expression=filter_expression,
            ),
            notifications_with_subscribers=[
                budgets.CfnBudget.NotificationWithSubscribersProperty(
                    notification=budgets.CfnBudget.NotificationProperty(
                        notification_type="ACTUAL",
                        comparison_operator="GREATER_THAN",
                        threshold=threshold,
                        threshold_type=threshold_type,
                    ),
                    subscribers=[
                        budgets.CfnBudget.SubscriberProperty(
                            subscription_type="SNS", address=topic.topic_arn
                        )
                    ],
                )
                for threshold, threshold_type in thresholds
            ],
        )

    def _let_publish(self, topic: sns.Topic, *services: str) -> None:
        topic.add_to_resource_policy(
            iam.PolicyStatement(
                actions=["sns:Publish"],
                principals=[iam.ServicePrincipal(s) for s in services],
                resources=[topic.topic_arn],
                conditions={"StringEquals": {"aws:SourceAccount": self.account}},
            )
        )

    def _bucket(self, id: str) -> s3.Bucket:
        # Deleting the stack can never delete identities: retained, no autoDeleteObjects.
        return s3.Bucket(
            self,
            id,
            block_public_access=s3.BlockPublicAccess.BLOCK_ALL,
            encryption=s3.BucketEncryption.S3_MANAGED,
            enforce_ssl=True,
            versioned=False,
            removal_policy=cdk.RemovalPolicy.RETAIN,
            auto_delete_objects=False,
        )


class ApiStack(cdk.Stack):
    """The API and everything that protects it. A rollback here can't touch the base stack."""

    def __init__(
        self,
        scope: Construct,
        id: str,
        *,
        config: Config,
        base: BaseStack,
        api_code: lambda_.Code,
        kill_switch_code: lambda_.Code,
        openapi_text: str,
        **kwargs,
    ) -> None:
        super().__init__(scope, id, **kwargs)

        self.function = lambda_.Function(
            self,
            "ApiFunction",
            runtime=lambda_.Runtime.PROVIDED_AL2023,
            architecture=lambda_.Architecture.ARM_64,
            handler="bootstrap",
            code=api_code,
            environment={
                "REQUIRED_BITS": str(REQUIRED_BITS),
                "IDENTITIES_BUCKET": base.identities_bucket.bucket_name,
                "CLAIMS_BUCKET": base.claims_bucket.bucket_name,
            },
            reserved_concurrent_executions=API_RESERVED_CONCURRENCY,
            timeout=cdk.Duration.seconds(API_TIMEOUT_SECONDS),
            log_group=logs.LogGroup(
                self,
                "ApiFunctionLogs",
                retention=logs.RetentionDays.TWO_WEEKS,
                removal_policy=cdk.RemovalPolicy.DESTROY,
            ),
        )
        self._grant_storage(base)

        # Email only: any error tells the owner, and nothing here can stop the API.
        errors = self.function.metric_errors(period=cdk.Duration.minutes(5), statistic="Sum")
        errors_alarm = errors.create_alarm(
            self,
            "ApiFunctionErrors",
            threshold=1,
            evaluation_periods=1,
            comparison_operator=cloudwatch.ComparisonOperator.GREATER_THAN_OR_EQUAL_TO_THRESHOLD,
            treat_missing_data=cloudwatch.TreatMissingData.NOT_BREACHING,
        )
        errors_alarm.add_alarm_action(cw_actions.SnsAction(base.alerts_topic))

        definition = fill_openapi(
            openapi_text, api_lambda_arn=self.function.function_arn, api_host=api_host(config)
        )
        self.rest_api = apigw.SpecRestApi(
            self,
            "Api",
            api_definition=apigw.ApiDefinition.from_inline(definition),
            # Our own CloudFront is in front, so not CDK's default of EDGE.
            endpoint_types=[apigw.EndpointType.REGIONAL],
            # Access logs are off, so nothing needs the account-level CloudWatch role, and
            # CDK's default depends on a feature flag.
            cloud_watch_role=False,
            deploy_options=apigw.StageOptions(
                stage_name=STAGE,
                # One stage-wide default throttle, so a route can't be forgotten.
                throttling_rate_limit=THROTTLE_RATE,
                throttling_burst_limit=THROTTLE_BURST,
                # No execution, body or access logs, so a flood can't run up a log bill.
                logging_level=apigw.MethodLoggingLevel.OFF,
                data_trace_enabled=False,
            ),
        )
        # SpecRestApi doesn't build the model from code, so CDK's deployment hash wouldn't see
        # an edit to sfab.yaml; hashing the definition makes each edit a new deployment.
        self.rest_api.latest_deployment.add_to_logical_id(definition)
        # The origin key: CloudFront adds it, and every operation in sfab.yaml requires it.
        # It works only through a usage plan; the stage holds the throttles, so the plan has
        # none. No name, because a changed value replaces the key, and a named key can't be.
        origin_key = apigw.ApiKey(self, "OriginKey", value=config.origin_key, enabled=True)
        plan = self.rest_api.add_usage_plan(
            "OriginKeyPlan",
            api_stages=[
                apigw.UsagePlanPerApiStage(api=self.rest_api, stage=self.rest_api.deployment_stage)
            ],
        )
        plan.add_api_key(origin_key)
        # SpecRestApi can't see the integrations inside the OpenAPI document, so it adds no
        # invoke permission; without this every route fails.
        self.function.add_permission(
            "ApiGatewayInvoke",
            principal=iam.ServicePrincipal("apigateway.amazonaws.com"),
            source_arn=self.rest_api.arn_for_execute_api(),
        )

        self._front_door(config, base, origin_key=config.origin_key)

        self.kill_switch = KillSwitch(
            self,
            "KillSwitch",
            code=kill_switch_code,
            stop_topic=base.stop_topic,
            alerts_topic=base.alerts_topic,
            stop_budget_name=STOP_BUDGET_NAME,
            api_function=self.function,
            rest_api=self.rest_api,
            stage_name=STAGE,
            web_acl=self.web_acl,
        )

    def _front_door(self, config: Config, base: BaseStack, *, origin_key: str) -> None:
        """CloudFront on the Free flat-rate plan, with WAF, in front of the REST API."""
        self.web_acl = wafv2.CfnWebACL(
            self,
            "WebAcl",
            scope="CLOUDFRONT",
            # The kill switch flips this to Block (kill_switch.py).
            default_action=wafv2.CfnWebACL.DefaultActionProperty(allow={}),
            # Samples return each header for 3 hours, which could expose live claims.
            visibility_config=_waf_visibility("sfabApi"),
            rules=[
                wafv2.CfnWebACL.RuleProperty(
                    name="RatePerIp",
                    priority=0,
                    statement=wafv2.CfnWebACL.StatementProperty(
                        rate_based_statement=wafv2.CfnWebACL.RateBasedStatementProperty(
                            limit=WAF_RATE_LIMIT,
                            evaluation_window_sec=WAF_RATE_WINDOW_SECONDS,
                            aggregate_key_type="IP",
                        )
                    ),
                    # No custom response: WAF's plain 403 is what agents read as "blocked".
                    action=wafv2.CfnWebACL.RuleActionProperty(block={}),
                    visibility_config=_waf_visibility("sfabApiRatePerIp"),
                )
            ],
        )

        not_found = cloudfront.Function(
            self,
            "OutsideV1",
            runtime=cloudfront.FunctionRuntime.JS_2_0,
            code=cloudfront.FunctionCode.from_inline(outside_v1_function(api_host(config))),
            comment="A problem+json 404 for any path outside /v1/",
        )

        self.distribution = cloudfront.Distribution(
            self,
            "Distribution",
            domain_names=[api_host(config)],
            certificate=base.certificate,
            web_acl_id=self.web_acl.attr_arn,
            default_behavior=cloudfront.BehaviorOptions(
                origin=origins.RestApiOrigin(
                    self.rest_api,
                    # RestApiOrigin defaults to the stage, but public URLs already hold /v1.
                    origin_path="/",
                    custom_headers={"x-api-key": origin_key},
                ),
                # Not a redirect: that would come after the token was already sent.
                viewer_protocol_policy=cloudfront.ViewerProtocolPolicy.HTTPS_ONLY,
                allowed_methods=cloudfront.AllowedMethods.ALLOW_ALL,
                cache_policy=cloudfront.CachePolicy.CACHING_DISABLED,
                # Sends Authorization on every method.
                origin_request_policy=cloudfront.OriginRequestPolicy.ALL_VIEWER_EXCEPT_HOST_HEADER,
                function_associations=[
                    cloudfront.FunctionAssociation(
                        function=not_found, event_type=cloudfront.FunctionEventType.VIEWER_REQUEST
                    )
                ],
            ),
        )

        zone = route53.HostedZone.from_hosted_zone_attributes(
            self, "Zone", hosted_zone_id=config.hosted_zone_id, zone_name=config.domain_name
        )
        # Free: no overage charges, WAF included, and the zone's fee and queries covered.
        pricingplanmanager.CfnSubscription(
            self,
            "FlatRatePlan",
            plan_family="CloudFront",
            plan_tier="FREE",
            usage_level="DEFAULT",
            resource_arns=[
                self.distribution.distribution_arn,
                self.web_acl.attr_arn,
                zone.hosted_zone_arn,
            ],
        )

        target = route53.RecordTarget.from_alias(targets.CloudFrontTarget(self.distribution))
        route53.ARecord(self, "ApiA", zone=zone, record_name=api_host(config), target=target)
        route53.AaaaRecord(self, "ApiAaaa", zone=zone, record_name=api_host(config), target=target)

    def _grant_storage(self, base: BaseStack) -> None:
        """spec.md "Architecture": only these prefixes, never `federation/verified/`.
        Story 4 adds its own prefixes here."""
        identities, claims = base.identities_bucket, base.claims_bucket
        read_write = [identities.arn_for_objects(f"{p}*") for p in API_READ_WRITE_PREFIXES]
        read_write.append(claims.arn_for_objects("*"))
        self.function.add_to_role_policy(
            iam.PolicyStatement(actions=["s3:GetObject", "s3:PutObject"], resources=read_write)
        )
        self.function.add_to_role_policy(
            iam.PolicyStatement(
                actions=["s3:GetObject"],
                resources=[identities.arn_for_objects(f"{p}*") for p in API_READ_PREFIXES],
            )
        )
        # Without it, S3 answers a missing key with 403 rather than 404, which the Lambda
        # reads as "not found". It lists key names only, never an object's content.
        self.function.add_to_role_policy(
            iam.PolicyStatement(
                actions=["s3:ListBucket"], resources=[identities.bucket_arn, claims.bucket_arn]
            )
        )


@dataclass(frozen=True)
class Stacks:
    base: BaseStack
    api: ApiStack
    website: WebsiteStack


def build(
    app: Construct,
    config: Config,
    api_code: lambda_.Code,
    kill_switch_code: lambda_.Code,
    openapi_text: str | None = None,
    website_source: Path = WEBSITE_BUILD,
) -> Stacks:
    if openapi_text is None:
        openapi_text = OPENAPI.read_text(encoding="utf-8")
    env = cdk.Environment(account=config.account_id, region=REGION)
    base = BaseStack(app, "SfabBase", config=config, env=env)
    api = ApiStack(
        app,
        "SfabApi",
        config=config,
        base=base,
        api_code=api_code,
        kill_switch_code=kill_switch_code,
        openapi_text=openapi_text,
        env=env,
    )
    website = WebsiteStack(
        app,
        "SfabWebsite",
        config=config,
        certificate=base.certificate,
        source=website_source,
        env=env,
    )
    return Stacks(base=base, api=api, website=website)
