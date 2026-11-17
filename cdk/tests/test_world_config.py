"""`cdk/world_config.py`: the synth checks on `world/founders.yaml`, `world/quarters.yaml` and
`world/shelves.yaml`, `.stories/4-life-in-the-world/spec.md` "Foundations" and "Vouches"."""

import re
from pathlib import Path

import pytest

import world_config

FOUNDERS = "founders:\n  - foundera2222\n  - founderb2222\n  - founderc2222\n"
QUARTERS = "quarters:\n  - name: harbour\n    lanes: [lantern-lane, rope-walk]\n"
SHELVES = "shelves: [craft, 7-seas]\n"


def files(tmp_path: Path, founders=FOUNDERS, quarters=QUARTERS, shelves=SHELVES) -> world_config.Files:
    for name, text in (("founders", founders), ("quarters", quarters), ("shelves", shelves)):
        (tmp_path / f"{name}.yaml").write_text(text)
    return world_config.Files.under(tmp_path)


def test_three_founders_and_good_names_load(tmp_path):
    world = world_config.load(files(tmp_path))

    assert world.founders == ("foundera2222", "founderb2222", "founderc2222")
    assert world.quarters == (world_config.Quarter("harbour", ("lantern-lane", "rope-walk")),)
    assert world.shelves == ("craft", "7-seas")


def test_the_committed_files_with_the_fixture_founders_load():
    world = world_config.load(world_config.FIXTURE_FILES)

    assert len(world.founders) >= world_config.MIN_FOUNDERS
    assert world.quarters and world.shelves


def test_fewer_than_three_founders_fail(tmp_path):
    two = "founders:\n  - foundera2222\n  - founderb2222\n"

    with pytest.raises(world_config.WorldConfigError, match="at least 3 founders"):
        world_config.load(files(tmp_path, founders=two))


@pytest.mark.parametrize(
    "founders, named",
    [
        ("founders:\n  - foundera2222\n  - founderb2222\n  - FounderC2222\n", "FounderC2222"),
        ("founders:\n  - foundera2222\n  - founderb2222\n  - founderb2222\n", "founderb2222"),
        ("founders: foundera2222\n", "list"),
    ],
)
def test_a_founder_that_isnt_an_agent_id_or_comes_twice_fails(tmp_path, founders, named):
    with pytest.raises(world_config.WorldConfigError, match=named):
        world_config.load(files(tmp_path, founders=founders))


@pytest.mark.parametrize(
    "quarters, named",
    [
        ("quarters:\n  - name: Harbour\n    lanes: [lantern-lane]\n", "Harbour"),
        ("quarters:\n  - name: harbour\n    lanes: [lantern_lane]\n", "lantern_lane"),
        ("quarters:\n  - name: harbour\n    lanes: [a/b]\n", "a/b"),
        ("quarters:\n  - name: harbour\n    lanes: []\n", "harbour"),
        ("quarters:\n  - name: harbour\n    lanes: [x, x]\n", "x"),
        (
            "quarters:\n  - name: harbour\n    lanes: [x]\n  - name: harbour\n    lanes: [y]\n",
            "harbour",
        ),
        ("quarters: []\n", "quarters"),
    ],
)
def test_a_bad_quarter_or_lane_name_fails(tmp_path, quarters, named):
    with pytest.raises(world_config.WorldConfigError, match=named):
        world_config.load(files(tmp_path, quarters=quarters))


@pytest.mark.parametrize(
    "shelves, named",
    [("shelves: [craft, 'old books']\n", "old books"), ("shelves: [a, a]\n", "a"), ("shelves: []\n", "shelves")],
)
def test_a_bad_shelf_name_fails(tmp_path, shelves, named):
    with pytest.raises(world_config.WorldConfigError, match=named):
        world_config.load(files(tmp_path, shelves=shelves))


def test_the_name_rule_is_the_crates_rule():
    # lambda/core/src/config.rs parses the same files at runtime with its own copy of the rule.
    core = (world_config.REPO_ROOT / "lambda" / "core" / "src" / "config.rs").read_text()
    found = re.search(r"pub const NAME_MAX: usize = (\d+);", core)
    assert found
    limit = int(found[1])
    assert world_config.NAME.fullmatch("a" * limit)
    assert not world_config.NAME.fullmatch("a" * (limit + 1))
    vouches = (world_config.REPO_ROOT / "lambda" / "world" / "vouches" / "src" / "lib.rs").read_text()
    found = re.search(r"pub const MIN_FOUNDERS: usize = (\d+);", vouches)
    assert found and int(found[1]) == world_config.MIN_FOUNDERS


def test_a_missing_file_fails_naming_it(tmp_path):
    found = files(tmp_path)
    found.founders.unlink()

    with pytest.raises(world_config.WorldConfigError, match="founders.yaml"):
        world_config.load(found)
