#!/usr/bin/env python3
"""The CDK app: story 2's base, API and website stacks (see stacks.py), and story 18's
`SfabDeployer` (see deployer_stack.py).

`SfabDeployer` needs no config, so while the config doesn't load or the Lambda builds or website
pages are missing, the app synthesizes only it, with a note naming what's missing
(`.stories/18-deploy-from-codebuild/spec.md` "Synth without the config"). The app can't tell
which stacks the CLI was asked for, so scripts/deploy refuses any other stack then.
"""

from __future__ import annotations

import os
import sys
from collections.abc import Mapping
from pathlib import Path

import aws_cdk as cdk
from aws_cdk import aws_lambda as lambda_

import deployer_stack
import sfab_config
import stacks

# What `cargo lambda build --release --arm64` writes, which scripts/check runs first.
LAMBDA_BUILDS = sfab_config.REPO_ROOT / "lambda" / "target" / "lambda"
LAMBDAS = ("api", "killswitch", "job")
# Read only when SFAB_CONFIG is set, as in tests: a deploy refuses SFAB_CONFIG, so it can't be
# pointed at other builds.
LAMBDA_BUILDS_VAR = "SFAB_LAMBDA_BUILDS"
WEBSITE_BUILD_VAR = "SFAB_WEBSITE_BUILD"


def build_paths(environ: Mapping[str, str]) -> tuple[Path, Path]:
    """The Lambda builds' folder and the website pages' folder."""
    lambda_builds, website = LAMBDA_BUILDS, stacks.WEBSITE_BUILD
    if environ.get(sfab_config.ENV_VAR):
        lambda_builds = Path(environ.get(LAMBDA_BUILDS_VAR) or lambda_builds)
        website = Path(environ.get(WEBSITE_BUILD_VAR) or website)
    return lambda_builds, website


def missing_builds(lambda_builds: Path, website: Path) -> list[str]:
    missing = [
        f"sfab build missing: {lambda_builds / name / 'bootstrap'}. Run scripts/check, "
        "which builds the Lambdas, or scripts/deploy."
        for name in LAMBDAS
        if not (lambda_builds / name / "bootstrap").is_file()
    ]
    if not (website / "index.html").is_file():
        missing.append(
            f"sfab build missing: {website / 'index.html'}. Run scripts/check, "
            "which generates the website pages, or scripts/deploy."
        )
    return missing


def main() -> None:
    try:
        account = sfab_config.deploy_account()
    except sfab_config.ConfigError as e:
        sys.exit(f"sfab config error: {e}")

    app = cdk.App()
    deployer_stack.DeployerStack(app, account=account)

    problems: list[str] = []
    config = None
    try:
        config = sfab_config.load()
    except sfab_config.ConfigError as e:
        problems.append(f"sfab config error: {e}")
    lambda_builds, website = build_paths(os.environ)
    problems += missing_builds(lambda_builds, website)

    if problems:
        print(
            f"sfab: synthesizing only {deployer_stack.STACK_NAME}, because:",
            *problems,
            sep="\n  ",
            file=sys.stderr,
        )
    else:
        try:
            stacks.build(
                app,
                config,
                api_code=lambda_.Code.from_asset(str(lambda_builds / "api")),
                kill_switch_code=lambda_.Code.from_asset(str(lambda_builds / "killswitch")),
                job_code=lambda_.Code.from_asset(str(lambda_builds / "job")),
                website_source=website,
            )
        except stacks.SynthError as e:
            sys.exit(f"sfab synth error: {e}")
    app.synth()


if __name__ == "__main__":
    main()
