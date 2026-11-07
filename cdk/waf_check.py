"""The WAF check, run by hand against the deployed API: `.stories/2-identity-api/spec.md` "Testing".

Sends about one `birth` with no proof per second, for up to 5 minutes, and passes at the first
plain `403` that isn't `problem+json`. AWS says WAF can take several minutes to react, so it fails
only if no block has arrived by then. It gets this machine's IP blocked for a while, so it stays
out of the smoke script: don't run the smoke script just after it.

    scripts/waf-check [--api-url https://api.<domain_name>]

Needs no AWS credentials. Without `--api-url`, it reads `domain_name` from the config.
"""

from __future__ import annotations

import argparse
import sys
import time
from typing import Callable

import sfab_config
from api_client import Request, Transport, is_waf_block, urllib_transport

WINDOW_SECONDS = 300
INTERVAL_SECONDS = 1
BODY = b'{"name":"waf check"}'


def run(
    transport: Transport,
    api_url: str,
    *,
    clock: Callable[[], float],
    sleep: Callable[[float], None],
    log: Callable[[str], None],
) -> bool:
    """True at the first plain `403`, False when the window ends without one."""
    deadline = clock() + WINDOW_SECONDS
    request = Request(
        "POST",
        f"{api_url}/v1/identity/birth",
        {"Content-Type": "application/json"},
        BODY,
    )
    sent = 0
    while True:
        response = transport(request)
        sent += 1
        if is_waf_block(response):
            log(f"pass: request {sent} got WAF's plain 403")
            return True
        if sent % 10 == 1:
            log(f"request {sent}: {response.status}, not blocked yet")
        if clock() + INTERVAL_SECONDS > deadline:
            log(f"FAIL: {sent} requests in {WINDOW_SECONDS} seconds and no plain 403")
            return False
        sleep(INTERVAL_SECONDS)


def main(argv: list[str] | None = None, transport: Transport = urllib_transport) -> int:
    parser = argparse.ArgumentParser(prog="waf-check", description=__doc__.split("\n\n")[1])
    parser.add_argument("--api-url", help="defaults to https://api.<domain_name> from the config")
    args = parser.parse_args(argv)
    api_url = args.api_url or _api_url_from_config()
    if api_url is None:
        return 2
    passed = run(
        transport, api_url.rstrip("/"), clock=time.time, sleep=time.sleep, log=print
    )
    return 0 if passed else 1


def _api_url_from_config() -> str | None:
    try:
        return sfab_config.load().api_url
    except sfab_config.ConfigError as e:
        print(e, file=sys.stderr)
        return None


if __name__ == "__main__":
    sys.exit(main())
