"""A fake clock and transport for the scripts that call the deployed API, so their tests never
touch the network."""

from __future__ import annotations

import hashlib
import json
import re

import yaml

import sfab_config
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
# The deployed world's IP rule: one birth per viewer address in this many seconds, unless the
# genesis code skips it. Every request to the fake comes from one address.
IP_WINDOW = 4 * 3600
NEXT_DAILY_RUN = "2026-11-06T00:05:00Z"


PUBLISHABLE = ("name", "whoiam", "friends")
_CONTACT = re.compile(r"/v1/identity/self/(?P<list>friends|blocked)/(?P<id>[a-z2-7]{12})")
_ABOUTME = re.compile(r"/v1/identity/(?P<id>[a-z2-7]{12})/aboutme")
_SEND = re.compile(r"/v1/identity/(?P<id>[a-z2-7]{12})/messages")
_MESSAGE = re.compile(r"/v1/identity/self/messages/(?P<message_id>k-[a-z2-7]{12}|m-[0-9a-z-]+)")


def _routes() -> list[tuple[str, re.Pattern]]:
    """Every method and path in openapi/sfab.yaml, under /v1, so the fake knows the same routes
    as the API and a new route needs no second list here."""
    document = yaml.safe_load((sfab_config.REPO_ROOT / "openapi" / "sfab.yaml").read_text())
    return [
        (method.upper(), re.compile("/v1" + re.sub(r"\{[^}]+\}", "[^/]+", path)))
        for path, operations in document["paths"].items()
        for method in operations
        if method in ("get", "put", "post", "delete")
    ]


ROUTES = _routes()


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
        self.public: dict[str, set[str]] = {SMOKE_ID: set()}
        self.contacts: dict[str, dict[str, list[str]]] = {}
        # Each agent's knocks, by sender: only knocks, since the smoke script never friends itself.
        self.knocks: dict[str, dict[str, str]] = {}
        # Broken behaviours the smoke script must catch.
        self.keep_blocked_friends_public = False
        self.ignore_unpublish = False
        self.stale_friends: dict[str, str] = {}
        # Federation requests: domain -> token.
        self.federation: dict[str, str] = {}
        # Unused genesis codes, by kind: "exempt" (`--no-ip-limit`), "world" or "ordinary".
        self.genesis: dict[str, str] = {}
        self.issued = 0
        # Today's world pool; None until the daily job's first run.
        self.pool: list[str] | None = self.issue(3, "world")
        # The last birth from the one viewer address, under the IP rule.
        self.last_birth: int | None = None

    def issue(self, count: int, kind: str = "exempt") -> list[str]:
        """New genesis codes of this kind, 16 characters of lowercase base32."""
        codes = []
        for _ in range(count):
            self.issued += 1
            codes.append("".join("abcdefghij"[int(d)] for d in f"{self.issued:016d}"))
        self.genesis.update(dict.fromkeys(codes, kind))
        return codes

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
        path = r.url[len(API_HOST) :].partition("?")[0]
        if not path.startswith("/v1/"):
            return "outside v1", problem(404, detail=START)
        if not any(m == r.method and route.fullmatch(path) for m, route in ROUTES):
            return "unknown route", problem(404, detail=START)
        contact = _CONTACT.fullmatch(path)
        about = _ABOUTME.fullmatch(path)
        send = _SEND.fullmatch(path)
        message = _MESSAGE.fullmatch(path)
        if path == "/v1/openapi.yaml":
            return "openapi", Response(
                200,
                {"content-type": "application/yaml", "cache-control": "no-store"},
                b"openapi: 3.0.1\n",
            )
        if path == "/v1/world/constitution":
            constitution = {"version": "0123456789abcdef", "text": "...\n## Terms\n..."}
            return "constitution", ok(200, constitution)
        if path == "/v1/world/census":
            census = {"residents": len(self.claims), "as_of": "2026-11-05T00:05:00Z"}
            return "census", ok(200, census)
        if path == "/v1/world/residents":
            return "residents", ok(200, {"items": sorted(self.claims), "next": None})
        if path == "/v1/federation/world":
            return "federation world", ok(
                200,
                {
                    "domain": API_HOST.removeprefix("https://api."),
                    "name": "sfab",
                    "constitution": f"{API_HOST}/v1/world/constitution",
                    "api": f"{API_HOST}/v1/openapi.yaml",
                    "contact": "abuse@example.invalid",
                },
            )
        if path == "/v1/federation/worlds":
            return "federation worlds", ok(200, {"items": [], "next": None})
        if path == "/v1/world/genesis-code":
            if self.pool is None:
                return "genesis code", problem(404, "not-built-yet")
            if not self.pool:
                return "genesis code", problem(409, "try-later", retry_after=NEXT_DAILY_RUN)
            return "genesis code", ok(200, {"genesis_code": self.pool[0]})
        # The gateway's validator, with `additionalProperties: false`.
        if r.method in ("POST", "PUT") and path != "/v1/identity/reclaim" and not contact:
            try:
                body = json.loads(r.body)
            except ValueError:
                return "malformed body", problem(400, detail="Invalid request body")
            allowed = {"name", "whoiam"} if path.endswith("birth") else {"whoiam"}
            if path.endswith("birth"):
                allowed.add("genesis_code")
                code = body.get("genesis_code") if isinstance(body, dict) else None
                if not isinstance(code, str) or len(code) > 64 or "name" not in body:
                    return "invalid birth body", problem(400, detail="Invalid request body")
            if send:
                allowed = {"text"}
            if path.endswith("federation/requests"):
                allowed = {"domain"}
            if path.endswith("publication"):
                allowed = set(PUBLISHABLE)
                values = body.values() if isinstance(body, dict) else [None]
                if not all(isinstance(v, bool) for v in values):
                    return "extra field", problem(400, detail="Invalid request body")
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
        if about:
            return self._aboutme(about["id"])
        if path == "/v1/federation/requests":
            domain = json.loads(r.body)["domain"]
            if domain in self.federation:
                return "federation request", ok(200, self._token_reply(domain))
            self.federation[domain] = f"{len(self.federation):032x}"
            return "federation request", ok(202, self._token_reply(domain))
        if path == "/v1/identity/birth":
            # The code first, then the IP rule unless the code skips it (story 17 "What `birth`
            # does"), so a replay meets its used code.
            code = json.loads(r.body)["genesis_code"]
            kind = self.genesis.get(code)
            if kind is None:
                label = "birth replay" if digest in self.used else "invalid genesis code"
                return label, problem(403, "invalid-genesis-code")
            label = "world birth" if kind == "world" else "birth"
            if kind != "exempt" and self.last_birth is not None:
                if now - self.last_birth < IP_WINDOW:
                    return label, problem(409, "too-soon", retry_after="2026-11-05T04:00:00Z")
            self.used.add(digest)
            del self.genesis[code]
            if kind == "world":
                self.pool.remove(code)
            if kind != "exempt":
                self.last_birth = now
            new_id = agent_id(digest)
            claim, recovery = self._secret(), self._secret()
            self.claims[new_id] = (claim, recovery)
            self.identities[new_id] = json.loads(r.body)
            self.public[new_id] = set()
            return label, ok(
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
        if path == "/v1/identity/self/publication":
            if r.method == "PUT":
                wanted = {k for k, v in json.loads(r.body).items() if v}
                if not (self.ignore_unpublish and not wanted):
                    self.public[agent] = wanted
                return "publish", ok(200, self._settings(agent))
            return "read publication", ok(200, self._settings(agent))
        if path == "/v1/identity/self":
            return "self", ok(200, {"id": agent, **identity})
        if send:
            return self._send(agent, send["id"], json.loads(r.body)["text"], now)
        if path == "/v1/identity/self/messages":
            items = [
                {"message_id": f"k-{sender}", "sender": sender, "text": text,
                 "sent_at": "2026-11-05T00:00:00Z"}
                for sender, text in self.knocks.get(agent, {}).items()
            ]
            if "kind=knock" not in r.url:
                return "read messages", ok(200, {"items": [], "next": None})
            return "read knocks", ok(200, {"items": items, "next": None})
        if message:
            self.knocks.get(agent, {}).pop(message["message_id"].removeprefix("k-"), None)
            return "remove message", ok(200, {"message_id": message["message_id"]})
        lists = self.contacts.setdefault(agent, {"friends": [], "blocked": []})
        if contact:
            kind, other = contact["list"], contact["id"]
            entries = lists[kind]
            label = {
                ("friends", "PUT"): "add friend",
                ("friends", "DELETE"): "remove friend",
                ("blocked", "PUT"): "block",
                ("blocked", "DELETE"): "unblock",
            }[(kind, r.method)]
            if r.method == "DELETE":
                if other in entries:
                    entries.remove(other)
                return label, ok(200, {"id": other})
            if other in entries:
                return label, ok(200, {"id": other, "added_at": "2026-11-05T00:00:00Z"})
            entries.append(other)
            if kind == "blocked" and other in lists["friends"]:
                lists["friends"].remove(other)
                if self.keep_blocked_friends_public:
                    self.stale_friends[agent] = other
            return label, ok(201, {"id": other, "added_at": "2026-11-05T00:00:00Z"})
        kind = path.rsplit("/", 1)[1]
        items = [{"id": i, "added_at": "2026-11-05T00:00:00Z"} for i in lists[kind]]
        return f"read {kind}", ok(200, {"items": items, "next": None})

    def _send(self, sender: str, to: str, text: str, now: int):
        if to not in self.identities:
            return "message to an unknown agent", problem(404)
        if sender in self.contacts.get(to, {}).get("blocked", []):
            return "blocked message", problem(403, "blocked")
        self.knocks.setdefault(to, {})[sender] = text
        return "send message", ok(202, {"to": to, "kind": "knock", "sent_at": "2026-11-05T00:00:00Z"})


    def _token_reply(self, domain: str) -> dict:
        token = self.federation[domain]
        return {
            "domain": domain,
            "token": token,
            "url": f"https://{domain}/.well-known/sfab-federation/{token}",
            "expires_at": "2026-11-08T00:00:00Z",
            "instructions": "Publish it",
            "manual": "https://github.com/gratinierer/sfab/blob/main/docs/federation.md",
        }

    def _settings(self, agent: str) -> dict:
        return {f: f in self.public[agent] for f in PUBLISHABLE}

    def _aboutme(self, agent: str):
        public = self.public.get(agent)
        if not public:
            return "aboutme 404", problem(404)
        copy = {"id": agent}
        identity = self.identities[agent]
        if "name" in public:
            copy["name"] = identity["name"]
        if "whoiam" in public and "whoiam" in identity:
            copy["whoiam"] = identity["whoiam"]
        if "friends" in public:
            friends = list(self.contacts.get(agent, {}).get("friends", []))
            stale = self.stale_friends.get(agent)
            if stale:
                friends.append(stale)
            copy["friends"] = friends
        return "aboutme", ok(200, copy)
