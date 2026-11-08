"""The smoke script, run by hand against the deployed API: `.stories/2-identity-api/spec.md`
"Testing". Each later ticket adds its routes here.

    scripts/smoke --execute-api <stage URL> [--production] [--api-url https://api.<domain_name>]

The stage URL is the API stack's `Endpoint` output that `cdk deploy` prints, such as
`https://abc123.execute-api.us-east-1.amazonaws.com/v1/`. Needs no AWS credentials.

- Before launch, the full run births a test agent and covers every route, the gateway's
  validation and errors, both `404`s, and the direct `execute-api` way in. It births with the
  owner's `--no-ip-limit` genesis codes from `smoke-genesis-codes.txt` at the repo root
  (gitignored), one per line, and drops each code it spends, so reruns from one IP pass. The
  owner tops the file up, with the owner's AWS credentials, from `lambda/`:

      IDENTITIES_BUCKET=<name> CLAIMS_BUCKET=<name> cargo run -q -p sfab-genesis-codes -- \\
        issue --label smoke --count <n> --no-ip-limit >> ../smoke-genesis-codes.txt

  It also births once with a code from `GET /world/genesis-code`, when the pool has one.
- After launch, `--production` births nothing and writes nothing public: it reads every read
  route and makes only private writes, as the fixed smoke identity, whose `<id>.<claim>` is
  `smoke_claim` in `sfab.config.toml`.

It sends one request every 4 seconds, to stay under WAF's 100 per 5 minutes, and solves a
`Proof` for each write. WAF's plain `403` means this IP is blocked, or the API is stopped:
it stops and says to wait and rerun, rather than failing.
"""

from __future__ import annotations

import argparse
import re
import secrets
import string
import sys
import time
import tomllib
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable

import sfab_config
from api_client import (
    Request,
    Response,
    Transport,
    agent_id,
    is_problem_json,
    is_waf_block,
    solve_proof,
    urllib_transport,
)

PACE_SECONDS = 4
# Until the API's first answer says otherwise: spec.md "Proof of work".
DEFAULT_BITS = 20
MAX_WHOIAM_CHARS = 32768
SMOKE_NAME = "smoke"
SMOKE_WHOIAM = "The sfab smoke identity. It writes nothing public."
# Friended, blocked and removed again; IDs aren't checked for existence.
SMOKE_FRIEND = "smokefriends"
# An ID nobody is likely to have, for `aboutme`'s 404.
UNKNOWN_ID = "aaaaaaaaaaaa"
START = "/v1/openapi.yaml"
# The owner's `--no-ip-limit` genesis codes for the full run's births, gitignored.
CODES_FILE = sfab_config.REPO_ROOT / "smoke-genesis-codes.txt"
TOP_UP = (
    "the owner tops it up from lambda/ with IDENTITIES_BUCKET=<name> CLAIMS_BUCKET=<name> "
    "cargo run -q -p sfab-genesis-codes -- issue --label smoke --count <n> --no-ip-limit "
    ">> ../smoke-genesis-codes.txt"
)
# Well-formed, so the gateway's validator passes the proof-less `birth` on to the Lambda.
DUMMY_GENESIS_CODE = "a" * 16
_SECRET = re.compile(r"[A-Za-z0-9_-]{43}")
_GENESIS_CODE = re.compile(r"[a-z2-7]{16}")
_ID = re.compile(r"[a-z2-7]{12}")
_TOKEN = re.compile(r"[0-9a-f]{32}")
# The federation request's domain, under this world's own, so nobody else can ever own it. It
# has no host, so it's never verified, and it expires after 3 days (spec.md "Testing").
SMOKE_FEDERATION_LABEL = "smoke-federation"
BLOCKED_MESSAGE = (
    "Blocked: WAF's plain 403, from the rate limit on this IP or because the API is "
    "stopped. Wait at least 5 minutes and rerun. A stop lasts until the owner restarts the API."
)


class Codes:
    """The `--no-ip-limit` genesis codes the full run births with, one per birth. A code is
    spent, and dropped, once a birth with it gets past the code check."""

    def __init__(self, codes: list[str]):
        self.codes = list(codes)

    def next(self) -> str | None:
        return self.codes[0] if self.codes else None

    def spend(self, code: str) -> None:
        self.codes.remove(code)
        self.save()

    def save(self) -> None:
        pass


class CodeFile(Codes):
    """`smoke-genesis-codes.txt`, one code per line, rewritten as codes are spent."""

    def __init__(self, path: Path):
        self.path = path
        try:
            text = path.read_text(encoding="utf-8")
        except FileNotFoundError:
            text = ""
        super().__init__([line.strip() for line in text.splitlines() if line.strip()])

    def save(self) -> None:
        self.path.write_text("".join(f"{c}\n" for c in self.codes), encoding="utf-8")


@dataclass(frozen=True)
class Target:
    api_url: str
    execute_api_url: str
    # `<id>.<claim>` of the fixed smoke identity, for `--production`.
    credential: str | None = None


@dataclass
class Result:
    failures: list[str] = field(default_factory=list)
    blocked: bool = False


class Blocked(Exception):
    pass


class _Session:
    def __init__(self, transport, target, clock, sleep, log):
        self.transport, self.target = transport, target
        self.clock, self.sleep, self.log = clock, sleep, log
        self.sent = 0
        self.bits = DEFAULT_BITS
        self.clock_offset = 0
        self.failures: list[str] = []
        # The test agent's recovery code, from its birth.
        self.recovery: str | None = None
        # A code from `GET /world/genesis-code`, when the pool had one.
        self.world_code: str | None = None

    def learn(self, refusal: dict) -> None:
        """The proof's difficulty and the server's clock, from a refused `birth`."""
        self.bits = refusal["required_bits"]
        self.clock_offset = refusal["server_time"] - int(self.clock())
        if abs(self.clock_offset) > 30:
            self.log(f"note: this clock is {self.clock_offset}s off the server's; using the server's")

    def send(self, method, url, headers=None, body=b"") -> Response:
        if self.sent:
            self.sleep(PACE_SECONDS)
        self.sent += 1
        response = self.transport(Request(method, url, dict(headers or {}), body))
        if url.startswith(self.target.api_url) and is_waf_block(response):
            raise Blocked()
        return response

    def api(self, method, path, headers=None, body=b"") -> Response:
        return self.send(method, self.target.api_url + path, headers, body)

    def write(self, method, path, body, *, credential=None) -> tuple[Response, str, bytes]:
        """A `POST` or `PUT` with a solved proof; returns the response, the header and the hash."""
        caller = agent_of(credential) if credential else "-"
        now = int(self.clock()) + self.clock_offset
        proof, digest = solve_proof(method, path, body, caller=caller, now=now, bits=self.bits)
        return self.api(method, path, _headers(credential, proof, body), body), proof, digest

    def check(
        self,
        name: str,
        response: Response,
        status: int | tuple[int, ...],
        *,
        problem: bool | str = False,
        api: bool = True,
        verify: Callable[[Response], str | None] | None = None,
    ) -> bool:
        """Records a failure unless the response has this status and every expectation holds.
        `problem` is True, or the `type` the `problem+json` body must have. `api` is False for
        a response that doesn't come from the API, so needn't send `Cache-Control: no-store`.
        `verify` is a function of the response that returns what's wrong, or None."""
        wrong = []
        if response.status not in (status if isinstance(status, tuple) else (status,)):
            wrong.append(f"status {response.status}, not {status}")
        if problem and not is_problem_json(response):
            wrong.append("not problem+json")
        elif isinstance(problem, str) and response.json().get("type") != problem:
            wrong.append(f"type {response.json().get('type')!r}, not {problem!r}")
        if api and response.headers.get("cache-control") != "no-store":
            wrong.append("no Cache-Control: no-store")
        if not wrong and verify:
            problem_text = verify(response)
            if problem_text:
                wrong.append(problem_text)
        if wrong:
            body = response.body[:300].decode("utf-8", "replace")
            self.failures.append(f"{name}: {'; '.join(wrong)}. Body: {body}")
            self.log(f"FAIL {name}")
            return False
        self.log(f"ok   {name}")
        return True


def agent_of(credential: str) -> str:
    """The agent ID in an `<id>.<secret>` credential, as `Authorization` carries it."""
    return credential.split(".", 1)[0]


def _headers(credential, proof=None, body=b"") -> dict[str, str]:
    headers = {}
    if credential:
        headers["Authorization"] = f"Bearer {credential}"
    if proof:
        headers["Proof"] = proof
    if body:
        headers["Content-Type"] = "application/json"
    return headers


def _json_object(response: Response) -> dict:
    """The body as a JSON object, or `{}` for anything else, so checks can use `.get`."""
    body = response.json()
    return body if isinstance(body, dict) else {}


def _points_to_start(response: Response) -> str | None:
    detail = _json_object(response).get("detail", "")
    return None if START in detail else f"detail {detail!r} doesn't point to {START}"


def _no_required_bits(response: Response) -> str | None:
    if "required_bits" in _json_object(response):
        return "required_bits present, so the gateway's validator let it through"
    return None


def run(
    transport: Transport,
    target: Target,
    *,
    production: bool,
    clock: Callable[[], float],
    sleep: Callable[[float], None],
    log: Callable[[str], None],
    codes: Codes,
) -> Result:
    """`codes` are the full run's `--no-ip-limit` genesis codes; `--production` births nothing."""
    s = _Session(transport, target, clock, sleep, log)
    try:
        _public(s)
        if production:
            _identity(s, target.credential, birthed=False)
            _profile(s, target.credential, public=False)
            _messages(s, target.credential)
        else:
            credential = _birth(s, codes)
            if credential:
                credential = _identity(s, credential, birthed=True)
            if credential:
                _profile(s, credential, public=True)
                _messages(s, credential)
            _world_birth(s)
            _request_federation(s)
        _execute_api(s)
    except Blocked:
        log(BLOCKED_MESSAGE)
        return Result(s.failures, blocked=True)
    return Result(s.failures)


def _public(s: _Session) -> None:
    """Anonymous reads, both `404`s, and a `birth` with no proof, which writes nothing."""
    s.check(
        "GET /v1/openapi.yaml",
        s.api("GET", "/v1/openapi.yaml"),
        200,
        verify=lambda r: None if b"openapi:" in r.body else "not an OpenAPI document",
    )
    _world(s)
    _federation_directory(s)
    s.check(
        "outside /v1/: the CloudFront Function's 404",
        s.api("GET", "/"),
        404,
        problem=True,
        verify=_points_to_start,
    )
    for name, method, path in [
        ("bare /v1/", "GET", "/v1/"),
        ("wrong method", "DELETE", "/v1/identity/birth"),
    ]:
        s.check(
            f"{name}: the gateway's 404",
            s.api(method, path),
            404,
            problem=True,
            verify=_points_to_start,
        )

    body = _birth_body(DUMMY_GENESIS_CODE)
    response = s.api("POST", "/v1/identity/birth", _headers(None, body=body), body)

    def has_bits_and_time(r: Response) -> str | None:
        found = _json_object(r)
        if not isinstance(found.get("required_bits"), int):
            return "no integer required_bits"
        if not isinstance(found.get("server_time"), int):
            return "no integer server_time"
        return None

    if s.check("birth without a proof", response, 400, problem=True, verify=has_bits_and_time):
        s.learn(_json_object(response))


def _world(s: _Session) -> None:
    """The world routes, which write nothing. Before the daily job's first run, the census and
    resident list answer `404` with `not-built-yet`, which passes. A page past the last is left
    to the unit tests, to keep the run near 43 requests."""

    def constitution(r: Response) -> str | None:
        body = _json_object(r)
        if not isinstance(body.get("version"), str) or not body["version"]:
            return "no version"
        if "## Terms" not in str(body.get("text", "")):
            return "the text has no Terms"
        return None

    s.check(
        "GET /v1/world/constitution",
        s.api("GET", "/v1/world/constitution"),
        200,
        verify=constitution,
    )

    def census(r: Response) -> str | None:
        body = _json_object(r)
        if not isinstance(body.get("residents"), int) or not isinstance(body.get("as_of"), str):
            return "no integer residents or as_of"
        return None

    def residents(r: Response) -> str | None:
        body = _json_object(r)
        if not isinstance(body.get("items"), list) or "next" not in body:
            return "not a list with items and next"
        return None

    for path, check in (("/v1/world/census", census), ("/v1/world/residents", residents)):
        response = s.api("GET", path)
        if response.status == 404:
            s.check(f"GET {path}", response, 404, problem="not-built-yet")
        else:
            s.check(f"GET {path}", response, 200, verify=check)
    _genesis_code(s)


def _genesis_code(s: _Session) -> None:
    """One of today's world codes, kept for the full run's world-code birth. Before the daily
    job's first pool it's `404 not-built-yet`, and with today's pool empty or not yet built it's
    `409 try-later` with `retry_after`; both pass, and leave no code to birth with."""
    path = "/v1/world/genesis-code"
    response = s.api("GET", path)
    if response.status == 404:
        s.check(f"GET {path}", response, 404, problem="not-built-yet")
        return
    if response.status == 409:

        def retry_after(r: Response) -> str | None:
            return None if _json_object(r).get("retry_after") else "no retry_after"

        s.check(f"GET {path}", response, 409, problem="try-later", verify=retry_after)
        return

    def code(r: Response) -> str | None:
        found = str(_json_object(r).get("genesis_code", ""))
        return None if _GENESIS_CODE.fullmatch(found) else f"genesis_code {found!r}"

    if s.check(f"GET {path}", response, 200, verify=code):
        s.world_code = _json_object(response)["genesis_code"]


def _world_domain(s: _Session) -> str:
    return s.target.api_url.removeprefix("https://").removeprefix("api.")


def _federation_directory(s: _Session) -> None:
    """This world's directory entry and the approved worlds, which write nothing."""
    domain = _world_domain(s)

    def world(r: Response) -> str | None:
        body = _json_object(r)
        if body.get("domain") != domain:
            return f"domain {body.get('domain')!r}, not {domain!r}"
        missing = [k for k in ("name", "constitution", "contact") if not body.get(k)]
        return f"no {', '.join(missing)}" if missing else None

    s.check("GET /v1/federation/world", s.api("GET", "/v1/federation/world"), 200, verify=world)

    def worlds(r: Response) -> str | None:
        body = _json_object(r)
        if not isinstance(body.get("items"), list) or "next" not in body:
            return "not a list with items and next"
        return None

    s.check("GET /v1/federation/worlds", s.api("GET", "/v1/federation/worlds"), 200, verify=worlds)


def _request_federation(s: _Session) -> None:
    """A federation request for a subdomain of this world. A rerun gets the same token, with
    `200`. Only before launch: it holds one of the 100 places for 3 days."""
    domain = f"{SMOKE_FEDERATION_LABEL}.{_world_domain(s)}"
    body = f'{{"domain":"{domain}"}}'.encode()
    response, _, _ = s.write("POST", "/v1/federation/requests", body)

    def token(r: Response) -> str | None:
        found = _json_object(r)
        if not _TOKEN.fullmatch(str(found.get("token", ""))):
            return "no token of 32 hex characters"
        if found.get("domain") != domain:
            return f"domain {found.get('domain')!r}, not {domain!r}"
        return None

    s.check("federation request", response, (200, 202), verify=token)


def _birth_body(genesis_code: str) -> bytes:
    return f'{{"name":"{SMOKE_NAME}","genesis_code":"{genesis_code}"}}'.encode()


def _born(digest: bytes):
    """A check that a `birth` answer holds the ID from the proof's hash and both secrets."""

    def check(r: Response) -> str | None:
        born = _json_object(r)
        if born.get("id") != agent_id(digest):
            return f"id {born.get('id')!r}, not {agent_id(digest)!r} from the proof's hash"
        if not all(_SECRET.fullmatch(str(born.get(k, ""))) for k in ("claim", "recovery")):
            return "claim or recovery isn't 43 characters of base64url"
        return None

    return check


def _birth(s: _Session, codes: Codes) -> str | None:
    """A test agent, with a `--no-ip-limit` code so reruns from one IP pass, and a replay of
    its proof, which meets the used code."""
    code = codes.next()
    if code is None:
        s.failures.append(
            f"birth: no --no-ip-limit genesis code left in {CODES_FILE.name}; {TOP_UP}"
        )
        s.log(f"FAIL birth: {CODES_FILE.name} has no codes left")
        return None
    body = _birth_body(code)
    response, proof, digest = s.write("POST", "/v1/identity/birth", body)
    if response.status == 201 or (
        response.status == 403 and _json_object(response).get("type") == "invalid-genesis-code"
    ):
        codes.spend(code)
    if not s.check("birth", response, 201, verify=_born(digest)):
        return None
    born = _json_object(response)
    s.log(f"     born test agent {born['id']}")
    replay = s.api("POST", "/v1/identity/birth", _headers(None, proof, body), body)
    s.check("birth replay", replay, 403, problem="invalid-genesis-code")
    s.recovery = born["recovery"]
    return f"{born['id']}.{born['claim']}"


def _world_birth(s: _Session) -> None:
    """A birth with the world's code. The code is checked before the IP rule, so `201` or
    `409 too-soon` both prove it was accepted, and a rerun from one IP within the window still
    passes; only `201` uses up the pool code. The pool is only mostly fresh, so after a `403
    invalid-genesis-code` it fetches a fresh code and births once more, which must be accepted:
    a racing birth may use one code, but a world that rejects two is broken. A refetch that
    finds no code skips the retry, as a first fetch would, and so does one that hands back the
    same used code, since the pool picks at random and keeps a code a race used."""
    if s.world_code is None:
        s.log("skip birth with the world's code: GET /v1/world/genesis-code gave none")
        return
    response, _, digest = s.write("POST", "/v1/identity/birth", _birth_body(s.world_code))
    name = "birth with the world's code"
    if response.status == 403:
        if not s.check(name, response, 403, problem="invalid-genesis-code"):
            return
        s.log(
            "note: the world's code was already used, which a racing birth may do, "
            "since the pool is only mostly fresh; trying a fresh one"
        )
        used, s.world_code = s.world_code, None
        _genesis_code(s)
        if s.world_code is None:
            s.log("skip birth with a fresh world code: GET /v1/world/genesis-code gave none")
            return
        if s.world_code == used:
            s.log(
                "skip birth with a fresh world code: GET /v1/world/genesis-code gave the same "
                "used code again, which the pool's random pick may do while it holds it"
            )
            return
        response, _, digest = s.write("POST", "/v1/identity/birth", _birth_body(s.world_code))
        name = "birth with a fresh world code"
    if response.status == 409:

        def retry_after(r: Response) -> str | None:
            return None if _json_object(r).get("retry_after") else "no retry_after"

        s.check(name, response, 409, problem="too-soon", verify=retry_after)
    else:
        s.check(name, response, 201, verify=_born(digest))


def _identity(s: _Session, credential: str, *, birthed: bool) -> str | None:
    """`self`, `evolve` and its gateway validation, wrong credentials and, for a test agent,
    `reclaim`. Returns the credential that works afterwards, if any."""
    agent = agent_of(credential)
    wrong = f"{agent}.{'A' * 43}"

    def is_agent(r: Response) -> str | None:
        found = _json_object(r).get("id")
        return None if found == agent else f"id {found!r}, not {agent!r}"

    s.check(
        "self, so Authorization reaches the origin",
        s.api("GET", "/v1/identity/self", _headers(credential)),
        200,
        verify=is_agent,
    )
    s.check(
        "self with a wrong claim",
        s.api("GET", "/v1/identity/self", _headers(wrong)),
        401,
        problem=True,
    )

    body = f'{{"whoiam":"{SMOKE_WHOIAM}"}}'.encode()
    response, _, _ = s.write("PUT", "/v1/identity/evolve", body, credential=credential)
    s.check(
        "evolve",
        response,
        200,
        verify=lambda r: None if _json_object(r).get("whoiam") == SMOKE_WHOIAM else "whoiam not replaced",
    )

    extra = f'{{"whoiam":"{SMOKE_WHOIAM}","extra":1}}'.encode()
    response, _, _ = s.write("PUT", "/v1/identity/evolve", extra, credential=credential)
    s.check(
        "evolve with an extra field: the gateway's 400",
        response,
        400,
        problem=True,
        verify=_no_required_bits,
    )

    # Every character escaped: within the gateway's maxLength, which counts characters, but
    # about 192 KB, so only the Lambda's 32 KB check can refuse it (spec.md "Validator limit").
    escaped = ('{"whoiam":"' + "\\u00e9" * MAX_WHOIAM_CHARS + '"}').encode()
    response, _, _ = s.write("PUT", "/v1/identity/evolve", escaped, credential=credential)
    s.check(
        "evolve with a maximum-size escaped body: the Lambda's 32 KB check",
        response,
        400,
        problem=True,
        verify=lambda r: None
        if "32 KB" in _json_object(r).get("title", "")
        else "not the Lambda's 32 KB refusal",
    )

    malformed = b'{"whoiam":'
    response, _, _ = s.write("PUT", "/v1/identity/evolve", malformed, credential=credential)
    s.check("evolve with a malformed body: valid problem+json", response, 400, problem=True)

    if birthed:
        return _reclaim(s, credential)
    return credential


def _reclaim(s: _Session, old_credential: str) -> str | None:
    agent = agent_of(old_credential)
    old_recovery = f"{agent}.{s.recovery}"
    response, _, _ = s.write("POST", "/v1/identity/reclaim", b"", credential=old_recovery)
    new = _json_object(response)

    def fresh(r: Response) -> str | None:
        if new.get("id") != agent:
            return f"id {new.get('id')!r}, not {agent!r}"
        if f"{agent}.{new.get('claim')}" == old_credential or new.get("recovery") == s.recovery:
            return "the secrets didn't change"
        return None

    if not s.check("reclaim", response, 200, verify=fresh):
        return None
    s.check(
        "self with the old claim after reclaim",
        s.api("GET", "/v1/identity/self", _headers(old_credential)),
        401,
        problem=True,
    )
    response, _, _ = s.write("POST", "/v1/identity/reclaim", b"", credential=old_recovery)
    s.check("reclaim with the spent recovery code", response, 401, problem=True)
    # The next request, reading the publication, uses the new claim, so it checks it too.
    return f"{agent}.{new['claim']}"


def _profile(s: _Session, credential: str, *, public: bool) -> None:
    """Publication, `aboutme`, friends and blocks. With `public`, for a test agent, it publishes
    and checks the copy; otherwise it writes only private things, and removes them again."""
    agent = agent_of(credential)
    me = f"/v1/identity/{agent}/aboutme"
    publication = "/v1/identity/self/publication"

    def settings(r: Response) -> str | None:
        found = _json_object(r)
        if not found or not all(isinstance(v, bool) for v in found.values()):
            return f"not every field true or false: {found!r}"
        if not public and any(found.values()):
            return f"the smoke identity publishes {found!r}, and must publish nothing"
        return None

    s.check(
        "read publication",
        s.api("GET", publication, _headers(credential)),
        200,
        verify=settings,
    )
    if public:
        body = b'{"name":true,"friends":true}'
        response, _, _ = s.write("PUT", publication, body, credential=credential)
        s.check("publish", response, 200, verify=_has(friends=True, whoiam=False))

    added = (200, 201) if not public else 201
    friend = f"/v1/identity/self/friends/{SMOKE_FRIEND}"
    response, _, _ = s.write("PUT", friend, b"", credential=credential)
    s.check("add a friend", response, added, verify=_has(id=SMOKE_FRIEND))
    s.check(
        "read friends",
        s.api("GET", "/v1/identity/self/friends", _headers(credential)),
        200,
        verify=_lists(SMOKE_FRIEND),
    )
    if public:
        s.check(
            "aboutme with the friend",
            s.api("GET", me),
            200,
            verify=_has(id=agent, name=SMOKE_NAME, friends=[SMOKE_FRIEND]),
        )

    blocked = f"/v1/identity/self/blocked/{SMOKE_FRIEND}"
    response, _, _ = s.write("PUT", blocked, b"", credential=credential)
    s.check("block the friend", response, added, verify=_has(id=SMOKE_FRIEND))
    if public:
        s.check(
            "aboutme without the blocked friend",
            s.api("GET", me),
            200,
            verify=_has(friends=[]),
        )
    s.check(
        "read blocked",
        s.api("GET", "/v1/identity/self/blocked", _headers(credential)),
        200,
        verify=_lists(SMOKE_FRIEND),
    )
    s.check("unblock", s.api("DELETE", blocked, _headers(credential)), 200)
    s.check("remove the friend", s.api("DELETE", friend, _headers(credential)), 200)

    if public:
        response, _, _ = s.write("PUT", publication, b"{}", credential=credential)
        s.check("unpublish", response, 200, verify=_has(name=False, friends=False))
        s.check("aboutme after unpublishing", s.api("GET", me), 404, problem=True)
    s.check(
        "aboutme of an unknown agent",
        s.api("GET", f"/v1/identity/{UNKNOWN_ID}/aboutme"),
        404,
        problem=True,
    )


def _messages(s: _Session, credential: str) -> None:
    """A knock on itself, which nobody else sees, read back with `?kind=knock` and removed.
    It isn't its own friend, so it's a knock; the unit tests cover messages between friends.
    A message to an unknown agent reads a missing `identity.json`, so it also checks that real
    S3 answers `404` there, not `403`."""
    agent = agent_of(credential)
    body = b'{"text":"smoke"}'
    response, _, _ = s.write(
        "POST", f"/v1/identity/{UNKNOWN_ID}/messages", body, credential=credential
    )
    s.check("message to an unknown agent", response, 404, problem=True)
    response, _, _ = s.write("POST", f"/v1/identity/{agent}/messages", body, credential=credential)
    s.check("knock on itself", response, 202, verify=_has(to=agent, kind="knock"))
    knock = f"k-{agent}"

    def holds_the_knock(r: Response) -> str | None:
        items = [i for i in _json_object(r).get("items", []) if isinstance(i, dict)]
        if not any(i.get("message_id") == knock and i.get("text") == "smoke" for i in items):
            return f"{knock} with its text not in {items!r}"
        return None

    s.check(
        "read knocks",
        s.api("GET", "/v1/identity/self/messages?kind=knock", _headers(credential)),
        200,
        verify=holds_the_knock,
    )
    s.check(
        "remove the knock",
        s.api("DELETE", f"/v1/identity/self/messages/{knock}", _headers(credential)),
        200,
        verify=_has(message_id=knock),
    )


def _has(**expected):
    """A check that the JSON body has these values."""

    def check(r: Response) -> str | None:
        found = _json_object(r)
        wrong = {k: found.get(k) for k, v in expected.items() if found.get(k) != v}
        return f"expected {expected!r}, got {wrong!r}" if wrong else None

    return check


def _lists(agent: str):
    """A check that a list page holds this agent, and ends there."""

    def check(r: Response) -> str | None:
        found = _json_object(r)
        ids = [item.get("id") for item in found.get("items", []) if isinstance(item, dict)]
        if agent not in ids:
            return f"{agent} not in {ids!r}"
        if "next" not in found:
            return "no next"
        return None

    return check


def _execute_api(s: _Session) -> None:
    """Straight to API Gateway, past CloudFront: without the origin key, and with a wrong one."""
    url = s.target.execute_api_url.rstrip("/") + "/openapi.yaml"
    s.check("execute-api without the origin key is rejected", s.send("GET", url), 403, api=False)
    s.check(
        "execute-api with a wrong origin key",
        s.send("GET", url, {"x-api-key": _wrong_key()}),
        403,
        problem="origin-key-replaced",
    )


def _wrong_key() -> str:
    alphabet = string.ascii_letters + string.digits
    return "smoke" + "".join(secrets.choice(alphabet) for _ in range(35))


# The command line.


class UsageError(Exception):
    pass


def load_target(args: argparse.Namespace, config_path) -> Target:
    """The URLs and, for `--production`, the smoke identity, from the arguments and config."""
    api_url = args.api_url
    if api_url is None:
        try:
            api_url = sfab_config.load(config_path).api_url
        except sfab_config.ConfigError as e:
            raise UsageError(str(e)) from None
    credential = None
    if args.production:
        credential = _smoke_claim(config_path)
    if not args.execute_api.startswith("https://"):
        raise UsageError(
            "--execute-api must be the stage's https:// URL, the stack's Endpoint output"
        )
    return Target(api_url.rstrip("/"), args.execute_api, credential)


def _smoke_claim(config_path) -> str:
    try:
        value = tomllib.loads(config_path.read_text(encoding="utf-8")).get("smoke_claim")
    except (OSError, tomllib.TOMLDecodeError) as e:
        raise UsageError(f"{config_path}: can't read smoke_claim: {e}") from None
    if value is None or value == "CHANGE_ME":
        raise UsageError(
            f"--production needs smoke_claim in {config_path}: the fixed smoke identity's "
            '<id>.<claim>. Birth it once after launch and add it, as spec.md "Local config" says.'
        )
    agent, _, claim = str(value).partition(".")
    if not (_ID.fullmatch(agent) and _SECRET.fullmatch(claim)):
        raise UsageError(f"{config_path}: smoke_claim must be <id>.<claim>, as in Authorization")
    return str(value)


def parse_args(argv: list[str] | None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="smoke", description="The smoke script; see cdk/smoke.py."
    )
    parser.add_argument(
        "--execute-api", required=True, help="the stage URL, the API stack's Endpoint output"
    )
    parser.add_argument(
        "--production",
        action="store_true",
        help="after launch: birth nothing, write nothing public",
    )
    parser.add_argument("--api-url", help="defaults to https://api.<domain_name> from the config")
    return parser.parse_args(argv)


def main(
    argv: list[str] | None = None,
    transport: Transport = urllib_transport,
    config_path=None,
    codes_path: Path = CODES_FILE,
) -> int:
    args = parse_args(argv)
    try:
        target = load_target(args, config_path or sfab_config.config_path())
    except UsageError as e:
        print(e, file=sys.stderr)
        return 2
    codes = CodeFile(codes_path)
    if not args.production and codes.next() is None:
        print(f"{codes_path} has no --no-ip-limit genesis codes left: {TOP_UP}", file=sys.stderr)
        return 2
    result = run(
        transport,
        target,
        production=args.production,
        clock=time.time,
        sleep=time.sleep,
        log=print,
        codes=codes,
    )
    if result.failures:
        print(f"\n{len(result.failures)} failed:", *result.failures, sep="\n  ")
    if result.blocked:
        return 3
    return 1 if result.failures else 0


if __name__ == "__main__":
    sys.exit(main())
