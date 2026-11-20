"""`cdk/app.py`: which stacks it synthesizes, `.stories/18-deploy-from-codebuild/spec.md` "Synth
without the config". The synths run app.py in a subprocess with `CDK_DEFAULT_ACCOUNT` set and
the config from a fixture through `SFAB_CONFIG`, so none reaches Secrets Manager or STS."""

import json
import os
import subprocess
import sys
from pathlib import Path

import app
import sfab_config
import world_config

CDK = Path(__file__).resolve().parent.parent
FIXTURE = CDK / "tests" / "fixtures" / "sfab.config.test.toml"
FIXTURE_ACCOUNT = sfab_config.load(environ={"SFAB_CONFIG": str(FIXTURE)}).account_id
# The account of the caller's credentials, which only SfabDeployer follows.
CALLER = "210987654321"
ALL_STACKS = {"SfabBase", "SfabApi", "SfabWebsite", "SfabDeployer"}


def synth(config: Path, tmp_path: Path, **extra: str) -> subprocess.CompletedProcess:
    env = os.environ | {
        "SFAB_CONFIG": str(config),
        "CDK_OUTDIR": str(tmp_path / "out"),
        "CDK_DEFAULT_ACCOUNT": CALLER,
        **extra,
    }
    return subprocess.run(
        [sys.executable, "app.py"], cwd=CDK, env=env, capture_output=True, text=True
    )


def stacks(tmp_path: Path) -> dict[str, str]:
    """Each synthesized stack's environment, by name."""
    manifest = json.loads((tmp_path / "out" / "manifest.json").read_text())
    return {
        name: artifact["environment"]
        for name, artifact in manifest["artifacts"].items()
        if artifact["type"] == "aws:cloudformation:stack"
    }


def assert_only_the_deployer(result, tmp_path: Path, *named: str) -> None:
    assert result.returncode == 0, result.stderr
    assert stacks(tmp_path) == {"SfabDeployer": f"aws://{CALLER}/us-east-1"}
    assert "only SfabDeployer" in result.stderr
    for text in named:
        assert text in result.stderr


def test_synth_builds_all_four_stacks_with_the_fixture(tmp_path):
    # Needs the arm64 build and the website pages, which scripts/build makes before the tests.
    result = synth(FIXTURE, tmp_path)

    assert result.returncode == 0, result.stderr
    found = stacks(tmp_path)
    assert set(found) == ALL_STACKS
    assert found["SfabDeployer"] == f"aws://{CALLER}/us-east-1"
    assert found["SfabBase"] == f"aws://{FIXTURE_ACCOUNT}/us-east-1"
    assert "only SfabDeployer" not in result.stderr


def test_synth_with_no_config_builds_only_the_deployer(tmp_path):
    result = synth(tmp_path / "missing.toml", tmp_path)

    assert_only_the_deployer(result, tmp_path, "missing.toml", "config file not found")


def test_synth_with_an_incomplete_config_builds_only_the_deployer(tmp_path):
    incomplete = tmp_path / "incomplete.toml"
    incomplete.write_text(
        FIXTURE.read_text()
        .replace('"sfab-test.invalid"', '"CHANGE_ME"')
        .replace('"Z00000000000000000000"', '"CHANGE_ME"')
    )

    result = synth(incomplete, tmp_path)

    assert_only_the_deployer(
        result, tmp_path, "domain_name: still CHANGE_ME", "hosted_zone_id: still CHANGE_ME"
    )


def test_synth_with_missing_builds_builds_only_the_deployer(tmp_path):
    empty = tmp_path / "empty"
    empty.mkdir()

    result = synth(
        FIXTURE, tmp_path, SFAB_LAMBDA_BUILDS=str(empty), SFAB_WEBSITE_BUILD=str(empty)
    )

    assert_only_the_deployer(
        result, tmp_path, str(empty / "api" / "bootstrap"), str(empty / "index.html")
    )


def test_the_build_paths_are_today_s_unless_sfab_config_is_set():
    overrides = {"SFAB_LAMBDA_BUILDS": "/elsewhere/lambda", "SFAB_WEBSITE_BUILD": "/elsewhere/site"}

    assert app.build_paths(overrides) == (
        sfab_config.REPO_ROOT / "lambda" / "target" / "lambda",
        sfab_config.REPO_ROOT / "build" / "website",
    )
    assert app.build_paths({"SFAB_CONFIG": str(FIXTURE), **overrides}) == (
        Path("/elsewhere/lambda"),
        Path("/elsewhere/site"),
    )
    assert app.build_paths({"SFAB_CONFIG": str(FIXTURE)}) == app.build_paths({})


# Story 4's world files: `.stories/4-life-in-the-world/spec.md` "Foundations".


def world_folder(tmp_path: Path, founders: str | None) -> Path:
    folder = tmp_path / "world"
    folder.mkdir()
    for name in ("quarters", "shelves"):
        (folder / f"{name}.yaml").write_text((world_config.WORLD / f"{name}.yaml").read_text())
    if founders is not None:
        (folder / "founders.yaml").write_text(founders)
    return folder


def test_synth_fails_with_fewer_than_three_founders(tmp_path):
    two = "founders:\n  - foundera2222\n  - founderb2222\n"

    result = synth(FIXTURE, tmp_path, SFAB_WORLD=str(world_folder(tmp_path, two)))

    assert result.returncode != 0
    assert "at least 3 founders" in result.stderr


def test_synth_fails_with_a_bad_quarter_name(tmp_path):
    folder = world_folder(tmp_path, world_config.FOUNDERS_FIXTURE.read_text())
    (folder / "quarters.yaml").write_text("quarters:\n  - name: Old_Town\n    lanes: [x]\n")

    result = synth(FIXTURE, tmp_path, SFAB_WORLD=str(folder))

    assert result.returncode != 0
    assert "Old_Town" in result.stderr


def test_synth_without_a_founders_file_builds_only_the_deployer(tmp_path):
    result = synth(FIXTURE, tmp_path, SFAB_WORLD=str(world_folder(tmp_path, None)))

    assert_only_the_deployer(result, tmp_path, "founders.yaml")


def test_the_world_files_are_the_real_ones_unless_sfab_config_is_set(tmp_path):
    override = {"SFAB_WORLD": str(tmp_path)}

    assert app.world_files(override) == world_config.REAL_FILES
    assert app.world_files({"SFAB_CONFIG": str(FIXTURE)}) == world_config.FIXTURE_FILES
    assert app.world_files({"SFAB_CONFIG": str(FIXTURE), **override}) == (
        world_config.Files.under(tmp_path)
    )
