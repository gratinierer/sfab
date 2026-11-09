"""What scripts/deploy may deploy: `.stories/18-deploy-from-codebuild/spec.md` "A build".

    python deploy_plan.py --all | <stack>...

Checks, in this order, stopping at the first that fails: `SFAB_CONFIG` isn't set, so the
fixture can't deploy; the arguments are `--all` or one or more of the app's stacks, so a
build's `DEPLOY_STACKS` can't carry other CDK flags; the deploy runs in us-east-1; and the
config loads from the secret. On success it prints the mode scripts/check runs in
(`require-secret`) and then the stacks for `cdk deploy`, one per line; on failure it prints
every problem to stderr and exits 1. scripts/deploy is a thin wrapper around it, so no
environment variable changes which checks run.
"""

from __future__ import annotations

import os
import sys
from collections.abc import Mapping
from dataclasses import dataclass, field

import sfab_config

STACKS = ("SfabBase", "SfabApi", "SfabWebsite")
ALL = "--all"
REGION = "us-east-1"
REQUIRE_SECRET = "require-secret"


@dataclass(frozen=True)
class Plan:
    """The `cdk deploy` stack arguments, and whether scripts/check must read the secret;
    or, when `errors` isn't empty, why nothing may deploy."""

    stacks: list[str] = field(default_factory=list)
    require_secret: bool = True
    errors: list[str] = field(default_factory=list)


def plan(
    argv: list[str],
    environ: Mapping[str, str] | None = None,
    aws: sfab_config.Aws | None = None,
) -> Plan:
    environ = os.environ if environ is None else environ
    aws = sfab_config.Aws() if aws is None else aws

    if environ.get(sfab_config.ENV_VAR):
        return Plan(
            errors=[
                f"{sfab_config.ENV_VAR} is set ({environ[sfab_config.ENV_VAR]}); a deploy reads "
                f"{sfab_config.SECRET}, never the fixture. Unset it."
            ]
        )

    errors = _check_arguments(argv)
    if errors:
        return Plan(errors=errors)

    region = aws.region(environ)
    if region != REGION:
        found = region if region else "no region"
        return Plan(
            errors=[
                f"the deploy runs in {found}, but sfab deploys only to {REGION}. "
                f"Set AWS_REGION={REGION}."
            ]
        )

    try:
        sfab_config.load(environ, aws)
    except sfab_config.ConfigError as e:
        return Plan(errors=[f"the config doesn't load: {e}"])
    return Plan(stacks=list(argv), require_secret=True)


def _check_arguments(argv: list[str]) -> list[str]:
    usage = f"name {ALL} or one or more of: {', '.join(STACKS)}"
    if not argv:
        return [f"nothing to deploy: {usage}"]
    if argv == [ALL]:
        return []
    refused = [a for a in argv if a not in STACKS]
    if refused:
        return [f"refused: {' '.join(refused)}. Deploy takes only {usage}"]
    return []


def main(
    argv: list[str] | None = None,
    environ: Mapping[str, str] | None = None,
    aws: sfab_config.Aws | None = None,
) -> int:
    found = plan(sys.argv[1:] if argv is None else argv, environ, aws)
    if found.errors:
        print("deploy:", *found.errors, sep="\n  ", file=sys.stderr)
        return 1
    print(REQUIRE_SECRET if found.require_secret else "fallback")
    print(*found.stacks, sep="\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
