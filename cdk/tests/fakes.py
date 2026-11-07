"""A fake clock and transport for the scripts that call the deployed API, so their tests never
touch the network."""

from __future__ import annotations

import hashlib
import json

from api_client import Request, Response, agent_id, leading_zero_bits


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


def problem(status: int, kind: str = "about:blank", title: str = "x", **extra) -> Response:
    body = {"type": kind, "title": title, "status": status, **extra}
    headers = {"content-type": "application/problem+json", "cache-control": "no-store"}
    return Response(status, headers, json.dumps(body).encode())


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


API_HOST = "https://api.example.invalid"
EXECUTE_API = "https://abc123.execute-api.us-east-1.amazonaws.com/v1/"
START = "Start at https://api.example.invalid/v1/openapi.yaml"
SMOKE_ID = "smokesmokesm"
SMOKE_CREDENTIAL = f"{SMOKE_ID}.{'c' * 43}"


class FakeApi:
    """Stage 1 as deployed, behind CloudFront and API Gateway: just enough of each layer for the
    smoke script to tell a working API from a broken one. A test breaks one behaviour at a time
    through `override`, a map from a label, the first item `_answer` returns, to the response to
    give instead."""

    def __init__(self, clock: Clock, bits: int = 8):
        self.clock = clock
        self.bits = bits
        self.claims: dict[str, tuple[str, str]] = {SMOKE_ID: ("c" * 43, "r" * 43)}
        self.identities: dict[str, dict] = {SMOKE_ID: {"name": "smoke"}}
        self.used: set[bytes] = set()
        self.counter = 0
        self.override: dict[str, Response] = {}
        self.requests: list[Request] = []
        self.labels: list[str] = []

    def __call__(self, request: Request) -> Response:
        self.requests.append(request)
        label, response = self._answer(request)
        self.labels.append(label)
        return self.override.get(label, response)

    def _secret(self) -> str:
        self.counter += 1
        return f"{self.counter:043d}"

    def _answer(self, r: Request):
        headers = {k.lower(): v for k, v in r.headers.items()}
        if r.url.startswith(EXECUTE_API):
            if headers.get("x-api-key") is None:
                return "execute-api no key", problem(403, "origin-key-replaced")
            return "execute-api wrong key", problem(403, "origin-key-replaced")
        assert r.url.startswith(API_HOST), r.url
        path = r.url[len(API_HOST) :]
        if not path.startswith("/v1/"):
            return "outside v1", problem(404, detail=START)
        known = {
            ("GET", "/v1/openapi.yaml"),
            ("POST", "/v1/identity/birth"),
            ("PUT", "/v1/identity/evolve"),
            ("GET", "/v1/identity/self"),
            ("POST", "/v1/identity/reclaim"),
        }
        if (r.method, path) not in known:
            return "unknown route", problem(404, detail=START)
        if path == "/v1/openapi.yaml":
            return "openapi", Response(
                200,
                {"content-type": "application/yaml", "cache-control": "no-store"},
                b"openapi: 3.0.1\n",
            )
        # The gateway's validator, with `additionalProperties: false`.
        if r.method in ("POST", "PUT") and path != "/v1/identity/reclaim":
            try:
                body = json.loads(r.body)
            except ValueError:
                return "malformed body", problem(400, detail="Invalid request body")
            allowed = {"name", "whoiam"} if path.endswith("birth") else {"whoiam"}
            if not isinstance(body, dict) or set(body) - allowed:
                return "extra field", problem(400, detail="Invalid request body")
        # The Lambda: the proof, then the body limit, then the route.
        now = int(self.clock())
        auth = headers.get("authorization")
        bearer = auth.removeprefix("Bearer ").split(".", 1) if auth else None
        digest = None
        if r.method in ("POST", "PUT"):
            proof = headers.get("proof")
            if proof is None:
                return "no proof", problem(400, required_bits=self.bits, server_time=now)
            ts, nonce = proof.split(":")
            caller = bearer[0] if bearer else "-"
            body_hash = hashlib.sha256(r.body).hexdigest()
            text = f"sfab:{r.method}:{path}:{ts}:{body_hash}:{caller}:{nonce}"
            digest = hashlib.sha256(text.encode()).digest()
            if leading_zero_bits(digest) < self.bits or not -300 <= int(ts) - now <= 60:
                return "bad proof", problem(400, required_bits=self.bits, server_time=now)
            if len(r.body) > 32 * 1024:
                return "body over 32 KB", problem(
                    400, title="The request body is over 32 KB (32,768 bytes)"
                )
        if path == "/v1/identity/birth":
            if digest in self.used:
                return "birth replay", problem(409, "already-exists")
            self.used.add(digest)
            new_id = agent_id(digest)
            claim, recovery = self._secret(), self._secret()
            self.claims[new_id] = (claim, recovery)
            self.identities[new_id] = json.loads(r.body)
            return "birth", ok(
                201,
                {
                    "id": new_id,
                    "claim": claim,
                    "recovery": recovery,
                    "notice": "shown once",
                    "constitution": "/v1/world/constitution",
                },
            )
        if not bearer or len(bearer) != 2 or bearer[0] not in self.claims:
            return "unauthorized", problem(401)
        agent, secret = bearer
        claim, recovery = self.claims[agent]
        if path == "/v1/identity/reclaim":
            if secret != recovery:
                return "unauthorized", problem(401)
            self.claims[agent] = (self._secret(), self._secret())
            claim, recovery = self.claims[agent]
            return "reclaim", ok(
                200, {"id": agent, "claim": claim, "recovery": recovery, "notice": "once"}
            )
        if secret != claim:
            return "unauthorized", problem(401)
        identity = self.identities[agent]
        if path == "/v1/identity/evolve":
            identity["whoiam"] = json.loads(r.body)["whoiam"]
            return "evolve", ok(200, {"id": agent, **identity})
        return "self", ok(200, {"id": agent, **identity})
