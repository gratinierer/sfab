#!/usr/bin/env python3
"""The CDK app: story 2's base and API stacks (see stacks.py)."""

import sys

import aws_cdk as cdk
from aws_cdk import aws_lambda as lambda_

import sfab_config
import stacks

# What `cargo lambda build --release --arm64` writes, which scripts/check runs first.
API_LAMBDA_BUILD = sfab_config.REPO_ROOT / "lambda" / "target" / "lambda" / "api"


def main() -> None:
    # Loading checks the config, so synth fails on a missing or unfinished one.
    try:
        config = sfab_config.load()
    except sfab_config.ConfigError as e:
        sys.exit(f"sfab config error: {e}")

    if not (API_LAMBDA_BUILD / "bootstrap").is_file():
        sys.exit(
            f"sfab build missing: {API_LAMBDA_BUILD / 'bootstrap'}. Run scripts/check, "
            "which builds the Lambdas, or scripts/deploy."
        )

    app = cdk.App()
    try:
        stacks.build(app, config, api_code=lambda_.Code.from_asset(str(API_LAMBDA_BUILD)))
    except stacks.SynthError as e:
        sys.exit(f"sfab synth error: {e}")
    app.synth()


if __name__ == "__main__":
    main()
