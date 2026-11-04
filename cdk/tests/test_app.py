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
    result = synth(FIXTURE, tmp_path)
    assert result.returncode == 0, result.stderr


def test_synth_refuses_a_missing_config(tmp_path):
    result = synth(tmp_path / "missing.toml", tmp_path)
    assert result.returncode != 0
    assert "sfab config error" in result.stderr
    assert "missing.toml" in result.stderr


def test_synth_refuses_the_copied_example(tmp_path):
    result = synth(CDK.parent / "sfab.config.example.toml", tmp_path)
    assert result.returncode != 0
    assert "still CHANGE_ME" in result.stderr
