"""CDK assertion tests for the base and API stacks, from spec.md "Testing".

Built from the committed fixture config, never a real sfab.config.toml, and with a
stand-in Lambda asset and website folder, so they need no build. The website stack's
own tests are in test_website_stack.py.
"""

import json
import subprocess
from pathlib import Path

import aws_cdk as cdk
import pytest
from aws_cdk import aws_lambda as lambda_
from aws_cdk.assertions import Match, Template

import sfab_config
import stacks

FIXTURE = Path(__file__).resolve().parent / "fixtures" / "sfab.config.test.toml"
# The same context `cdk synth` reads from cdk.json.
CDK_CONTEXT = json.loads((Path(__file__).resolve().parent.parent / "cdk.json").read_text())[
    "context"
]


@pytest.fixture(scope="module")
def config():
    return sfab_config.load(FIXTURE)


@pytest.fixture(scope="module")
def built(config, tmp_path_factory):
    asset = tmp_path_factory.mktemp("api")
    (asset / "bootstrap").write_text("stand-in")
    site = tmp_path_factory.mktemp("website")
    (site / "index.html").write_text("stand-in")
    app = cdk.App()
    code = lambda_.Code.from_asset(str(asset))
    return stacks.build(
        app, config, api_code=code, kill_switch_code=code, website_source=site
    )


@pytest.fixture(scope="module")
def base(built):
    return Template.from_stack(built.base)


@pytest.fixture(scope="module")
def api(built):
    return Template.from_stack(built.api)


def test_the_stacks_pass_cloudformation_s_schema_rules(config, tmp_path):
    # A value outside a resource's schema, such as a budget threshold under 0.1, would
    # otherwise only warn at synth and fail at deploy.
    asset = tmp_path / "api"
    asset.mkdir()
    (asset / "bootstrap").write_text("stand-in")
    site = tmp_path / "website"
    site.mkdir()
    (site / "index.html").write_text("stand-in")
    app = cdk.App(context=CDK_CONTEXT, outdir=str(tmp_path / "out"))
    code = lambda_.Code.from_asset(str(asset))
    stacks.build(app, config, api_code=code, kill_switch_code=code, website_source=site)
    app.synth()


def test_every_stack_targets_the_config_account_in_us_east_1(built, config):
    for stack in (built.base, built.api, built.website):
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
    # 1% of $1: CloudFormation's schema refuses a threshold below 0.1 (docs/aws-facts.md).
    assert props["Budget"]["BudgetLimit"] == {"Amount": 1, "Unit": "USD"}
    assert notifications(props) == [
        ("ACTUAL", "GREATER_THAN", 1, "PERCENTAGE", sns_to(base, "Alerts"))
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
        and a["Properties"]["Dimensions"][0]["Value"] == {"Ref": function_id}
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


# The API stack: the REST API from openapi/sfab.yaml


def rest_api_body(api: Template) -> str:
    apis = api.find_resources("AWS::ApiGateway::RestApi")
    assert len(apis) == 1
    return json.dumps(next(iter(apis.values()))["Properties"]["Body"])


def test_the_rest_api_is_built_from_sfab_yaml_with_both_placeholders_filled(api, config):
    body = rest_api_body(api)
    assert "{{" not in body
    assert f"Start at https://api.{config.domain_name}/v1/openapi.yaml" in body
    function_id, _ = api_function(api)
    assert '"Fn::GetAtt": ["%s", "Arn"]' % function_id in body
    assert "/identity/birth" in body


def test_a_placeholder_left_in_the_definition_fails_the_synth():
    text = "a: '{{api_lambda_arn}}'\nb: '{{api_host}}'\nc: '{{something_new}}'\n"
    with pytest.raises(stacks.SynthError, match="something_new"):
        stacks.fill_openapi(text, api_lambda_arn="arn", api_host="host")


def test_the_rest_api_is_regional(api):
    api.has_resource_properties(
        "AWS::ApiGateway::RestApi", {"EndpointConfiguration": {"Types": ["REGIONAL"]}}
    )


def test_there_is_no_cloudwatch_role_because_access_logs_are_off(api):
    api.resource_count_is("AWS::ApiGateway::Account", 0)
    assert not [k for k in api.find_resources("AWS::IAM::Role") if "CloudWatchRole" in k]


def test_the_v1_stage_has_one_default_throttle_and_no_logs(api):
    stages = api.find_resources("AWS::ApiGateway::Stage")
    assert len(stages) == 1
    stage = next(iter(stages.values()))["Properties"]
    assert stage["StageName"] == "v1"
    assert "AccessLogSetting" not in stage
    assert stage["MethodSettings"] == [
        {
            "DataTraceEnabled": False,
            "HttpMethod": "*",
            "LoggingLevel": "OFF",
            "ResourcePath": "/*",
            "ThrottlingBurstLimit": 20,
            "ThrottlingRateLimit": 10,
        }
    ]


def test_api_gateway_may_invoke_the_api_lambda(api):
    function_id, _ = api_function(api)
    permissions = [
        p["Properties"]
        for p in api.find_resources("AWS::Lambda::Permission").values()
        if p["Properties"]["Principal"] == "apigateway.amazonaws.com"
    ]
    assert len(permissions) == 1
    permission = permissions[0]
    assert permission["Action"] == "lambda:InvokeFunction"
    assert permission["FunctionName"] == {"Fn::GetAtt": [function_id, "Arn"]}
    assert "execute-api" in json.dumps(permission["SourceArn"])


def deployment_id(template: Template) -> str:
    deployments = template.find_resources("AWS::ApiGateway::Deployment")
    assert len(deployments) == 1
    return next(iter(deployments))


def test_an_edit_to_sfab_yaml_gives_a_new_deployment(config, tmp_path):
    asset = tmp_path / "api"
    asset.mkdir()
    (asset / "bootstrap").write_text("stand-in")
    source = stacks.OPENAPI.read_text(encoding="utf-8")

    def synth(text: str) -> str:
        built = stacks.build(
            cdk.App(),
            config,
            api_code=lambda_.Code.from_asset(str(asset)),
            kill_switch_code=lambda_.Code.from_asset(str(asset)),
            openapi_text=text,
            website_source=asset,
        )
        return deployment_id(Template.from_stack(built.api))

    assert synth(source) == synth(source)
    assert synth(source) != synth(source.replace("Create an identity", "Make an identity"))


def test_the_origin_key_is_unnamed_and_in_a_usage_plan_on_v1_without_a_throttle(api, config):
    keys = api.find_resources("AWS::ApiGateway::ApiKey")
    assert len(keys) == 1
    key_id, key = next(iter(keys.items()))
    # A named key can't be replaced, and changing Value replaces it.
    assert "Name" not in key["Properties"]
    assert key["Properties"]["Value"] == config.origin_key
    assert key["Properties"]["Enabled"] is True

    plans = api.find_resources("AWS::ApiGateway::UsagePlan")
    assert len(plans) == 1
    plan_id, plan = next(iter(plans.items()))
    assert "Throttle" not in plan["Properties"] and "Quota" not in plan["Properties"]
    (stage,) = plan["Properties"]["ApiStages"]
    assert not stage.get("Throttle")  # CDK writes an empty map
    stage_id = next(iter(api.find_resources("AWS::ApiGateway::Stage")))
    assert stage["Stage"] == {"Ref": stage_id}

    api.has_resource_properties(
        "AWS::ApiGateway::UsagePlanKey",
        {"KeyId": {"Ref": key_id}, "KeyType": "API_KEY", "UsagePlanId": {"Ref": plan_id}},
    )


# The API stack: the front door

CACHING_DISABLED = "4135ea2d-6df8-44a3-9df3-4b5a84be39ad"
ALL_VIEWER_EXCEPT_HOST_HEADER = "b689b0a8-53d0-40ab-baf2-68738e2966ac"


def only(template: Template, kind: str) -> tuple[str, dict]:
    found = template.find_resources(kind)
    assert len(found) == 1, (kind, list(found))
    logical_id, resource = next(iter(found.items()))
    return logical_id, resource["Properties"]


def test_the_web_acl_rate_limits_each_ip_with_waf_s_plain_403_and_no_samples(api):
    _, acl = only(api, "AWS::WAFv2::WebACL")
    assert acl["Scope"] == "CLOUDFRONT"
    assert acl["DefaultAction"] == {"Allow": {}}
    assert acl["VisibilityConfig"]["SampledRequestsEnabled"] is False
    (rule,) = acl["Rules"]
    assert rule["Statement"] == {
        "RateBasedStatement": {"Limit": 100, "EvaluationWindowSec": 300, "AggregateKeyType": "IP"}
    }
    # No custom response: Free has none, and agents read a plain 403 as "blocked".
    assert rule["Action"] == {"Block": {}}
    assert rule["VisibilityConfig"]["SampledRequestsEnabled"] is False
    assert "CustomResponseBodies" not in acl


def test_the_distribution_passes_every_request_uncached_over_https_only(api, config):
    _, props = only(api, "AWS::CloudFront::Distribution")
    dist = props["DistributionConfig"]
    assert dist["Aliases"] == [f"api.{config.domain_name}"]
    behavior = dist["DefaultCacheBehavior"]
    assert behavior["ViewerProtocolPolicy"] == "https-only"
    assert sorted(behavior["AllowedMethods"]) == sorted(
        ["GET", "HEAD", "OPTIONS", "PUT", "PATCH", "POST", "DELETE"]
    )
    assert behavior["CachePolicyId"] == CACHING_DISABLED
    assert behavior["OriginRequestPolicyId"] == ALL_VIEWER_EXCEPT_HOST_HEADER
    acl_id, _ = only(api, "AWS::WAFv2::WebACL")
    assert dist["WebACLId"] == {"Fn::GetAtt": [acl_id, "Arn"]}
    function_id, _ = only(api, "AWS::CloudFront::Function")
    assert behavior["FunctionAssociations"] == [
        {"EventType": "viewer-request", "FunctionARN": {"Fn::GetAtt": [function_id, "FunctionARN"]}}
    ]


def test_the_origin_is_the_api_without_a_doubled_stage_and_with_the_origin_key(api, config):
    _, props = only(api, "AWS::CloudFront::Distribution")
    (origin,) = props["DistributionConfig"]["Origins"]
    assert "execute-api" in json.dumps(origin["DomainName"])
    # Public URLs already hold /v1, so an origin path of /v1 would send /v1/v1/...
    assert origin.get("OriginPath", "") == ""
    assert origin["CustomOriginConfig"]["OriginProtocolPolicy"] == "https-only"
    assert origin["OriginCustomHeaders"] == [
        {"HeaderName": "x-api-key", "HeaderValue": config.origin_key}
    ]


def run_viewer_function(code: str, uri: str) -> dict:
    script = code + "\nconsole.log(JSON.stringify(handler({request: {uri: %s, method: 'GET'}})));" % (
        json.dumps(uri)
    )
    out = subprocess.run(["node", "-e", script], capture_output=True, text=True, check=True)
    return json.loads(out.stdout)


@pytest.mark.parametrize("uri", ["/", "/v2/identity/self", "/v1", "/favicon.ico", "/v1x/a"])
def test_the_viewer_function_answers_a_path_outside_v1_with_a_problem_404(api, config, uri):
    _, props = only(api, "AWS::CloudFront::Function")
    assert props["FunctionConfig"]["Runtime"] == "cloudfront-js-2.0"
    response = run_viewer_function(props["FunctionCode"], uri)
    assert response["statusCode"] == 404
    assert response["headers"]["content-type"] == {"value": "application/problem+json"}
    body = json.loads(response["body"]["data"])
    assert body["status"] == 404
    assert body["type"] == "about:blank"
    assert body["detail"] == f"Start at https://api.{config.domain_name}/v1/openapi.yaml"


@pytest.mark.parametrize("uri", ["/v1/", "/v1/identity/self", "/v1/openapi.yaml"])
def test_the_viewer_function_passes_a_path_inside_v1(api, uri):
    _, props = only(api, "AWS::CloudFront::Function")
    assert run_viewer_function(props["FunctionCode"], uri) == {"uri": uri, "method": "GET"}


def test_the_free_flat_rate_plan_covers_the_distribution_web_acl_and_zone(api, config):
    _, plan = only(api, "AWS::PricingPlanManager::Subscription")
    assert plan["PlanFamily"] == "CloudFront"
    assert plan["PlanTier"] == "FREE"
    arns = json.dumps(plan["ResourceArns"])
    dist_id, _ = only(api, "AWS::CloudFront::Distribution")
    acl_id, _ = only(api, "AWS::WAFv2::WebACL")
    assert len(plan["ResourceArns"]) == 3
    assert dist_id in arns and acl_id in arns
    assert f"hostedzone/{config.hosted_zone_id}" in arns


def test_api_dns_records_alias_the_distribution(api, config):
    dist_id, _ = only(api, "AWS::CloudFront::Distribution")
    records = [r["Properties"] for r in api.find_resources("AWS::Route53::RecordSet").values()]
    assert sorted(r["Type"] for r in records) == ["A", "AAAA"]
    for record in records:
        assert record["Name"] == f"api.{config.domain_name}."
        assert record["HostedZoneId"] == config.hosted_zone_id
        assert dist_id in json.dumps(record["AliasTarget"]["DNSName"])


def test_buckets_topics_and_budgets_are_only_in_the_base_stack(api):
    for kind in ("AWS::S3::Bucket", "AWS::SNS::Topic", "AWS::Budgets::Budget"):
        assert api.find_resources(kind) == {}, kind
    # autoDeleteObjects would add a custom resource that can empty a bucket.
    assert api.find_resources("Custom::S3AutoDeleteObjects") == {}


def test_the_api_stack_outputs_the_stage_url_the_smoke_script_takes(api):
    # `cdk deploy` prints it, so the owner can pass it as `scripts/smoke --execute-api`.
    endpoints = [
        o["Value"] for name, o in api.find_outputs("*").items() if name.startswith("ApiEndpoint")
    ]
    assert len(endpoints) == 1
    assert ".execute-api." in json.dumps(endpoints[0])
    assert "DeploymentStagev1" in json.dumps(endpoints[0])
