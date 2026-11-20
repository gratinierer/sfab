"""`SfabDeployer`: the CodeBuild project that deploys the app, and the secret it reads.

See `.stories/18-deploy-from-codebuild/spec.md` "The deployer stack" and "A build". It needs no
config, so it synthesizes and deploys before `sfab/config` holds a value.
"""

from __future__ import annotations

import aws_cdk as cdk
from aws_cdk import aws_codebuild as codebuild
from aws_cdk import aws_codeconnections as codeconnections
from aws_cdk import aws_iam as iam
from aws_cdk import aws_logs as logs
from aws_cdk import aws_s3 as s3
from aws_cdk import aws_secretsmanager as secretsmanager
from constructs import Construct

import sfab_config

STACK_NAME = "SfabDeployer"
# The one region sfab deploys to; stacks.py and deploy_plan.py read it from here.
REGION = "us-east-1"
# SfabBase's failure alert names the project by this name, since it can't depend on this stack.
PROJECT_NAME = "sfab-deploy"
REPOSITORY = "gratinierer/sfab"
BRANCH = "production"
# In the repo root; it installs the tools tool-versions.env pins and runs scripts/deploy.
BUILDSPEC = "buildspec.yml"
# The Arm image (docs/aws-facts.md, "CodeBuild").
IMAGE = "aws/codebuild/amazonlinux-aarch64-standard:3.0"
DEPLOY_STACKS_DEFAULT = "--all"
# The roles `cdk bootstrap` creates that the CDK CLI assumes, named
# `cdk-hnb659fds-<name>-role-<account>-<region>`; the CloudFormation execution role is assumed
# by CloudFormation, not by the build (docs/aws-facts.md, "CodeBuild and CodeConnections").
BOOTSTRAP_ROLES = ("deploy", "file-publishing", "image-publishing", "lookup")
CACHE_DAYS = 30
LOG_DAYS = logs.RetentionDays.THREE_MONTHS
BUILD_TIMEOUT_MINUTES = 60


class DeployerStack(cdk.Stack):
    def __init__(self, scope: Construct, *, account: str) -> None:
        super().__init__(scope, STACK_NAME, env=cdk.Environment(account=account, region=REGION))
        # A raw CfnSecret, because CDK's Secret with no value emits `GenerateSecretString: {}`
        # and starts with a random password (docs/aws-facts.md). With neither SecretString nor
        # GenerateSecretString, CloudFormation creates an empty secret, the owner fills it, and
        # no update can overwrite the owner's value.
        self.secret = secretsmanager.CfnSecret(
            self,
            "Config",
            name=sfab_config.SECRET_ID,
            description="sfab's private config, one JSON entry per key; filled by the owner.",
        )
        self.secret.apply_removal_policy(cdk.RemovalPolicy.RETAIN)

        # Created PENDING; the owner approves it once in the console.
        self.connection = codeconnections.CfnConnection(
            self, "GitHub", connection_name="sfab-github", provider_type="GitHub"
        )

        # cargo and uv downloads only, so it can be rebuilt any time: expired and deletable.
        self.cache = s3.Bucket(
            self,
            "Cache",
            block_public_access=s3.BlockPublicAccess.BLOCK_ALL,
            encryption=s3.BucketEncryption.S3_MANAGED,
            enforce_ssl=True,
            lifecycle_rules=[s3.LifecycleRule(expiration=cdk.Duration.days(CACHE_DAYS))],
            removal_policy=cdk.RemovalPolicy.DESTROY,
        )

        self.log_group = logs.LogGroup(
            self,
            "Logs",
            log_group_name=f"/aws/codebuild/{PROJECT_NAME}",
            retention=LOG_DAYS,
            removal_policy=cdk.RemovalPolicy.DESTROY,
        )

        self.role = iam.Role(
            self,
            "Role",
            assumed_by=iam.ServicePrincipal(
                "codebuild.amazonaws.com",
                conditions={"StringEquals": {"aws:SourceAccount": self.account}},
            ),
        )
        self._grant()

        self.project = codebuild.CfnProject(
            self,
            "Project",
            name=PROJECT_NAME,
            service_role=self.role.role_arn,
            source=codebuild.CfnProject.SourceProperty(
                type="GITHUB",
                location=f"https://github.com/{REPOSITORY}.git",
                auth=codebuild.CfnProject.SourceAuthProperty(
                    type="CODECONNECTIONS", resource=self.connection.attr_connection_arn
                ),
                build_spec=BUILDSPEC,
            ),
            source_version=BRANCH,
            artifacts=codebuild.CfnProject.ArtifactsProperty(type="NO_ARTIFACTS"),
            environment=codebuild.CfnProject.EnvironmentProperty(
                type="ARM_CONTAINER",
                # ARM/Medium, 4 vCPU and 8 GB: cargo's default jobs fit, where ARM/Small's 4 GB
                # needed one job and twice the time (docs/aws-facts.md, "CodeBuild").
                compute_type="BUILD_GENERAL1_MEDIUM",
                image=IMAGE,
                image_pull_credentials_type="CODEBUILD",
                privileged_mode=False,
                environment_variables=[
                    codebuild.CfnProject.EnvironmentVariableProperty(
                        name="DEPLOY_STACKS", type="PLAINTEXT", value=DEPLOY_STACKS_DEFAULT
                    )
                ],
            ),
            cache=codebuild.CfnProject.ProjectCacheProperty(
                type="S3", location=f"{self.cache.bucket_name}/{PROJECT_NAME}"
            ),
            logs_config=codebuild.CfnProject.LogsConfigProperty(
                cloud_watch_logs=codebuild.CfnProject.CloudWatchLogsConfigProperty(
                    status="ENABLED", group_name=self.log_group.log_group_name
                ),
                s3_logs=codebuild.CfnProject.S3LogsConfigProperty(status="DISABLED"),
            ),
            timeout_in_minutes=BUILD_TIMEOUT_MINUTES,
        )
        # The role's policy must exist before CodeBuild checks it can use the connection.
        self.project.node.add_dependency(self.role)

    def _grant(self) -> None:
        """The build role's only permissions. The bootstrap's CloudFormation execution role,
        which CloudFormation assumes, does the deploying."""
        bootstrap_roles = [
            f"arn:{self.partition}:iam::{self.account}:role/"
            f"cdk-{cdk.DefaultStackSynthesizer.DEFAULT_QUALIFIER}-{name}-role-"
            f"{self.account}-{self.region}"
            for name in BOOTSTRAP_ROLES
        ]
        statements = [
            iam.PolicyStatement(
                actions=["secretsmanager:GetSecretValue"], resources=[self.secret.ref]
            ),
            iam.PolicyStatement(actions=["sts:AssumeRole"], resources=bootstrap_roles),
            iam.PolicyStatement(
                actions=["codeconnections:GetConnection", "codeconnections:GetConnectionToken"],
                resources=[self.connection.attr_connection_arn],
            ),
            iam.PolicyStatement(
                actions=["logs:CreateLogGroup", "logs:CreateLogStream", "logs:PutLogEvents"],
                resources=[self.log_group.log_group_arn],
            ),
            # The S3 cache's permissions (docs/aws-facts.md, "CodeBuild and CodeConnections"); restoring the cache
            # needs s3:ListBucket, which AWS's example leaves out.
            iam.PolicyStatement(
                actions=["s3:GetBucketAcl", "s3:GetBucketLocation", "s3:ListBucket"],
                resources=[self.cache.bucket_arn],
            ),
            iam.PolicyStatement(
                actions=["s3:GetObject", "s3:PutObject"],
                resources=[self.cache.arn_for_objects("*")],
            ),
        ]
        for statement in statements:
            self.role.add_to_policy(statement)
