"""CDK assertion tests for the daily world job, from spec.md "Daily world job" and "Testing",
and issue 11: its schedule, timeout, reserved concurrency, no retries and permissions, and the
kill switch's right to pause it.

Built once, from the committed fixture config with stand-in Lambda assets, in conftest.py.
"""

import json

import pytest
from aws_cdk.assertions import Template

from test_kill_switch import actions, kill_switch
from test_stacks import api_function, grants, topic_id


@pytest.fixture(scope="module")
def base(built):
    return Template.from_stack(built.base)


@pytest.fixture(scope="module")
def api(built):
    return Template.from_stack(built.api)


def world_job(api: Template) -> tuple[str, dict]:
    functions = {
        k: v
        for k, v in api.find_resources("AWS::Lambda::Function").items()
        if k.startswith("WorldJob")
    }
    assert len(functions) == 1, list(functions)
    return next(iter(functions.items()))


def test_there_are_three_rust_functions_on_arm64(api):
    functions = [
        f
        for f in api.find_resources("AWS::Lambda::Function").values()
        if f["Properties"].get("Runtime") == "provided.al2023"
    ]
    assert len(functions) == 3
    for f in functions:
        assert f["Properties"]["Architectures"] == ["arm64"]
        assert f["Properties"]["Handler"] == "bootstrap"


def test_the_job_has_lambda_s_maximum_timeout_and_never_overlaps(api):
    _, function = world_job(api)
    props = function["Properties"]
    # CDK's default is 3 seconds.
    assert props["Timeout"] == 900
    assert props["ReservedConcurrentExecutions"] == 1


def test_a_failed_run_is_never_retried(api):
    function_id, _ = world_job(api)
    configs = [
        c["Properties"]
        for c in api.find_resources("AWS::Lambda::EventInvokeConfig").values()
        if c["Properties"]["FunctionName"] == {"Ref": function_id}
    ]
    assert len(configs) == 1
    assert configs[0]["MaximumRetryAttempts"] == 0


def test_the_job_runs_just_after_midnight_utc(api):
    function_id, _ = world_job(api)
    rules = [
        r["Properties"]
        for r in api.find_resources("AWS::Events::Rule").values()
        if function_id in json.dumps(r["Properties"]["Targets"])
    ]
    assert len(rules) == 1
    assert rules[0]["ScheduleExpression"] == "cron(5 0 * * ? *)"


def test_the_job_keeps_logs_for_14_days(api):
    _, function = world_job(api)
    group = function["Properties"]["LoggingConfig"]["LogGroup"]["Ref"]
    assert api.find_resources("AWS::Logs::LogGroup")[group]["Properties"]["RetentionInDays"] == 14


def test_the_job_gets_both_buckets_and_the_alerts_topic(api, base):
    _, function = world_job(api)
    env = function["Properties"]["Environment"]["Variables"]
    assert set(env) == {"IDENTITIES_BUCKET", "CLAIMS_BUCKET", "ALERTS_TOPIC_ARN"}
    # The federation email goes to the owner's alerts topic, never the stop topic.
    assert topic_id(base, "Alerts") in json.dumps(env["ALERTS_TOPIC_ARN"])


def test_the_job_may_list_claims_and_write_only_world_and_federation(api, base):
    found = grants(api, base, "WorldJobFunctionServiceRole")
    publish = {g for g in found if g[0] == "sns:Publish"}
    assert found - publish == {
        ("s3:ListBucket", "Claims"),
        ("s3:ListBucket", "Identities"),
        ("s3:GetObject", "Identities/world/*"),
        ("s3:PutObject", "Identities/world/*"),
        ("s3:DeleteObject", "Identities/world/*"),
        ("s3:GetObject", "Identities/federation/*"),
        ("s3:PutObject", "Identities/federation/*"),
        ("s3:DeleteObject", "Identities/federation/*"),
    }
    assert len(publish) == 1
    assert topic_id(base, "Alerts") in next(iter(publish))[1]


def test_the_job_lists_only_world_and_federation_in_the_identities_bucket(api):
    conditions = []
    for policy in api.find_resources("AWS::IAM::Policy").values():
        if "WorldJobFunctionServiceRole" not in json.dumps(policy["Properties"]["Roles"]):
            continue
        for statement in policy["Properties"]["PolicyDocument"]["Statement"]:
            if statement["Action"] == "s3:ListBucket" and "Identities" in json.dumps(
                statement["Resource"]
            ):
                conditions.append(statement["Condition"])
    assert conditions == [{"StringLike": {"s3:prefix": ["world/*", "federation/*"]}}]


def test_only_the_job_may_touch_verified_federation_requests(api, base):
    for role in ("ApiFunctionServiceRole", "KillSwitchFunctionServiceRole"):
        for _, resource in grants(api, base, role):
            assert "federation/verified" not in resource, role
            assert resource != "Identities/federation/*", role


def test_the_job_s_errors_alarm_only_emails(api, base):
    function_id, _ = world_job(api)
    alarms = [
        a["Properties"]
        for a in api.find_resources("AWS::CloudWatch::Alarm").values()
        if a["Properties"].get("Dimensions")
        == [{"Name": "FunctionName", "Value": {"Ref": function_id}}]
    ]
    assert len(alarms) == 1
    alarm = alarms[0]
    assert alarm["MetricName"] == "Errors"
    assert alarm["Threshold"] == 1
    assert len(alarm["AlarmActions"]) == 1
    assert topic_id(base, "Alerts") in json.dumps(alarm["AlarmActions"])


def test_only_the_job_may_write_or_delete_world(api, base):
    for role in ("ApiFunctionServiceRole", "KillSwitchFunctionServiceRole"):
        found = grants(api, base, role)
        assert ("s3:PutObject", "Identities/world/*") not in found, role
        # The API deletes only in `messages/` (issue 14); nothing but the job deletes in `world/`.
        assert ("s3:DeleteObject", "Identities/world/*") not in found, role
        deletes = {resource for action, resource in found if action == "s3:DeleteObject"}
        assert deletes <= {"Identities/messages/*"}, role


# The kill switch's fourth stop step


def test_the_kill_switch_may_set_the_job_s_concurrency(api):
    job_id, _ = world_job(api)
    api_id, _ = api_function(api)
    allowed = actions(api, "KillSwitchFunctionServiceRole")
    for action in ("lambda:GetFunctionConcurrency", "lambda:PutFunctionConcurrency"):
        text = json.dumps(allowed[action])
        assert job_id in text and api_id in text, action


def test_the_kill_switch_knows_the_job_and_the_restart_list_can_find_it(api):
    job_id, _ = world_job(api)
    _, function = kill_switch(api)
    env = function["Properties"]["Environment"]["Variables"]
    assert env["JOB_FUNCTION_NAME"] == {"Ref": job_id}
    assert api.to_json()["Outputs"]["JobFunctionName"]["Value"] == {"Ref": job_id}
