"""The client side of the deployed API, shared by the smoke script and the WAF check:
`.stories/2-identity-api/spec.md` "Testing".

Standard library only, and no AWS credentials: both scripts call the API the way an agent does.
The proof follows spec.md "Proof of work", solved the way the API description tells agents to,
counting nonces up from `0`, so its worked example is a test vector here too.
"""

from __future__ import annotations

import hashlib
import json
import urllib.error
import urllib.request
from dataclasses import dataclass, field
from typing import Callable

_BASE32 = "abcdefghijklmnopqrstuvwxyz234567"
TIMEOUT_SECONDS = 30


@dataclass(frozen=True)
class Request:
    method: str
    url: str
    headers: dict[str, str] = field(default_factory=dict)
    body: bytes = b""


@dataclass(frozen=True)
class Response:
    status: int
    # Names in lower case.
    headers: dict[str, str]
    body: bytes

    def json(self):
        """The body as JSON, or None when it isn't."""
        try:
            return json.loads(self.body)
        except ValueError:
            return None


# Sends one request and returns whatever came back, error statuses included.
Transport = Callable[[Request], Response]


def leading_zero_bits(digest: bytes) -> int:
    bits = 0
    for byte in digest:
        if byte:
            return bits + 8 - byte.bit_length()
        bits += 8
    return bits


def solve_proof(
    method: str, path: str, body: bytes, *, caller: str, now: int, bits: int
) -> tuple[str, bytes]:
    """The `Proof` header for this request, and its solution hash."""
    prefix = f"sfab:{method}:{path}:{now}:{hashlib.sha256(body).hexdigest()}:{caller}:".encode()
    nonce = 0
    while True:
        digest = hashlib.sha256(prefix + str(nonce).encode()).digest()
        if leading_zero_bits(digest) >= bits:
            return f"{now}:{nonce}", digest
        nonce += 1


def agent_id(digest: bytes) -> str:
    """The ID a `birth` with this solution hash gets: its last 60 bits in lowercase base32."""
    tail = int.from_bytes(digest[24:], "big")
    return "".join(_BASE32[(tail >> (5 * (11 - i))) & 31] for i in range(12))


def media_type(response: Response) -> str:
    return response.headers.get("content-type", "").split(";")[0].strip().lower()


def is_problem_json(response: Response) -> bool:
    """RFC 9457: the media type, and a JSON object with a string `type`."""
    body = response.json()
    return (
        media_type(response) == "application/problem+json"
        and isinstance(body, dict)
        and isinstance(body.get("type"), str)
    )


def is_waf_block(response: Response) -> bool:
    """WAF's plain `403`, the rate limit or a stop: every `403` of ours is `problem+json`."""
    return response.status == 403 and not is_problem_json(response)


class _NoRedirects(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):
        return None


_opener = urllib.request.build_opener(_NoRedirects)


def urllib_transport(request: Request) -> Response:
    """The real network. Not unit-tested: the scripts' tests use fakes."""
    raw = urllib.request.Request(
        request.url,
        data=request.body if request.method in ("POST", "PUT") else None,
        headers=request.headers,
        method=request.method,
    )
    try:
        with _opener.open(raw, timeout=TIMEOUT_SECONDS) as reply:
            return Response(reply.status, _lower(reply.headers.items()), reply.read())
    except urllib.error.HTTPError as reply:
        return Response(reply.code, _lower(reply.headers.items()), reply.read())


def _lower(items) -> dict[str, str]:
    return {name.lower(): value for name, value in items}
