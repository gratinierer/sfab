import os
import subprocess
import sys
from pathlib import Path

CDK = Path(__file__).resolve().parent.parent
FIXTURE = CDK / "tests" / "fixtures" / "sfab.config.test.toml"


def synth(config: Path, tmp_path: Path) -> subprocess.CompletedProcess:
    env = os.environ | {"SFAB_CONFIG": str(config), "CDK_OUTDIR": str(tmp_path / "out")}
    return subprocess.run(
        [sys.executable, "app.py"], cwd=CDK, env=env, capture_output=True, text=True
    )


def test_synth_succeeds_with_the_fixture(tmp_path):
    # Needs the arm64 build and the website pages, which scripts/check makes before the tests.
    result = synth(FIXTURE, tmp_path)
    assert result.returncode == 0, result.stderr
    out = tmp_path / "out"
    assert (out / "SfabBase.template.json").is_file()
    assert (out / "SfabApi.template.json").is_file()
    assert (out / "SfabWebsite.template.json").is_file()


def test_synth_refuses_a_missing_config(tmp_path):
    result = synth(tmp_path / "missing.toml", tmp_path)
    assert result.returncode != 0
    assert "sfab config error" in result.stderr
    assert "missing.toml" in result.stderr


def test_synth_refuses_an_incomplete_config(tmp_path):
    incomplete = tmp_path / "incomplete.toml"
    incomplete.write_text(
        FIXTURE.read_text()
        .replace('"sfab-test.invalid"', '"CHANGE_ME"')
        .replace('"Z00000000000000000000"', '"CHANGE_ME"')
    )
    result = synth(incomplete, tmp_path)
    assert result.returncode != 0
    assert "domain_name: still CHANGE_ME" in result.stderr
    assert "hosted_zone_id: still CHANGE_ME" in result.stderr
