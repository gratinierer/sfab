"""A fake clock and transport for the scripts that call the deployed API, so their tests never
touch the network."""

from __future__ import annotations

import json

from api_client import Request, Response


class Clock:
    """Time moves only when the script sleeps, or when a test moves it."""

    def __init__(self, now: float = 1_793_836_800.0):
        self.now = now
        self.sleeps: list[float] = []

    def __call__(self) -> float:
        return self.now

    def sleep(self, seconds: float) -> None:
        self.sleeps.append(seconds)
        self.now += seconds


def problem(status: int, kind: str = "about:blank", **extra) -> Response:
    body = {"type": kind, "title": "x", "status": status, **extra}
    return Response(status, {"content-type": "application/problem+json"}, json.dumps(body).encode())


def plain(status: int, body: bytes = b"<html>403 Forbidden</html>") -> Response:
    return Response(status, {"content-type": "text/html"}, body)


def ok(status: int, body: dict) -> Response:
    return Response(
        status,
        {"content-type": "application/json", "cache-control": "no-store"},
        json.dumps(body).encode(),
    )


class Recorder:
    """Records each request and answers with `answer(request)`."""

    def __init__(self, answer):
        self.answer = answer
        self.requests: list[Request] = []

    def __call__(self, request: Request) -> Response:
        self.requests.append(request)
        return self.answer(request)
