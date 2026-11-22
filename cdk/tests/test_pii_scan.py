"""scripts/pii-scan on a throwaway repo built in each test: .stories/done/12-public-repo/spec.md "The
personal data scan" and issue 03. The values planted here are made up."""

import os
import subprocess
from pathlib import Path

import pytest

import sfab_config

SCRIPT = sfab_config.REPO_ROOT / "scripts" / "pii-scan"
IDENTITY = {
    "GIT_AUTHOR_NAME": "Test Bot",
    "GIT_AUTHOR_EMAIL": "bot@example.invalid",
    "GIT_COMMITTER_NAME": "Test Bot",
    "GIT_COMMITTER_EMAIL": "bot@example.invalid",
}
LITERAL = "quokkastreet"
# Joined at runtime, so this file never holds a value the scan of this repo would report.
EMAIL = "max.muster" + "@mail.de"
ACCOUNT_ID = "112233" + "445566"
PHONE = "+49 40" + " 7654321"
ZONE_ID = "Z9XYZ" + "876WVU54"
PRIVATE = "mallory" + "@botrefugium.com"


class Repo:
    def __init__(self, path: Path):
        self.path = path
        self.env = {**os.environ, **IDENTITY, "GIT_CONFIG_GLOBAL": os.devnull}
        self.git("init", "-q", "-b", "main")

    def git(self, *args: str, env: dict | None = None) -> str:
        return subprocess.run(
            ["git", "-C", str(self.path), *args], check=True, capture_output=True, text=True,
            env={**self.env, **(env or {})},
        ).stdout

    def commit(self, files: dict[str, str | bytes], message: str = "change", **env: str) -> None:
        for name, content in files.items():
            target = self.path / name
            target.parent.mkdir(parents=True, exist_ok=True)
            if isinstance(content, bytes):
                target.write_bytes(content)
            else:
                target.write_text(content)
            self.git("add", name)
        self.git("commit", "-q", "-m", message, env=env)


def run(repo: Repo, literals: Path) -> subprocess.CompletedProcess:
    return subprocess.run(
        [str(SCRIPT), str(literals), "--repo", str(repo.path)], capture_output=True, text=True
    )


@pytest.fixture
def repo(tmp_path) -> Repo:
    path = tmp_path / "repo"
    path.mkdir()
    return Repo(path)


@pytest.fixture
def literals(tmp_path) -> Path:
    path = tmp_path / "pii.md"
    path.write_text(f"# the owner's list\n\n* {LITERAL}\n- watchdog@botrefugium.com\n")
    return path


def lines(result: subprocess.CompletedProcess) -> list[str]:
    return result.stdout.splitlines()


def test_each_planted_value_is_found_once_however_many_commits_hold_it(repo, literals):
    planted = f"street {LITERAL}\nmail {EMAIL}\naccount {ACCOUNT_ID}\nphone {PHONE}\nzone {ZONE_ID}\n"
    repo.commit({"notes.md": planted})
    for n in range(5):
        repo.commit({"notes.md": planted + f"edit {n}\n", "other.txt": f"unrelated {n}\n"})

    result = run(repo, literals)

    assert result.returncode == 1, result.stderr
    found = lines(result)
    for name in ("literal 3", "email", "account-id", "phone", "hosted-zone-id"):
        hits = [line for line in found if "notes.md:" in line and f" {name}" in line]
        assert len(hits) == 1, (name, found)
        assert "(+5 more commits)" in hits[0], hits[0]
    first = repo.git("rev-list", "--max-parents=0", "HEAD").strip()
    assert all(line.startswith(first[:12]) for line in found), found


def test_a_literal_is_found_in_commit_metadata_tags_and_ref_names(repo, literals):
    repo.commit({"a.txt": "nothing here\n"}, GIT_AUTHOR_NAME=f"Ann {LITERAL.title()}")
    repo.git("branch", f"topic-{LITERAL}")
    repo.git("tag", "-a", "v1", "-m", f"release by {LITERAL}")
    repo.commit({"a.txt": "still nothing\n"}, message=f"thanks {LITERAL}")

    found = lines(run(repo, literals))

    for where in ("commit author:1", "ref name:1", "tag:2", "commit message:1"):
        assert any(f" {where} literal 3" in line for line in found), (where, found)


def test_allowlisted_values_and_binary_files_are_not_reported(repo, literals):
    repo.commit({
        "a.txt": "abuse@example.invalid 000000000000 noreply@anthropic.com\n",
        "blob.bin": b"\0\x01" + f"{EMAIL} {ACCOUNT_ID}".encode(),
    })

    result = run(repo, literals)

    assert result.returncode == 0, result.stdout
    assert result.stdout == ""


def test_literals_bypass_the_allowlist_and_an_exact_entry_does_not_cover_its_domain(repo, literals):
    repo.commit({"a.txt": f"contact watchdog@botrefugium.com\nprivate {PRIVATE}\n"})

    found = lines(run(repo, literals))

    masked = PRIVATE[0] + "*" * (len(PRIVATE) - 2) + PRIVATE[-1]
    assert [line.split(" ", 1)[1] for line in found] == ["a.txt:1 literal 4", f"a.txt:2 email {masked}"]


def test_the_report_never_holds_a_full_match(repo, literals):
    repo.commit({"a.txt": f"{LITERAL} {EMAIL} {ACCOUNT_ID} {PHONE} {ZONE_ID}\n"})

    result = run(repo, literals)

    report = result.stdout + result.stderr
    for value in (LITERAL, EMAIL, ACCOUNT_ID, PHONE, ZONE_ID):
        assert value not in report
    assert f" email {EMAIL[0]}{'*' * (len(EMAIL) - 2)}{EMAIL[-1]}" in result.stdout


def test_a_clean_repo_exits_0(repo, literals):
    repo.commit({"README.md": "Nothing personal.\n"})

    result = run(repo, literals)

    assert (result.returncode, result.stdout) == (0, "")


def test_errors_exit_2(tmp_path, repo, literals):
    repo.commit({"a.txt": "x\n"})
    empty = tmp_path / "empty.md"
    empty.write_text("# nothing\n")

    assert run(repo, tmp_path / "missing.md").returncode == 2
    assert run(repo, empty).returncode == 2
    not_a_repo = tmp_path / "plain"
    not_a_repo.mkdir()
    result = subprocess.run(
        [str(SCRIPT), str(literals), "--repo", str(not_a_repo)], capture_output=True, text=True
    )
    assert result.returncode == 2, result.stderr
