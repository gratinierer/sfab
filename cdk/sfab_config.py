"""The one parser for `sfab.config.toml`, the owner's private per-deploy values.

It reads `sfab.config.toml` at the repo root, or the file named by the
`SFAB_CONFIG` environment variable, which CI points at the committed test
fixture. Any missing file, missing value, `CHANGE_ME` or badly formed value
fails with one message naming every problem, so a copied example or a
forgotten value can't silently drop the alerts.

`smoke_claim` is allowed but ignored here: only the smoke script reads it.
"""

from __future__ import annotations

import os
import re
import tomllib
from dataclasses import dataclass, fields
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_PATH = REPO_ROOT / "sfab.config.toml"
EXAMPLE_PATH = REPO_ROOT / "sfab.config.example.toml"
ENV_VAR = "SFAB_CONFIG"


@dataclass(frozen=True)
class Config:
    """Every required key; add a new one here and the parser requires it."""

    account_id: str
    alert_email: str
    abuse_contact: str
    domain_name: str
    hosted_zone_id: str
    origin_key: str


REQUIRED_KEYS = tuple(f.name for f in fields(Config))
# Read only by the smoke script, never by synth.
IGNORED_KEYS = ("smoke_claim",)

PLACEHOLDER = "CHANGE_ME"
_ORIGIN_KEY = re.compile(r"[A-Za-z0-9]{20,128}")
_ACCOUNT_ID = re.compile(r"[0-9]{12}")


class ConfigError(Exception):
    """The config can't be used; the message says why and where."""


def config_path(environ: dict[str, str] | None = None) -> Path:
    environ = os.environ if environ is None else environ
    named = environ.get(ENV_VAR)
    return Path(named) if named else DEFAULT_PATH


def load(path: Path | None = None) -> Config:
    path = config_path() if path is None else path
    try:
        raw = path.read_text(encoding="utf-8")
    except FileNotFoundError:
        raise ConfigError(
            f"{path}: config file not found. Copy {EXAMPLE_PATH.name} to "
            f"{DEFAULT_PATH.name} and fill in every value, or set {ENV_VAR}."
        ) from None
    try:
        data = tomllib.loads(raw)
    except tomllib.TOMLDecodeError as e:
        raise ConfigError(f"{path}: not valid TOML: {e}") from None
    return parse(data, source=str(path))


def parse(data: dict, source: str = "config") -> Config:
    problems: list[str] = []

    unknown = sorted(set(data) - set(REQUIRED_KEYS) - set(IGNORED_KEYS))
    if unknown:
        problems.append(f"unknown key(s), possibly misspelt: {', '.join(unknown)}")

    values: dict[str, str] = {}
    for key in REQUIRED_KEYS:
        value = data.get(key)
        if value is None or (isinstance(value, str) and not value.strip()):
            problems.append(f"{key}: missing")
        elif not isinstance(value, str):
            problems.append(f"{key}: must be a string")
        elif PLACEHOLDER in value:
            problems.append(f"{key}: still {PLACEHOLDER}")
        else:
            values[key] = value.strip()

    if "origin_key" in values and not _ORIGIN_KEY.fullmatch(values["origin_key"]):
        problems.append("origin_key: must be 20 to 128 letters and digits (A-Z, a-z, 0-9)")
    if "account_id" in values and not _ACCOUNT_ID.fullmatch(values["account_id"]):
        problems.append("account_id: must be 12 digits")

    if problems:
        raise ConfigError(f"{source}:\n  " + "\n  ".join(problems))
    return Config(**values)
