#!/usr/bin/env python3
"""Generate the public API description served at `GET /v1/openapi.yaml`.

From `openapi/sfab.yaml` it removes every `x-amazon-*` extension, the `$default`
validator content type, and the origin key's `apiKey` scheme with every reference to
it, then merges in `openapi/public-security.yaml`, adds a relative `servers` entry of
`/v1`, and puts the config's `abuse_contact` in `info.contact`. It refuses to write a
copy that leaks anything private. See .stories/2-identity-api/spec.md "Architecture".

The copy is written to build/openapi.yaml, which the API Lambda compiles in, and is
never committed.

The config is scripts/check's (`sfab_config.load_for_check`): the secret, or the fixture
with a message, unless `--require-secret`, which scripts/deploy passes through.
"""

from __future__ import annotations

import copy
import re
import sys
from pathlib import Path

import yaml

import sfab_config

ROOT = sfab_config.REPO_ROOT
SOURCE = ROOT / "openapi" / "sfab.yaml"
PUBLIC_SECURITY = ROOT / "openapi" / "public-security.yaml"
OUTPUT = ROOT / "build" / "openapi.yaml"

METHODS = ("get", "put", "post", "delete", "patch", "head", "options")
_ACCOUNT_ID = re.compile(r"(?<![0-9])[0-9]{12}(?![0-9])")


class GenerateError(Exception):
    pass


def load_yaml(path: Path) -> dict:
    return yaml.safe_load(path.read_text(encoding="utf-8"))


def operations(doc: dict):
    """Yields (method, path, operation), with the method upper case."""
    for path, item in doc.get("paths", {}).items():
        for method in METHODS:
            if method in item:
                yield method.upper(), path, item[method]


def _strip(node):
    if isinstance(node, dict):
        return {
            k: _strip(v)
            for k, v in node.items()
            if not str(k).startswith("x-amazon-")
            # `$default` is a gateway validator content type, not a media type.
            and not (str(k).startswith("$") and k != "$ref")
        }
    if isinstance(node, list):
        return [_strip(v) for v in node]
    return node


def build_public(source: dict, security: dict, abuse_contact: str) -> dict:
    doc = _strip(copy.deepcopy(source))

    schemes = doc.setdefault("components", {}).setdefault("securitySchemes", {})
    private = {name for name, s in schemes.items() if s.get("type") == "apiKey"}
    for name in private:
        del schemes[name]

    bearer = set(security.get("bearer", []))
    known = {f"{m} {p}" for m, p, _ in operations(doc)}
    unknown = sorted(bearer - known)
    if unknown:
        raise GenerateError(f"public-security.yaml names operations not in sfab.yaml: {unknown}")

    for method, path, op in operations(doc):
        kept = [r for r in op.get("security", []) if not set(r) & private]
        if f"{method} {path}" in bearer:
            kept.append({name: [] for name in security["securitySchemes"]})
        if kept:
            op["security"] = kept
        else:
            op.pop("security", None)

    schemes.update(copy.deepcopy(security["securitySchemes"]))
    doc["servers"] = [{"url": "/v1"}]
    key = "email" if "@" in abuse_contact and "://" not in abuse_contact else "url"
    doc["info"]["contact"] = {"name": "Abuse and takedown reports", key: abuse_contact}

    # Keep `openapi`, `info` and `servers` first, as readers expect.
    head = {k: doc.pop(k) for k in ("openapi", "info", "servers")}
    return head | doc


def check_public(text: str, config: sfab_config.Config, bearer: list[str]) -> list[str]:
    """Everything the public copy must never contain, and what it must contain.

    It checks only the text, as written and served, parsing it for the structural checks.
    """
    doc = yaml.safe_load(text) or {}
    problems = []
    if "x-amazon-" in text:
        problems.append("contains an x-amazon- extension")
    if "arn:aws" in text:
        problems.append("contains an ARN")
    if config.origin_key in text:
        problems.append("contains the origin key")
    if config.account_id in text or _ACCOUNT_ID.search(text):
        problems.append("contains something that looks like an account ID")
    if "{{" in text:
        problems.append("contains an unfilled {{placeholder}}")
    schemes = doc.get("components", {}).get("securitySchemes", {})
    if any(s.get("type") == "apiKey" for s in schemes.values()):
        problems.append("has an apiKey security scheme")
    if not doc.get("info", {}).get("contact"):
        problems.append("has no info.contact")
    ops = {f"{m} {p}": op for m, p, op in operations(doc)}
    for name in bearer:
        if not any("bearer" in r for r in ops.get(name, {}).get("security", [])):
            problems.append(f"{name} doesn't declare Bearer")
    return problems


class _Dumper(yaml.SafeDumper):
    def ignore_aliases(self, data):
        return True


def _str(dumper, value: str):
    style = "|" if "\n" in value else None
    return dumper.represent_scalar("tag:yaml.org,2002:str", value, style=style)


_Dumper.add_representer(str, _str)


def dump(doc: dict) -> str:
    return yaml.dump(doc, Dumper=_Dumper, sort_keys=False, allow_unicode=True, width=1000)


def generate(config: sfab_config.Config) -> str:
    security = load_yaml(PUBLIC_SECURITY)
    doc = build_public(load_yaml(SOURCE), security, config.abuse_contact)
    text = dump(doc)
    problems = check_public(text, config, security["bearer"])
    if problems:
        raise GenerateError("the public API description " + "; ".join(problems))
    return text


def main() -> None:
    try:
        text = generate(sfab_config.load_for_check("--require-secret" in sys.argv[1:]))
    except (sfab_config.ConfigError, GenerateError) as e:
        sys.exit(f"generate_public_openapi: {e}")
    OUTPUT.parent.mkdir(exist_ok=True)
    OUTPUT.write_text(text, encoding="utf-8")
    print(f"wrote {OUTPUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
