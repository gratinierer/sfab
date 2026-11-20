"""What scripts/build, scripts/check and scripts/deploy run, read from their source:
`.stories/19-deploy-build-cache/spec.md` Goal 2. No test runs them: they stay thin wrappers, and
the decisions stay in `cdk/deploy_plan.py` (`.stories/done/18-deploy-from-codebuild/review.md`
R31, R34). Only command lines count, since the header comments name scripts/check and the audit.
"""

import re
import tomllib

import app
import sfab_config

SCRIPTS = sfab_config.REPO_ROOT / "scripts"


def command_lines(name: str) -> list[str]:
    lines = [line.strip() for line in (SCRIPTS / name).read_text().splitlines()]
    return [line for line in lines if line and not line.startswith("#")]


def indexes(lines: list[str], text: str) -> list[int]:
    return [i for i, line in enumerate(lines) if text in line]


def case_arms(lines: list[str]) -> dict[str, str]:
    """`pattern) command ;;` lines, as {pattern: command}."""
    return {m[1]: m[2] for line in lines if (m := re.fullmatch(r"([\w-]+)\)\s*(.*?)\s*;;", line))}


def test_build_runs_one_pytest_on_test_openapi_before_the_lambda_build_and_no_audit_or_cargo_test():
    lines = command_lines("build")

    pytest_lines = indexes(lines, "pytest")
    assert len(pytest_lines) == 1, lines
    assert "tests/test_openapi.py" in lines[pytest_lines[0]]
    lambda_build = indexes(lines, "cargo lambda build")
    assert len(lambda_build) == 1, lines
    assert pytest_lines[0] < lambda_build[0]
    assert indexes(lines, "generate_website.py")[0] < pytest_lines[0]
    assert indexes(lines, "cargo test") == []
    assert indexes(lines, "scripts/audit") == []


def test_build_compiles_only_the_lambdas_the_stacks_deploy():
    """The owner's tools (`genesis-codes`, `takedown`) build with features the Lambdas don't
    need, such as `aws-config`'s `credentials-login`, and cargo unifies features across one
    build: so the deploy's build names each deployed Lambda's package and nothing else
    (`.stories/2-identity-api/issues/20-owner-tools-use-aws-login.md`)."""
    lines = command_lines("build")
    (build,) = [lines[i] for i in indexes(lines, "cargo lambda build")]
    packages = re.findall(r"--package ([\w-]+)", build)
    lambda_dir = sfab_config.REPO_ROOT / "lambda"
    expected = [
        tomllib.loads((lambda_dir / name / "Cargo.toml").read_text())["package"]["name"]
        for name in app.LAMBDAS
    ]
    assert packages == expected


def test_check_runs_build_with_its_argument_before_the_audit_and_the_tests():
    lines = command_lines("check")

    build = indexes(lines, "scripts/build")
    assert len(build) == 1, lines
    assert 'scripts/build "${require_secret[@]}"' in lines[build[0]]
    audit = indexes(lines, "scripts/audit")
    tests = indexes(lines, "cargo test") + indexes(lines, "pytest")
    assert audit and tests
    assert build[0] < audit[0] < min(tests)
    # The steps scripts/build now runs aren't run twice.
    assert indexes(lines, "cargo lambda build") == []
    assert indexes(lines, "generate_website.py") == []


def test_deploy_runs_build_in_the_require_secret_mode_only_and_never_the_audit_or_tests():
    lines = command_lines("deploy")

    arms = case_arms(lines)
    assert arms["require-secret"] == "scripts/build --require-secret"
    assert "scripts/build" not in arms["fallback"]
    assert len(indexes(lines, "scripts/build")) == 1
    for never in ("scripts/check", "scripts/audit", "cargo test"):
        assert indexes(lines, never) == [], never
    for i in indexes(lines, "pytest"):
        assert "tests/test_openapi.py" in lines[i], lines[i]
