"""What the CDK assertion tests share: the committed fixture config, never a real
sfab.config.toml, and the stacks built once from it with stand-in Lambda assets and website
folder, so they need no build or generate step."""

from pathlib import Path

import aws_cdk as cdk
import pytest
from aws_cdk import aws_lambda as lambda_

import sfab_config
import stacks

FIXTURE = Path(__file__).resolve().parent / "fixtures" / "sfab.config.test.toml"


def stand_in(tmp_path_factory, name: str) -> lambda_.Code:
    asset = tmp_path_factory.mktemp(name)
    (asset / "bootstrap").write_text(f"stand-in {name}")
    return lambda_.Code.from_asset(str(asset))


def stand_in_site(tmp_path_factory) -> Path:
    site = tmp_path_factory.mktemp("website")
    (site / "index.html").write_text("stand-in")
    return site


def build_with_stand_ins(app: cdk.App, config, tmp_path_factory) -> stacks.Stacks:
    return stacks.build(
        app,
        config,
        api_code=stand_in(tmp_path_factory, "api"),
        kill_switch_code=stand_in(tmp_path_factory, "killswitch"),
        job_code=stand_in(tmp_path_factory, "job"),
        website_source=stand_in_site(tmp_path_factory),
    )


@pytest.fixture(scope="session")
def config():
    return sfab_config.load(FIXTURE)


@pytest.fixture(scope="session")
def built(config, tmp_path_factory):
    return build_with_stand_ins(cdk.App(), config, tmp_path_factory)
