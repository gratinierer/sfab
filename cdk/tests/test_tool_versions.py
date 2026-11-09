"""CI and the deploy build pin their tools in one place, `tool-versions.env`:
`.stories/18-deploy-from-codebuild/spec.md` "A build", "Pinned tools"."""

import re

import pytest
import yaml

import sfab_config

ROOT = sfab_config.REPO_ROOT
TOOL_VERSIONS = ROOT / "tool-versions.env"
CI = ROOT / ".github" / "workflows" / "ci.yml"
BUILDSPEC = ROOT / "buildspec.yml"


def pins() -> dict[str, str]:
    found = {}
    for line in TOOL_VERSIONS.read_text().splitlines():
        line = line.strip()
        if line and not line.startswith("#"):
            # Plain KEY=value, since GitHub's $GITHUB_ENV takes quotes literally.
            match = re.fullmatch(r"([A-Z_]+)=([0-9][0-9A-Za-z.+-]*)", line)
            assert match, f"not KEY=version: {line}"
            found[match[1]] = match[2]
    return found


def test_tool_versions_pins_every_tool_ci_and_the_build_install():
    assert set(pins()) == {
        "UV_VERSION",
        "CARGO_LAMBDA_VERSION",
        "ZIGLANG_VERSION",
        "CARGO_AUDIT_VERSION",
        "CDK_CLI_VERSION",
    }


@pytest.mark.parametrize("path", [CI, BUILDSPEC], ids=["ci.yml", "buildspec.yml"])
def test_no_version_is_pinned_inline(path):
    text = path.read_text()
    for name, version in pins().items():
        assert not re.search(rf"{name}\s*[:=]", text), f"{path.name} sets {name}"
        assert version not in text, f"{path.name} holds {name}'s {version}"


def test_ci_loads_the_pins_right_after_checkout_before_the_cache_reads_them():
    ci = yaml.safe_load(CI.read_text())
    steps = ci["jobs"]["check"]["steps"]

    assert steps[0]["uses"].startswith("actions/checkout@")
    assert "tool-versions.env" in steps[1]["run"]
    assert "$GITHUB_ENV" in steps[1]["run"]
    assert steps[2]["uses"].startswith("actions/cache@")
    assert "env.CARGO_AUDIT_VERSION" in steps[2]["with"]["key"]
    for name in ("UV_VERSION", "CARGO_LAMBDA_VERSION", "ZIGLANG_VERSION", "CARGO_AUDIT_VERSION"):
        assert name not in ci.get("env", {})
        assert name in CI.read_text()


def test_the_buildspec_loads_the_pins_and_uses_every_one():
    text = BUILDSPEC.read_text()

    assert "tool-versions.env" in text
    for name in pins():
        assert "${" + name + "}" in text or "$" + name in text, name
