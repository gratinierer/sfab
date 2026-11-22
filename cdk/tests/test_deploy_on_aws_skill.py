"""What the `/deploy-on-aws` skill may hold, and what it relies on in the repo:
`.stories/23-deploy-on-aws-skill/spec.md` "Decisions". The skill is a document an agent follows,
so no test runs it; these check its text is account-free and that `.env`, where it reads
`AWS_PROFILE`, can't be committed.
"""

import re
import subprocess

import sfab_config

SKILL = sfab_config.REPO_ROOT / ".claude" / "skills" / "deploy-on-aws" / "SKILL.md"

# The account-specific shapes the spec's "No account-specific data" names, as scripts/pii-scan
# spells the first two: letters and digits next to a match end it.
ACCOUNT_ID = re.compile(r"(?<![0-9A-Za-z])\d{12}(?![0-9A-Za-z])")
ARN = re.compile(r"\barn:aws[\w-]*:")
EMAIL = re.compile(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,}")


def test_the_patterns_catch_what_they_name():
    assert ACCOUNT_ID.search("account 123456789012 here")
    assert not ACCOUNT_ID.search("sha 1234567890123")
    assert ARN.search("arn:aws:codebuild:us-east-1:x:project/y")
    assert EMAIL.search("someone@example.invalid")


def test_the_skill_holds_no_account_id_arn_or_email():
    text = SKILL.read_text()
    assert ACCOUNT_ID.findall(text) == []
    assert ARN.findall(text) == []
    assert EMAIL.findall(text) == []


def test_dot_env_at_the_repo_root_is_ignored():
    result = subprocess.run(
        ["git", "check-ignore", "-q", ".env"], cwd=sfab_config.REPO_ROOT, check=False
    )
    assert result.returncode == 0
