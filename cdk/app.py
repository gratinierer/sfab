#!/usr/bin/env python3
"""The CDK app. Stacks arrive with story 2's later tickets (issue 01 onward)."""

import sys

import aws_cdk as cdk

import sfab_config

REGION = "us-east-1"


def main() -> None:
    try:
        config = sfab_config.load()
    except sfab_config.ConfigError as e:
        sys.exit(f"sfab config error: {e}")

    app = cdk.App()
    _env = cdk.Environment(account=config.account_id, region=REGION)
    app.synth()


if __name__ == "__main__":
    main()
