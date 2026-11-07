"""CDK assertion tests for the kill switch, from spec.md "Testing" and issue 07.

Built once, from the committed fixture config with stand-in Lambda assets, in conftest.py.
"""

import json

import pytest
from aws_cdk.assertions import Template

from test_stacks import api_function, topic_id


@pytest.fixture(scope="module")
def base(built):
    return Template.from_stack(built.base)


@pytest.fixture(scope="module")
def api(built):
    return Template.from_stack(built.api)


def kill_switch(api: Template) -> tuple[str, dict]:
    functions = {
        k: v
        for k, v in api.find_resources("AWS::Lambda::Function").items()
        if k.startswith("KillSwitch")
    }
    assert len(functions) == 1, list(functions)
    return next(iter(functions.items()))


def actions(api: Template, role_prefix: str) -> dict[str, list]:
    """Every action the role's policies allow, with the resources it's allowed on."""
    found: dict[str, list] = {}
    for policy in api.find_resources("AWS::IAM::Policy").values():
        if role_prefix not in json.dumps(policy["Properties"]["Roles"]):
            continue
        for statement in policy["Properties"]["PolicyDocument"]["Statement"]:
            assert statement["Effect"] == "Allow"
            names = statement["Action"]
            names = [names] if isinstance(names, str) else names
            resources = statement["Resource"]
            resources = resources if isinstance(resources, list) else [resources]
            for name in names:
                found.setdefault(name, []).extend(resources)
    return found


# The function


def test_the_kill_switch_runs_its_own_rust_binary_on_arm64(api):
    _, function = kill_switch(api)
    props = function["Properties"]
    assert props["Architectures"] == ["arm64"]
    assert props["Runtime"] == "provided.al2023"
    assert props["Handler"] == "bootstrap"
    # CDK's default of 3 seconds is too short for a WAF update and a stage deployment.
    assert props["Timeout"] >= 30


def test_the_kill_switch_keeps_logs_for_14_days(api):
    _, function = kill_switch(api)
    group = function["Properties"]["LoggingConfig"]["LogGroup"]["Ref"]
    assert api.find_resources("AWS::Logs::LogGroup")[group]["Properties"]["RetentionInDays"] == 14


def test_the_kill_switch_keeps_lambda_s_retries_so_a_stop_gets_through(api):
    function_id, _ = kill_switch(api)
    configs = api.find_resources("AWS::Lambda::EventInvokeConfig").values()
    assert not [c for c in configs if function_id in json.dumps(c["Properties"]["FunctionName"])]


def test_the_kill_switch_gets_the_daily_limit_and_what_it_stops(api, base):
    api_id, _ = api_function(api)
    _, function = kill_switch(api)
    env = function["Properties"]["Environment"]["Variables"]
    assert env["DAILY_CALL_LIMIT"] == "100000"
    assert env["API_FUNCTION_NAME"] == {"Ref": api_id}
    assert env["WEB_ACL"] == {"Ref": next(iter(api.find_resources("AWS::WAFv2::WebACL")))}
    assert env["REST_API_ID"] == {"Ref": next(iter(api.find_resources("AWS::ApiGateway::RestApi")))}
    assert env["STAGE_NAME"] == "v1"
    assert env["STOP_BUDGET_NAME"] == "sfab-stop"
    assert topic_id(base, "Alerts") in json.dumps(env["ALERTS_TOPIC_ARN"])
    assert "ACCOUNT_ID" in env


def test_the_api_stack_outputs_what_the_restart_list_needs(api):
    # docs/restart-after-a-stop.md reads these by name.
    api_id, _ = api_function(api)
    outputs = api.to_json()["Outputs"]
    assert outputs["ApiFunctionName"]["Value"] == {"Ref": api_id}
    rest_api = next(iter(api.find_resources("AWS::ApiGateway::RestApi")))
    assert outputs["RestApiId"]["Value"] == {"Ref": rest_api}
    web_acl = next(iter(api.find_resources("AWS::WAFv2::WebACL")))
    assert outputs["WebAcl"]["Value"] == {"Ref": web_acl}


# What triggers it


def test_the_kill_switch_is_subscribed_to_the_stop_topic(api, base):
    function_id, _ = kill_switch(api)
    subscriptions = api.find_resources("AWS::SNS::Subscription")
    assert len(subscriptions) == 1
    props = next(iter(subscriptions.values()))["Properties"]
    assert props["Protocol"] == "lambda"
    assert topic_id(base, "Stop") in json.dumps(props["TopicArn"])
    assert function_id in json.dumps(props["Endpoint"])
    permissions = [
        p["Properties"]
        for p in api.find_resources("AWS::Lambda::Permission").values()
        if p["Properties"]["Principal"] == "sns.amazonaws.com"
    ]
    assert len(permissions) == 1
    assert topic_id(base, "Stop") in json.dumps(permissions[0]["SourceArn"])


def schedules(api: Template) -> dict[str, dict]:
    """Each of the kill switch's rules' schedule, with the input it sends."""
    function_id, _ = kill_switch(api)
    found = {}
    for rule in api.find_resources("AWS::Events::Rule").values():
        props = rule["Properties"]
        (target,) = props["Targets"]
        if function_id not in json.dumps(target["Arn"]):
            continue
        found[props["ScheduleExpression"]] = json.loads(target["Input"])
    return found


def test_the_kill_switch_checks_the_daily_limit_every_5_minutes_and_spend_at_noon_utc(api):
    assert schedules(api) == {
        "rate(5 minutes)": {"check": "daily-limit"},
        "cron(0 12 * * ? *)": {"check": "spend"},
    }


# Its permissions


def test_the_kill_switch_may_only_stop_the_api_read_its_inputs_and_email(api, base):
    allowed = actions(api, "KillSwitchFunctionServiceRole")
    assert set(allowed) == {
        "wafv2:GetWebACL",
        "wafv2:UpdateWebACL",
        "lambda:GetFunctionConcurrency",
        "lambda:PutFunctionConcurrency",
        "apigateway:GET",
        "apigateway:PATCH",
        "apigateway:POST",
        "cloudwatch:GetMetricData",
        "budgets:ViewBudget",
        "sns:Publish",
    }
    api_id, _ = api_function(api)
    for action in ("lambda:GetFunctionConcurrency", "lambda:PutFunctionConcurrency"):
        assert api_id in json.dumps(allowed[action]), action
    assert "sfab-stop" in json.dumps(allowed["budgets:ViewBudget"])
    assert topic_id(base, "Alerts") in json.dumps(allowed["sns:Publish"])
    assert topic_id(base, "Stop") not in json.dumps(allowed)


def test_the_kill_switch_can_never_change_a_budget(api):
    allowed = actions(api, "KillSwitchFunctionServiceRole")
    assert not any(a.startswith("budgets:") and a != "budgets:ViewBudget" for a in allowed)
    assert not any("*" in a for a in allowed)


def test_the_api_lambda_has_none_of_the_kill_switch_s_rights(api):
    allowed = actions(api, "ApiFunctionServiceRole")
    assert all(a.startswith("s3:") for a in allowed), sorted(allowed)


def test_the_kill_switch_has_none_of_the_api_lambda_s_rights(api):
    allowed = actions(api, "KillSwitchFunctionServiceRole")
    assert not any(a.startswith("s3:") for a in allowed)


# Its alarms, which only email


def alarms_on(api: Template, metric: str) -> list[dict]:
    return [
        a["Properties"]
        for a in api.find_resources("AWS::CloudWatch::Alarm").values()
        if a["Properties"].get("MetricName") == metric
    ]


def test_the_kill_switch_errors_alarm_emails_only_the_alerts_topic(api, base):
    function_id, _ = kill_switch(api)
    alarms = [
        a
        for a in alarms_on(api, "Errors")
        if a["Dimensions"] == [{"Name": "FunctionName", "Value": {"Ref": function_id}}]
    ]
    assert len(alarms) == 1
    alarm = alarms[0]
    assert alarm["Namespace"] == "AWS/Lambda"
    assert alarm["Statistic"] == "Sum"
    assert alarm["ComparisonOperator"] == "GreaterThanOrEqualToThreshold"
    assert alarm["Threshold"] == 1
    assert len(alarm["AlarmActions"]) == 1
    assert topic_id(base, "Alerts") in json.dumps(alarm["AlarmActions"])


def test_the_50000_call_warning_counts_the_utc_day_and_can_t_reach_the_stop_topic(api, base):
    (alarm,) = alarms_on(api, "Count")
    assert alarm["Namespace"] == "AWS/ApiGateway"
    assert alarm["Dimensions"] == [{"Name": "ApiName", "Value": "Api"}]
    # AWS documents Count's statistic as SampleCount.
    assert alarm["Statistic"] == "SampleCount"
    assert alarm["Period"] == 86400
    assert alarm["EvaluationPeriods"] == 1
    assert alarm["ComparisonOperator"] == "GreaterThanOrEqualToThreshold"
    assert alarm["Threshold"] == 50000
    # Aligned to midnight UTC, like the daily limit, rather than a rolling 24 hours.
    assert alarm["EvaluationWindow"] == {"WallClockWindow": {"Timezone": "UTC"}}
    assert len(alarm["AlarmActions"]) == 1
    assert topic_id(base, "Alerts") in json.dumps(alarm["AlarmActions"])
    every_action = json.dumps(
        [alarm.get(k) for k in ("AlarmActions", "OKActions", "InsufficientDataActions")]
    )
    assert topic_id(base, "Stop") not in every_action

