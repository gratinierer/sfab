#!/usr/bin/env python3
"""Generate the finished website pages from `website/` into build/website/.

Each text file (`.html`, `.txt`, `.xml` and the extensionless `.well-known/api-catalog`)
gets the API's host in place of `{{api_host}}`, the site's host in place of `{{site_host}}`
and the config's `abuse_contact` in place of `{{contact}}`, so no committed page holds an
address and the discovery files hold absolute URLs (.stories/10-website/spec.md "Contact",
"Placeholders"); every other file is copied unchanged. The build folder is rebuilt from
scratch each run, because the website
stack's `BucketDeployment` prunes the bucket to match it. See
.stories/2-identity-api/spec.md "Architecture" (website).

The config is scripts/check's (`sfab_config.load_for_check`): the secret, or the fixture
with a message, unless `--require-secret`, which scripts/deploy passes through.
"""

from __future__ import annotations

import re
import shutil
import sys
from pathlib import Path

import sfab_config

ROOT = sfab_config.REPO_ROOT
SOURCE = ROOT / "website"
OUTPUT = ROOT / "build" / "website"

# Files that get their placeholders filled. Anything else, such as a font or a committed
# `.well-known/sfab-federation/<token>`, is copied byte for byte.
_TEXT_SUFFIXES = {".html", ".txt", ".xml"}
_TEXT_NAMES = {"api-catalog"}

# The placeholder syntax, spelled once: `_marker` writes one, `_PLACEHOLDER` finds leftovers.
_OPEN, _CLOSE = "{{", "}}"
_PLACEHOLDER = re.compile(re.escape(_OPEN) + r".*?" + re.escape(_CLOSE))


def _marker(name: str) -> str:
    return _OPEN + name + _CLOSE


# The contact goes into a `mailto:` link, so it must be one plain address: no scheme, no
# space, nothing HTML or a URL would need escaped.
_ADDRESS = re.compile(r"[A-Za-z0-9.+_-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}")


class GenerateError(Exception):
    pass


def generate(source: Path, out: Path, *, api_host: str, site_host: str, contact: str) -> None:
    if not _ADDRESS.fullmatch(contact):
        raise GenerateError("abuse_contact must be one plain email address for the pages' mailto: link")
    # Every placeholder and its value, in one place: a new one is one more entry here.
    values = {"api_host": api_host, "site_host": site_host, "contact": contact}
    if out.exists():
        shutil.rmtree(out)
    for path in sorted(source.rglob("*")):
        target = out / path.relative_to(source)
        if path.is_dir():
            target.mkdir(parents=True, exist_ok=True)
            continue
        target.parent.mkdir(parents=True, exist_ok=True)
        if path.suffix not in _TEXT_SUFFIXES and path.name not in _TEXT_NAMES:
            shutil.copyfile(path, target)
            continue
        page = path.read_text(encoding="utf-8")
        for name, value in values.items():
            page = page.replace(_marker(name), value)
        # A placeholder left over would show visitors the raw text.
        unfilled = sorted(set(_PLACEHOLDER.findall(page)))
        if unfilled:
            raise GenerateError(
                f"{path.relative_to(source)}: unfilled placeholder(s): {', '.join(unfilled)}"
            )
        target.write_text(page, encoding="utf-8")


def main() -> None:
    try:
        config = sfab_config.load_for_check("--require-secret" in sys.argv[1:])
        generate(
            SOURCE,
            OUTPUT,
            api_host=config.api_host,
            site_host=config.domain_name,
            contact=config.abuse_contact,
        )
    except (sfab_config.ConfigError, GenerateError) as e:
        sys.exit(f"generate_website: {e}")
    print(f"wrote {OUTPUT.relative_to(ROOT)}/")


if __name__ == "__main__":
    main()
