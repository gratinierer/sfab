"""CDK assertion tests for the base and API stacks, from spec.md "Testing".

Built from the committed fixture config, never a real sfab.config.toml, and with a
stand-in Lambda asset, so they need no build.
"""

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


def test_buckets_topics_and_budgets_are_only_in_the_base_stack(api):
    for kind in ("AWS::S3::Bucket", "AWS::SNS::Topic", "AWS::Budgets::Budget"):
        assert api.find_resources(kind) == {}, kind
    # autoDeleteObjects would add a custom resource that can empty a bucket.
    assert api.find_resources("Custom::S3AutoDeleteObjects") == {}
