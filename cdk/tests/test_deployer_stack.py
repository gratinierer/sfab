"""CDK assertion tests for `SfabDeployer`: `.stories/18-deploy-from-codebuild/spec.md` "The
deployer stack". It needs no config, so these build it alone, in a made-up account."""

import aws_cdk as cdk
import pytest
import yaml
from aws_cdk.assertions import Template

import deployer_stack
import sfab_config

ACCOUNT = "210987654321"


@pytest.fixture(scope="module")
def deployer():
    app = cdk.App()
    stack = deployer_stack.DeployerStack(app, account=ACCOUNT)
    return Template.from_stack(stack)


def only(template: Template, type_: str) -> tuple[str, dict]:
    found = template.find_resources(type_)
    assert len(found) == 1, found
    return next(iter(found.items()))


def test_the_secret_is_sfab_config_with_no_value_the_template_controls_and_is_retained(deployer):
    _, secret = only(deployer, "AWS::SecretsManager::Secret")

    # Neither SecretString nor GenerateSecretString: CloudFormation creates an empty secret,
    # which the owner fills, and an update never overwrites the owner's value.
    assert secret["Properties"]["Name"] == "sfab/config"
    assert "SecretString" not in secret["Properties"]
    assert "GenerateSecretString" not in secret["Properties"]
    assert secret["DeletionPolicy"] == "Retain"
    assert secret["UpdateReplacePolicy"] == "Retain"


def test_the_connection_is_to_github(deployer):
    _, connection = only(deployer, "AWS::CodeConnections::Connection")

    assert connection["Properties"]["ProviderType"] == "GitHub"


def test_the_project_builds_production_of_this_repo_through_the_connection(deployer):
    connection_id, _ = only(deployer, "AWS::CodeConnections::Connection")
    _, project = only(deployer, "AWS::CodeBuild::Project")
    props = project["Properties"]

    assert props["Name"] == "sfab-deploy"
    assert props["SourceVersion"] == "production"
    assert props["Source"] == {
        "Type": "GITHUB",
        "Location": "https://github.com/gratinierer/sfab.git",
        "Auth": {
            "Type": "CODECONNECTIONS",
            "Resource": {"Fn::GetAtt": [connection_id, "ConnectionArn"]},
        },
        "BuildSpec": "buildspec.yml",
    }
    assert props["Artifacts"] == {"Type": "NO_ARTIFACTS"}
    # No Triggers: only a start-build by the owner runs it.
    assert "Triggers" not in props


def test_the_project_runs_the_arm_image_on_medium_and_deploys_all_by_default(deployer):
    _, project = only(deployer, "AWS::CodeBuild::Project")

    assert project["Properties"]["Environment"] == {
        "Type": "ARM_CONTAINER",
        "ComputeType": "BUILD_GENERAL1_MEDIUM",
        "Image": "aws/codebuild/amazonlinux-aarch64-standard:3.0",
        "ImagePullCredentialsType": "CODEBUILD",
        "PrivilegedMode": False,
        "EnvironmentVariables": [
            {"Name": "DEPLOY_STACKS", "Type": "PLAINTEXT", "Value": "--all"}
        ],
    }


def test_the_buildspec_in_the_repo_runs_scripts_deploy_with_deploy_stacks():
    buildspec = yaml.safe_load((sfab_config.REPO_ROOT / "buildspec.yml").read_text())

    commands = [c for phase in buildspec["phases"].values() for c in phase["commands"]]
    assert any("scripts/deploy $DEPLOY_STACKS" in c for c in commands), commands


def test_the_build_role_is_codebuild_s_in_this_account_only(deployer):
    roles = deployer.find_resources("AWS::IAM::Role")
    project_id, project = only(deployer, "AWS::CodeBuild::Project")
    assert len(roles) == 1
    role_id, role = next(iter(roles.items()))

    assert project["Properties"]["ServiceRole"] == {"Fn::GetAtt": [role_id, "Arn"]}
    assert role["Properties"]["AssumeRolePolicyDocument"]["Statement"] == [
        {
            "Action": "sts:AssumeRole",
            "Effect": "Allow",
            "Principal": {"Service": "codebuild.amazonaws.com"},
            "Condition": {"StringEquals": {"aws:SourceAccount": ACCOUNT}},
        }
    ]
    assert "ManagedPolicyArns" not in role["Properties"]
    assert "Policies" not in role["Properties"]


def bootstrap_role(name: str) -> dict:
    return {
        "Fn::Join": [
            "",
            [
                "arn:",
                {"Ref": "AWS::Partition"},
                f":iam::{ACCOUNT}:role/cdk-hnb659fds-{name}-role-{ACCOUNT}-us-east-1",
            ],
        ]
    }


def test_the_build_role_has_exactly_its_grants_and_nothing_wider(deployer):
    secret_id, _ = only(deployer, "AWS::SecretsManager::Secret")
    connection_id, _ = only(deployer, "AWS::CodeConnections::Connection")
    bucket_id, _ = only(deployer, "AWS::S3::Bucket")
    logs_id, _ = only(deployer, "AWS::Logs::LogGroup")
    policies = deployer.find_resources("AWS::IAM::Policy")
    assert len(policies) == 1

    statements = next(iter(policies.values()))["Properties"]["PolicyDocument"]["Statement"]

    assert statements == [
        {
            "Action": "secretsmanager:GetSecretValue",
            "Effect": "Allow",
            "Resource": {"Ref": secret_id},
        },
        {
            "Action": "sts:AssumeRole",
            "Effect": "Allow",
            "Resource": [
                bootstrap_role(name)
                for name in ("deploy", "file-publishing", "image-publishing", "lookup")
            ],
        },
        {
            "Action": ["codeconnections:GetConnection", "codeconnections:GetConnectionToken"],
            "Effect": "Allow",
            "Resource": {"Fn::GetAtt": [connection_id, "ConnectionArn"]},
        },
        {
            "Action": ["logs:CreateLogGroup", "logs:CreateLogStream", "logs:PutLogEvents"],
            "Effect": "Allow",
            "Resource": {"Fn::GetAtt": [logs_id, "Arn"]},
        },
        {
            "Action": ["s3:GetBucketAcl", "s3:GetBucketLocation", "s3:ListBucket"],
            "Effect": "Allow",
            "Resource": {"Fn::GetAtt": [bucket_id, "Arn"]},
        },
        {
            "Action": ["s3:GetObject", "s3:PutObject"],
            "Effect": "Allow",
            "Resource": {"Fn::Join": ["", [{"Fn::GetAtt": [bucket_id, "Arn"]}, "/*"]]},
        },
    ]


def test_the_build_logs_to_its_own_group_and_caches_in_the_bucket(deployer):
    bucket_id, _ = only(deployer, "AWS::S3::Bucket")
    logs_id, log_group = only(deployer, "AWS::Logs::LogGroup")
    _, project = only(deployer, "AWS::CodeBuild::Project")
    props = project["Properties"]

    assert log_group["Properties"]["LogGroupName"] == "/aws/codebuild/sfab-deploy"
    assert props["LogsConfig"]["CloudWatchLogs"] == {
        "Status": "ENABLED",
        "GroupName": {"Ref": logs_id},
    }
    assert props["Cache"] == {
        "Type": "S3",
        "Location": {"Fn::Join": ["", [{"Ref": bucket_id}, "/sfab-deploy"]]},
    }


def test_the_cache_bucket_is_private_and_expires_its_objects(deployer):
    _, bucket = only(deployer, "AWS::S3::Bucket")
    props = bucket["Properties"]

    assert props["LifecycleConfiguration"] == {
        "Rules": [{"ExpirationInDays": 30, "Status": "Enabled"}]
    }
    assert props["PublicAccessBlockConfiguration"] == {
        "BlockPublicAcls": True,
        "BlockPublicPolicy": True,
        "IgnorePublicAcls": True,
        "RestrictPublicBuckets": True,
    }


def test_the_secret_lives_in_the_region_sfab_deploys_to():
    # sfab_config can't import the CDK code, so it names the secret's region itself.
    assert sfab_config.SECRET_REGION == deployer_stack.REGION
