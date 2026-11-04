#!/usr/bin/env python3
"""The CDK app. Stacks arrive with story 2's later tickets (issue 01 onward)."""

import sys

import aws_cdk as cdk

import sfab_config


def main() -> None:
    # Loading checks the config, so synth fails on a missing or unfinished one. The stacks from issue 01 on use it.
    try:
        sfab_config.load()
    except sfab_config.ConfigError as e:
        sys.exit(f"sfab config error: {e}")

    cdk.App().synth()


if __name__ == "__main__":
    main()
