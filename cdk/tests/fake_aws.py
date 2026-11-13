"""`sfab_config.Aws` with stubbed Secrets Manager and STS, so the config tests never reach AWS.

The clients are real boto3 clients behind botocore's Stubber: each answers the calls the test
set up, checks their parameters, and fails the test on any call it didn't expect.
"""

from __future__ import annotations

import json

import boto3
from botocore.stub import Stubber


class StubbedAws:
    """`secret` is the secret's value (a dict is sent as JSON, a string as it is), or
    `secret_error` the AWS error code and message GetSecretValue raises. `account` is what
    GetCallerIdentity answers, and `region` the region the deploy would run in."""

    def __init__(
        self,
        secret: str | dict | None = None,
        *,
        secret_error: tuple[str, str] | None = None,
        account: str | None = None,
        region: str | None = "us-east-1",
    ):
        self.secret = json.dumps(secret) if isinstance(secret, dict) else secret
        self.secret_error = secret_error
        self.account = account
        self._region = region
        self.calls: list[str] = []

    def region(self, environ) -> str | None:
        return self._region

    def client(self, service: str, region: str | None = None):
        self.calls.append(f"{service}@{region}")
        client = boto3.client(
            service,
            region_name=region or "us-east-1",
            aws_access_key_id="stub",
            aws_secret_access_key="stub",
        )
        stubber = Stubber(client)
        if service == "secretsmanager":
            if self.secret_error is not None:
                code, message = self.secret_error
                stubber.add_client_error(
                    "get_secret_value",
                    service_error_code=code,
                    service_message=message,
                    expected_params={"SecretId": "sfab/config"},
                )
            elif self.secret is not None:
                stubber.add_response(
                    "get_secret_value",
                    {"Name": "sfab/config", "SecretString": self.secret},
                    {"SecretId": "sfab/config"},
                )
        elif service == "sts" and self.account is not None:
            stubber.add_response(
                "get_caller_identity",
                {
                    "Account": self.account,
                    "Arn": f"arn:aws:iam::{self.account}:user/owner",
                    "UserId": "AIDAEXAMPLE",
                },
                {},
            )
        stubber.activate()
        return client


class NoAws:
    """Fails the test on any AWS call: for paths that must never reach AWS."""

    def region(self, environ) -> str | None:
        raise AssertionError("no AWS region lookup expected")

    def client(self, service: str, region: str | None = None):
        raise AssertionError(f"no AWS call expected, got {service}")
