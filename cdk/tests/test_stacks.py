"""CDK assertion tests for the base and API stacks, from spec.md "Testing".

Built from the committed fixture config, never a real sfab.config.toml, and with a
stand-in Lambda asset, so they need no build.
"""

import json
from pathlib import Path

import aws_cdk as cdk
import pytest
from aws_cdk import aws_lambda as lambda_
from aws_cdk.assertions import Match, Template

import sfab_config
import stacks

FIXTURE = Path(__file__).resolve().parent / "fixtures" / "sfab.config.test.toml"


@pytest.fixture(scope="module")
def config():
    return sfab_config.load(FIXTURE)


@pytest.fixture(scope="module")
def built(config, tmp_path_factory):
    asset = tmp_path_factory.mktemp("api")
    (asset / "bootstrap").write_text("stand-in")
    app = cdk.App()
    return stacks.build(app, config, api_code=lambda_.Code.from_asset(str(asset)))


@pytest.fixture(scope="module")
def base(built):
    return Template.from_stack(built.base)


@pytest.fixture(scope="module")
def api(built):
    return Template.from_stack(built.api)


def test_every_stack_targets_the_config_account_in_us_east_1(built, config):
    for stack in (built.base, built.api):
        assert stack.account == config.account_id
        assert stack.region == "us-east-1"


# The base stack: buckets


def test_the_base_stack_holds_two_private_retained_buckets(base):
    buckets = base.find_resources("AWS::S3::Bucket")
    assert len(buckets) == 2
    for bucket in buckets.values():
        assert bucket["DeletionPolicy"] == "Retain"
        assert bucket["UpdateReplacePolicy"] == "Retain"
        props = bucket["Properties"]
        assert props["PublicAccessBlockConfiguration"] == {
            "BlockPublicAcls": True,
            "BlockPublicPolicy": True,
            "IgnorePublicAcls": True,
            "RestrictPublicBuckets": True,
        }
        assert props["BucketEncryption"] == {
            "ServerSideEncryptionConfiguration": [
                {"ServerSideEncryptionByDefault": {"SSEAlgorithm": "AES256"}}
            ]
        }
        assert "VersioningConfiguration" not in props


# The base stack: topics


def topic_id(template: Template, name: str) -> str:
    """The logical ID of the topic whose ID starts with `name`, such as `Alerts`."""
    ids = [k for k in template.find_resources("AWS::SNS::Topic") if k.startswith(name)]
    assert len(ids) == 1, ids
    return ids[0]


def publishers(template: Template, topic: str) -> set[str]:
    """The services each policy on `topic` lets publish."""
    found = set()
    for policy in template.find_resources("AWS::SNS::TopicPolicy").values():
        if {"Ref": topic} not in policy["Properties"]["Topics"]:
            continue
        for statement in policy["Properties"]["PolicyDocument"]["Statement"]:
            if statement["Effect"] == "Allow" and statement["Action"] == "sns:Publish":
                service = statement["Principal"]["Service"]
                found |= {service} if isinstance(service, str) else set(service)
    return found


def test_there_are_two_topics_and_only_the_alerts_topic_emails_the_owner(base, config):
    assert len(base.find_resources("AWS::SNS::Topic")) == 2
    subscriptions = base.find_resources("AWS::SNS::Subscription")
    assert [s["Properties"] for s in subscriptions.values()] == [
        {
            "Protocol": "email",
            "Endpoint": config.alert_email,
            "TopicArn": {"Ref": topic_id(base, "Alerts")},
        }
    ]


def test_the_alerts_topic_lets_budgets_and_cloudwatch_publish(base):
    assert publishers(base, topic_id(base, "Alerts")) == {
        "budgets.amazonaws.com",
        "cloudwatch.amazonaws.com",
    }


def test_the_stop_topic_lets_only_budgets_publish(base):
    assert publishers(base, topic_id(base, "Stop")) == {"budgets.amazonaws.com"}


# The base stack: certificate


def test_the_certificate_covers_the_domain_and_the_api_validated_in_the_zone(base, config):
    base.resource_count_is("AWS::CertificateManager::Certificate", 1)
    base.has_resource_properties(
        "AWS::CertificateManager::Certificate",
        {
            "DomainName": config.domain_name,
            "SubjectAlternativeNames": [f"api.{config.domain_name}"],
            "ValidationMethod": "DNS",
            "DomainValidationOptions": Match.array_with(
                [{"DomainName": config.domain_name, "HostedZoneId": config.hosted_zone_id}]
            ),
        },
    )


# The base stack: budgets

NO_CREDITS = {"Dimensions": {"Key": "RECORD_TYPE", "Values": ["Credit"]}}


def budget(template: Template, name: str) -> dict:
    found = [
        b["Properties"]
        for b in template.find_resources("AWS::Budgets::Budget").values()
        if b["Properties"]["Budget"]["BudgetName"] == name
    ]
    assert len(found) == 1, name
    return found[0]


def notifications(props: dict) -> list[tuple]:
    """(type, operator, threshold, threshold type, subscribers) per notification."""
    return [
        (
            n["Notification"]["NotificationType"],
            n["Notification"]["ComparisonOperator"],
            n["Notification"]["Threshold"],
            n["Notification"]["ThresholdType"],
            n["Subscribers"],
        )
        for n in props["NotificationsWithSubscribers"]
    ]


def sns_to(template: Template, topic: str) -> list[dict]:
    return [{"SubscriptionType": "SNS", "Address": {"Ref": topic_id(template, topic)}}]


def test_there_are_exactly_three_budgets(base):
    names = {
        b["Properties"]["Budget"]["BudgetName"]
        for b in base.find_resources("AWS::Budgets::Budget").values()
    }
    assert names == {"sfab-stop", "sfab-alert", "sfab-check"}


def test_every_budget_measures_unblended_monthly_cost_without_cost_types(base):
    for name in ("sfab-stop", "sfab-alert", "sfab-check"):
        data = budget(base, name)["Budget"]
        assert data["BudgetType"] == "COST"
        assert data["TimeUnit"] == "MONTHLY"
        assert data["Metrics"] == ["UnblendedCost"]
        assert "CostTypes" not in data and "CostFilters" not in data


def test_the_stop_budget_excludes_credits_and_domain_charges(base):
    data = budget(base, "sfab-stop")["Budget"]
    assert data["BudgetLimit"] == {"Amount": 35, "Unit": "USD"}
    assert data["FilterExpression"] == {
        "Not": {
            "Or": [
                {"Dimensions": {"Key": "SERVICE", "Values": ["Amazon Registrar"]}},
                NO_CREDITS,
            ]
        }
    }


def test_the_stop_budget_alerts_only_the_stop_topic_at_100_percent(base):
    assert notifications(budget(base, "sfab-stop")) == [
        ("ACTUAL", "GREATER_THAN", 100, "PERCENTAGE", sns_to(base, "Stop"))
    ]


def test_the_alert_budget_emails_at_five_thresholds_including_domain_charges(base):
    props = budget(base, "sfab-alert")
    assert props["Budget"]["FilterExpression"] == {"Not": NO_CREDITS}
    assert notifications(props) == [
        ("ACTUAL", "GREATER_THAN", amount, "ABSOLUTE_VALUE", sns_to(base, "Alerts"))
        for amount in (29, 40, 50, 70, 100)
    ]


def test_the_check_budget_emails_at_one_cent_including_domain_charges(base):
    props = budget(base, "sfab-check")
    assert props["Budget"]["FilterExpression"] == {"Not": NO_CREDITS}
    assert notifications(props) == [
        ("ACTUAL", "GREATER_THAN", 0.01, "ABSOLUTE_VALUE", sns_to(base, "Alerts"))
    ]


# The API stack: the API Lambda


def api_function(template: Template) -> tuple[str, dict]:
    functions = {
        k: v
        for k, v in template.find_resources("AWS::Lambda::Function").items()
        if k.startswith("ApiFunction")
    }
    assert len(functions) == 1, list(functions)
    return next(iter(functions.items()))


def bucket_export(base: Template, name: str) -> str:
    """The bucket's logical ID in the base stack, which an import names."""
    ids = [k for k in base.find_resources("AWS::S3::Bucket") if k.startswith(name)]
    assert len(ids) == 1, ids
    return ids[0]


def test_the_api_lambda_runs_the_rust_binary_on_arm64_with_reserved_concurrency(api):
    _, function = api_function(api)
    props = function["Properties"]
    assert props["Architectures"] == ["arm64"]
    assert props["Runtime"] == "provided.al2023"
    assert props["Handler"] == "bootstrap"
    assert props["ReservedConcurrentExecutions"] == 5


def test_the_api_lambda_gets_required_bits_and_both_buckets(api, base):
    _, function = api_function(api)
    env = function["Properties"]["Environment"]["Variables"]
    assert set(env) == {"REQUIRED_BITS", "IDENTITIES_BUCKET", "CLAIMS_BUCKET"}
    assert env["REQUIRED_BITS"] == "20"
    assert bucket_export(base, "Identities") in str(env["IDENTITIES_BUCKET"])
    assert bucket_export(base, "Claims") in str(env["CLAIMS_BUCKET"])


def test_the_api_lambda_keeps_logs_for_14_days(api):
    _, function = api_function(api)
    group = function["Properties"]["LoggingConfig"]["LogGroup"]["Ref"]
    assert api.find_resources("AWS::Logs::LogGroup")[group]["Properties"]["RetentionInDays"] == 14


def bucket_resource(base: Template, resource) -> str:
    """An S3 resource in a policy as `<bucket>` or `<bucket>/<key pattern>`, with the
    bucket named by its logical ID in the base stack, such as `Identities/world/*`."""
    text = json.dumps(resource)
    names = [n for n in ("Identities", "Claims") if bucket_export(base, n) in text]
    assert len(names) == 1, text
    if isinstance(resource, dict) and "Fn::Join" in resource:
        tail = "".join(p for p in resource["Fn::Join"][1] if isinstance(p, str))
        return names[0] + tail
    return names[0]


def grants(api: Template, base: Template, role_prefix: str) -> set[tuple[str, str]]:
    """Every (action, resource) the role's policies allow, with S3 resources named."""
    found = set()
    for policy in api.find_resources("AWS::IAM::Policy").values():
        roles = json.dumps(policy["Properties"]["Roles"])
        if role_prefix not in roles:
            continue
        for statement in policy["Properties"]["PolicyDocument"]["Statement"]:
            assert statement["Effect"] == "Allow"
            actions = statement["Action"]
            actions = [actions] if isinstance(actions, str) else actions
            resources = statement["Resource"]
            resources = resources if isinstance(resources, list) else [resources]
            for action in actions:
                for resource in resources:
                    if action.startswith("s3:"):
                        resource = bucket_resource(base, resource)
                    found.add((action, json.dumps(resource) if isinstance(resource, dict) else resource))
    return found


def test_the_api_lambda_may_touch_only_its_own_prefixes(api, base):
    read_write = [
        "Identities/identities/*",
        "Identities/messages/*",
        "Identities/vitals/*",
        "Identities/federation/pending/*",
        "Claims/*",
    ]
    expected = {(a, r) for r in read_write for a in ("s3:GetObject", "s3:PutObject")}
    expected |= {("s3:GetObject", "Identities/world/*")}
    # Without ListBucket, S3 answers a missing key with 403 instead of 404 (docs/aws-facts.md).
    expected |= {("s3:ListBucket", "Identities"), ("s3:ListBucket", "Claims")}
    assert grants(api, base, "ApiFunctionServiceRole") == expected


def test_the_api_lambda_errors_alarm_emails_only_the_alerts_topic(api, base):
    function_id, _ = api_function(api)
    alarms = [
        a["Properties"]
        for a in api.find_resources("AWS::CloudWatch::Alarm").values()
        if a["Properties"].get("MetricName") == "Errors"
    ]
    assert len(alarms) == 1
    alarm = alarms[0]
    assert alarm["Namespace"] == "AWS/Lambda"
    assert alarm["Dimensions"] == [{"Name": "FunctionName", "Value": {"Ref": function_id}}]
    assert alarm["Statistic"] == "Sum"
    assert alarm["ComparisonOperator"] == "GreaterThanOrEqualToThreshold"
    assert alarm["Threshold"] == 1
    assert alarm["TreatMissingData"] == "notBreaching"
    actions = json.dumps(alarm["AlarmActions"])
    assert len(alarm["AlarmActions"]) == 1
    assert topic_id(base, "Alerts") in actions
    assert "OKActions" not in alarm and "InsufficientDataActions" not in alarm


def test_buckets_topics_and_budgets_are_only_in_the_base_stack(api):
    for kind in ("AWS::S3::Bucket", "AWS::SNS::Topic", "AWS::Budgets::Budget"):
        assert api.find_resources(kind) == {}, kind
    # autoDeleteObjects would add a custom resource that can empty a bucket.
    assert api.find_resources("Custom::S3AutoDeleteObjects") == {}
