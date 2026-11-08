#!/usr/bin/env python3
"""Generate the finished website pages from `website/` into build/website/.

Each page (`.html`) gets the API's host in place of `{{api_host}}`; every other file is
copied unchanged. The build folder is rebuilt from scratch each run, because the website
stack's `BucketDeployment` prunes the bucket to match it. See
.stories/2-identity-api/spec.md "Architecture" (website).
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

_PLACEHOLDER = re.compile(r"\{\{[^}]*\}\}")


class GenerateError(Exception):
    pass


def generate(source: Path, out: Path, *, api_host: str) -> None:
    if out.exists():
        shutil.rmtree(out)
    for path in sorted(source.rglob("*")):
        target = out / path.relative_to(source)
        if path.is_dir():
            target.mkdir(parents=True, exist_ok=True)
            continue
        target.parent.mkdir(parents=True, exist_ok=True)
        if path.suffix != ".html":
            shutil.copyfile(path, target)
            continue
        page = path.read_text(encoding="utf-8").replace("{{api_host}}", api_host)
        # A placeholder left over would show visitors the raw text.
        unfilled = sorted(set(_PLACEHOLDER.findall(page)))
        if unfilled:
            raise GenerateError(
                f"{path.relative_to(source)}: unfilled placeholder(s): {', '.join(unfilled)}"
            )
        target.write_text(page, encoding="utf-8")


def main() -> None:
    try:
        generate(SOURCE, OUTPUT, api_host=sfab_config.load().api_host)
    except (sfab_config.ConfigError, GenerateError) as e:
        sys.exit(f"generate_website: {e}")
    print(f"wrote {OUTPUT.relative_to(ROOT)}/")


if __name__ == "__main__":
    main()
