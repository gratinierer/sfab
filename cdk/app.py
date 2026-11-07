#!/usr/bin/env python3
"""The CDK app: story 2's base, API and website stacks (see stacks.py)."""

import sys

import aws_cdk as cdk
from aws_cdk import aws_lambda as lambda_

import sfab_config
import stacks

# What `cargo lambda build --release --arm64` writes, which scripts/check runs first.
LAMBDA_BUILDS = sfab_config.REPO_ROOT / "lambda" / "target" / "lambda"
API_LAMBDA_BUILD = LAMBDA_BUILDS / "api"
KILL_SWITCH_BUILD = LAMBDA_BUILDS / "killswitch"
JOB_BUILD = LAMBDA_BUILDS / "job"


def main() -> None:
    # Loading checks the config, so synth fails on a missing or unfinished one.
    try:
        config = sfab_config.load()
    except sfab_config.ConfigError as e:
        sys.exit(f"sfab config error: {e}")

    for build in (API_LAMBDA_BUILD, KILL_SWITCH_BUILD, JOB_BUILD):
        if not (build / "bootstrap").is_file():
            sys.exit(
                f"sfab build missing: {build / 'bootstrap'}. Run scripts/check, "
                "which builds the Lambdas, or scripts/deploy."
            )

    if not (stacks.WEBSITE_BUILD / "index.html").is_file():
        sys.exit(
            f"sfab build missing: {stacks.WEBSITE_BUILD}. Run scripts/check, "
            "which generates the website pages, or scripts/deploy."
        )

    app = cdk.App()
    try:
        stacks.build(
            app,
            config,
            api_code=lambda_.Code.from_asset(str(API_LAMBDA_BUILD)),
            kill_switch_code=lambda_.Code.from_asset(str(KILL_SWITCH_BUILD)),
            job_code=lambda_.Code.from_asset(str(JOB_BUILD)),
        )
    except stacks.SynthError as e:
        sys.exit(f"sfab synth error: {e}")
    app.synth()


if __name__ == "__main__":
    main()
