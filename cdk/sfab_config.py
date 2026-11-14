"""The one loader for the owner's private per-deploy values.

They live in the Secrets Manager secret `sfab/config` in us-east-1, a JSON object with one
entry per key, which the owner fills by hand. When the `SFAB_CONFIG` environment variable is
set, the loader reads the TOML file it names instead and never calls AWS: CI and the tests
point it at the committed fixture. Any missing secret, missing value, `CHANGE_ME` or badly
formed value fails with one message naming every problem, so a forgotten value can't
silently drop the alerts.

`account_id` isn't in the secret: a deploy targets the account of its credentials
(`deploy_account()`). The fixture carries it, so tests never call STS.

`smoke_claim` is optional and only checked to be a string here: only the smoke script's
`--production` checks its form.
"""

from __future__ import annotations

import json
import os
import re
import sys
import tomllib
from collections.abc import Callable, Mapping
from dataclasses import MISSING, Field, dataclass, fields
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
FIXTURE_PATH = REPO_ROOT / "cdk" / "tests" / "fixtures" / "sfab.config.test.toml"
ENV_VAR = "SFAB_CONFIG"
SECRET_ID = "sfab/config"
SECRET_REGION = "us-east-1"
SECRET = f"{SECRET_ID} in {SECRET_REGION}"


@dataclass(frozen=True)
class Config:
    account_id: str
    alert_email: str
    abuse_contact: str
    domain_name: str
    hosted_zone_id: str
    origin_key: str
    ip_hash_key: str
    smoke_claim: str | None = None

    @property
    def api_host(self) -> str:
        """The API's host, `api.<domain_name>`."""
        return f"api.{self.domain_name}"

    @property
    def api_url(self) -> str:
        """The API's public base URL, without the stage."""
        return f"https://{self.api_host}"


# Only the fixture carries it; a deploy takes it from its credentials.
FIXTURE_ONLY_KEYS = ("account_id",)


def has_default(f: Field) -> bool:
    """Whether a dataclass field has a default, as a value or a factory."""
    return f.default is not MISSING or f.default_factory is not MISSING


# The secret's required entries: every `Config` field without a default, so a new key is one field.
REQUIRED_KEYS = tuple(
    f.name for f in fields(Config) if not has_default(f) and f.name not in FIXTURE_ONLY_KEYS
)
# The fields with a default, such as `smoke_claim`, read only by the smoke script, which checks its form.
OPTIONAL_KEYS = tuple(f.name for f in fields(Config) if has_default(f))

PLACEHOLDER = "CHANGE_ME"
_ORIGIN_KEY = re.compile(r"[A-Za-z0-9]{20,128}")
_IP_HASH_KEY = re.compile(r"[A-Za-z0-9]{32,128}")
_ACCOUNT_ID = re.compile(r"[0-9]{12}")


class ConfigError(Exception):
    """The config can't be used; the message says why and where."""


class Aws:
    """The AWS calls the loader and the deploy plan make, behind one seam the tests stub."""

    def client(self, service: str, region: str | None = None):
        import boto3

        return boto3.client(service, region_name=region)

    def region(self, environ: Mapping[str, str]) -> str | None:
        """The region a deploy runs in: `AWS_REGION`, then what boto3 resolves
        (`AWS_DEFAULT_REGION`, then the profile). boto3 itself ignores `AWS_REGION`."""
        if environ.get("AWS_REGION"):
            return environ["AWS_REGION"]
        import boto3

        return boto3.session.Session().region_name


def load(environ: Mapping[str, str] | None = None, aws: Aws | None = None) -> Config:
    """The config from the fixture `SFAB_CONFIG` names, or else from the secret."""
    environ = os.environ if environ is None else environ
    aws = Aws() if aws is None else aws
    if environ.get(ENV_VAR):
        return _load_fixture(Path(environ[ENV_VAR]))
    values = parse(_read_secret(aws), source=SECRET)
    return Config(account_id=deploy_account(environ, aws), **values)


def deploy_account(environ: Mapping[str, str] | None = None, aws: Aws | None = None) -> str:
    """The account a deploy targets: `CDK_DEFAULT_ACCOUNT`, which the CDK CLI sets from the
    credentials, then the fixture's `account_id` when `SFAB_CONFIG` names a loadable one, then
    the caller identity from STS."""
    environ = os.environ if environ is None else environ
    aws = Aws() if aws is None else aws
    if environ.get("CDK_DEFAULT_ACCOUNT"):
        return environ["CDK_DEFAULT_ACCOUNT"]
    fixture_problem = ""
    if environ.get(ENV_VAR):
        try:
            return _load_fixture(Path(environ[ENV_VAR])).account_id
        except ConfigError as e:
            fixture_problem = f" {ENV_VAR}'s fixture didn't give one either: {e}."
    try:
        return aws.client("sts").get_caller_identity()["Account"]
    except Exception as e:  # noqa: BLE001 - any boto3 failure means no account
        raise ConfigError(
            f"can't find the deploy's account with sts get-caller-identity: {e}.{fixture_problem} "
            "Log in to AWS (aws login), or set SFAB_CONFIG to a fixture."
        ) from None


def load_for_check(
    require_secret: bool,
    environ: Mapping[str, str] | None = None,
    aws: Aws | None = None,
    warn: Callable[[str], None] | None = None,
) -> Config:
    """scripts/check's config: a set `SFAB_CONFIG`'s fixture, else the secret, else, unless
    `require_secret`, the committed fixture with a message naming why the secret didn't load."""
    environ = os.environ if environ is None else environ
    warn = (lambda m: print(m, file=sys.stderr)) if warn is None else warn
    if environ.get(ENV_VAR):
        if require_secret:
            raise ConfigError(
                f"{ENV_VAR} is set ({environ[ENV_VAR]}), but a deploy reads {SECRET}. Unset it."
            )
        return load(environ, aws)
    try:
        return load(environ, aws)
    except ConfigError as e:
        if require_secret:
            raise
        warn(f"check: the config doesn't load, so using {FIXTURE_PATH}:\n  {e}")
        return _load_fixture(FIXTURE_PATH)


def parse(data: dict, source: str = "config", *, fixture: bool = False) -> dict[str, str]:
    """The checked values from `data`, without `account_id` unless `fixture`."""
    required = REQUIRED_KEYS + FIXTURE_ONLY_KEYS if fixture else REQUIRED_KEYS
    problems: list[str] = []

    unknown = sorted(set(data) - set(required) - set(OPTIONAL_KEYS))
    if unknown:
        problems.append(f"unknown key(s), possibly misspelt: {', '.join(unknown)}")

    values: dict[str, str] = {}
    for key in required:
        value = data.get(key)
        if value is None or (isinstance(value, str) and not value.strip()):
            problems.append(f"{key}: missing")
        elif not isinstance(value, str):
            problems.append(f"{key}: must be a string")
        elif PLACEHOLDER in value:
            problems.append(f"{key}: still {PLACEHOLDER}")
        else:
            values[key] = value.strip()
    for key in OPTIONAL_KEYS:
        if key in data:
            if isinstance(data[key], str):
                values[key] = data[key]
            else:
                problems.append(f"{key}: must be a string")

    if "origin_key" in values and not _ORIGIN_KEY.fullmatch(values["origin_key"]):
        problems.append("origin_key: must be 20 to 128 letters and digits (A-Z, a-z, 0-9)")
    if "ip_hash_key" in values and not _IP_HASH_KEY.fullmatch(values["ip_hash_key"]):
        problems.append("ip_hash_key: must be 32 to 128 letters and digits (A-Z, a-z, 0-9)")
    if "account_id" in values and not _ACCOUNT_ID.fullmatch(values["account_id"]):
        problems.append("account_id: must be 12 digits")

    if problems:
        raise ConfigError(f"{source}:\n  " + "\n  ".join(problems))
    return values


def _load_fixture(path: Path) -> Config:
    try:
        raw = path.read_text(encoding="utf-8")
    except FileNotFoundError:
        raise ConfigError(
            f"{path}: config file not found. {ENV_VAR} names it; unset {ENV_VAR} to read {SECRET}."
        ) from None
    try:
        data = tomllib.loads(raw)
    except tomllib.TOMLDecodeError as e:
        raise ConfigError(f"{path}: not valid TOML: {e}") from None
    return Config(**parse(data, source=str(path), fixture=True))


_FILL = f"Fill it with one JSON entry per key: {', '.join(REQUIRED_KEYS)}."


def _read_secret(aws: Aws) -> dict:
    from botocore.exceptions import BotoCoreError, ClientError

    try:
        response = aws.client("secretsmanager", SECRET_REGION).get_secret_value(SecretId=SECRET_ID)
    except ClientError as e:
        error = e.response.get("Error", {})
        if error.get("Code") == "ResourceNotFoundException":
            # GetSecretValue says the same code for a secret with no value yet, naming the label.
            if "staging label" in error.get("Message", ""):
                raise ConfigError(f"{SECRET}: it has no value. {_FILL}") from None
            raise ConfigError(
                f"{SECRET}: the secret is missing. Deploy SfabDeployer, which creates it. {_FILL}"
            ) from None
        raise ConfigError(f"{SECRET}: the secret can't be read: {e}") from None
    except BotoCoreError as e:
        raise ConfigError(f"{SECRET}: the secret can't be read: {e}") from None

    value = response.get("SecretString")
    if not value or not value.strip():
        raise ConfigError(f"{SECRET}: it has no value. {_FILL}")
    try:
        data = json.loads(value)
    except json.JSONDecodeError:
        data = None
    if not isinstance(data, dict):
        raise ConfigError(f"{SECRET}: its value isn't a JSON object. {_FILL}")
    return data
