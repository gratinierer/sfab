"""The world's committed config files, checked at synth: `world/founders.yaml`,
`world/quarters.yaml` and `world/shelves.yaml` (`.stories/4-life-in-the-world/spec.md`
"Foundations", "Vouches", "Homes and quarters" and "The Library").

The crates under `lambda/world/` compile the same files in; this module is the synth check, so
a bad file fails `cdk synth` rather than a request. Founders are agent IDs, at least 3, because
nobody else could ever gain standing otherwise. Quarter, lane and shelf names are storage key
segments and path parameters, so they're `[a-z0-9-]`, 1 to 64 characters, each listed once.

`world/founders.yaml` holds the real founders' IDs, which `.stories/4-life-in-the-world/`
ticket 11 commits. Until then, and in every test, `world/founders.fixture.yaml` stands in with
made-up IDs, and `deploy_plan.py` refuses a deploy that would carry them (`founders_problems`).
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path

import yaml

REPO_ROOT = Path(__file__).resolve().parent.parent
WORLD = REPO_ROOT / "world"
FOUNDERS = WORLD / "founders.yaml"
FOUNDERS_FIXTURE = WORLD / "founders.fixture.yaml"
QUARTERS = WORLD / "quarters.yaml"
SHELVES = WORLD / "shelves.yaml"

MIN_FOUNDERS = 3
# An agent ID: 12 lowercase RFC 4648 base32 characters (lambda/core/src/id.rs).
AGENT_ID = re.compile(r"[a-z2-7]{12}")
NAME = re.compile(r"[a-z0-9-]{1,64}")


class WorldConfigError(Exception):
    """A world config file is missing or breaks a rule; the message names the file."""


@dataclass(frozen=True)
class Files:
    founders: Path
    quarters: Path
    shelves: Path

    @classmethod
    def under(cls, folder: Path) -> Files:
        """`founders.yaml`, `quarters.yaml` and `shelves.yaml` in one folder."""
        return cls(folder / "founders.yaml", folder / "quarters.yaml", folder / "shelves.yaml")


# What a deploy uses, and what the tests and an unconfigured synth use.
REAL_FILES = Files(FOUNDERS, QUARTERS, SHELVES)
FIXTURE_FILES = Files(FOUNDERS_FIXTURE, QUARTERS, SHELVES)


@dataclass(frozen=True)
class Quarter:
    name: str
    lanes: tuple[str, ...]


@dataclass(frozen=True)
class World:
    founders: tuple[str, ...]
    quarters: tuple[Quarter, ...]
    shelves: tuple[str, ...]


def load(files: Files = REAL_FILES) -> World:
    return World(
        founders=_founders(files.founders),
        quarters=_quarters(files.quarters),
        shelves=_shelves(files.shelves),
    )


def founders_problems(files: Files = REAL_FILES) -> list[str]:
    """Why these founders mustn't deploy: the file is missing or broken, or it holds a fixture
    ID. Empty when it may."""
    try:
        founders = _founders(files.founders)
    except WorldConfigError as e:
        return [f"{e}. Ticket 11 of .stories/4-life-in-the-world/ commits the real founders."]
    fixture = set(_founders(FOUNDERS_FIXTURE))
    made_up = [f for f in founders if f in fixture]
    if made_up:
        return [
            f"{files.founders.name} holds the fixture's made-up founders {', '.join(made_up)}; "
            "a deploy needs the real founders' IDs."
        ]
    return []


def _read(path: Path, key: str) -> list:
    if not path.is_file():
        raise WorldConfigError(f"{path.name}: file not found at {path}")
    try:
        doc = yaml.safe_load(path.read_text(encoding="utf-8"))
    except yaml.YAMLError as e:
        raise WorldConfigError(f"{path.name}: not valid YAML: {e}") from None
    found = doc.get(key) if isinstance(doc, dict) else None
    if not isinstance(found, list) or not found:
        raise WorldConfigError(f"{path.name}: needs a non-empty `{key}` list")
    return found


def _once(path: Path, kind: str, items: list[str]) -> None:
    seen: set[str] = set()
    for item in items:
        if item in seen:
            raise WorldConfigError(f"{path.name}: {kind} {item!r} is listed twice")
        seen.add(item)


def _names(path: Path, kind: str, items: list) -> tuple[str, ...]:
    for item in items:
        if not isinstance(item, str) or not NAME.fullmatch(item):
            raise WorldConfigError(
                f"{path.name}: {kind} {item!r} must be 1 to 64 characters of a-z, 0-9 and -"
            )
    _once(path, kind, items)
    return tuple(items)


def _founders(path: Path) -> tuple[str, ...]:
    items = _read(path, "founders")
    for item in items:
        if not isinstance(item, str) or not AGENT_ID.fullmatch(item):
            raise WorldConfigError(f"{path.name}: founder {item!r} isn't an agent ID")
    _once(path, "founder", items)
    if len(items) < MIN_FOUNDERS:
        raise WorldConfigError(
            f"{path.name}: needs at least {MIN_FOUNDERS} founders, found {len(items)}"
        )
    return tuple(items)


def _quarters(path: Path) -> tuple[Quarter, ...]:
    quarters = []
    for item in _read(path, "quarters"):
        name = item.get("name") if isinstance(item, dict) else None
        (name,) = _names(path, "quarter", [name])
        lanes = item.get("lanes")
        if not isinstance(lanes, list) or not lanes:
            raise WorldConfigError(f"{path.name}: quarter {name!r} needs a non-empty `lanes` list")
        quarters.append(Quarter(name, _names(path, f"{name}'s lane", lanes)))
    _once(path, "quarter", [q.name for q in quarters])
    return tuple(quarters)


def _shelves(path: Path) -> tuple[str, ...]:
    return _names(path, "shelf", _read(path, "shelves"))
