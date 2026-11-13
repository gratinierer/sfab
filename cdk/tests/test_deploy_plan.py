"""scripts/deploy's decisions, made in-process: `.stories/18-deploy-from-codebuild/spec.md`
"A build". Secrets Manager and STS are stubbed, so nothing here reaches AWS."""

import pytest
from fake_aws import NoAws, StubbedAws
from test_sfab_config import ACCOUNT, FIXTURE, secret_data

import deploy_plan


def full_secret(region="us-east-1"):
    return StubbedAws(secret_data(), account=ACCOUNT, region=region)


def test_a_set_sfab_config_is_refused_before_anything_else():
    found = deploy_plan.plan(["--bogus"], environ={"SFAB_CONFIG": str(FIXTURE)}, aws=NoAws())

    assert found.stacks == []
    assert len(found.errors) == 1
    assert "SFAB_CONFIG" in found.errors[0]


@pytest.mark.parametrize(
    "argv",
    [["--all"], ["SfabApi"], ["SfabBase", "SfabWebsite"]],
)
def test_all_one_or_several_stacks_pass_with_the_secret_required(argv):
    found = deploy_plan.plan(argv, environ={}, aws=full_secret())

    assert found.errors == []
    assert found.stacks == argv
    assert found.require_secret is True


def test_the_plan_knows_every_stack_the_app_builds(built):
    names = {built.base.stack_name, built.api.stack_name, built.website.stack_name}
    assert set(deploy_plan.STACKS) == names


@pytest.mark.parametrize(
    "argv, named",
    [
        ([], "--all"),
        (["--require-approval", "any-change"], "--require-approval"),
        (["SfabApi", "--force"], "--force"),
        (["SfabNope"], "SfabNope"),
        (["--all", "SfabApi"], "--all"),
    ],
)
def test_anything_but_all_or_stack_names_is_refused_before_the_region_or_config(argv, named):
    found = deploy_plan.plan(argv, environ={}, aws=NoAws())

    assert found.stacks == []
    assert any(named in error for error in found.errors), found.errors


@pytest.mark.parametrize("region", ["eu-central-1", None])
def test_a_region_other_than_us_east_1_is_refused_before_the_config(region):
    aws = StubbedAws(region=region)

    found = deploy_plan.plan(["--all"], environ={}, aws=aws)

    assert found.stacks == []
    assert "us-east-1" in found.errors[0]
    assert str(region) in found.errors[0] if region else "no region" in found.errors[0]
    assert aws.calls == []


def test_a_config_that_doesnt_load_is_refused_with_its_problems():
    data = secret_data()
    del data["hosted_zone_id"]

    found = deploy_plan.plan(["--all"], environ={}, aws=StubbedAws(data, account=ACCOUNT))

    assert found.stacks == []
    assert any("hosted_zone_id: missing" in error for error in found.errors)


def test_main_prints_the_mode_then_the_stacks(capsys):
    code = deploy_plan.main(["SfabBase", "SfabApi"], environ={}, aws=full_secret())

    assert code == 0
    assert capsys.readouterr().out.splitlines() == ["require-secret", "SfabBase", "SfabApi"]


def test_main_fails_with_the_errors_on_stderr(capsys):
    code = deploy_plan.main(["SfabNope"], environ={}, aws=NoAws())

    captured = capsys.readouterr()
    assert code == 1
    assert captured.out == ""
    assert "SfabNope" in captured.err
