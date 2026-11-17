"""The smoke script, against a fake of the deployed API (spec.md "Testing")."""

import pytest
from api_client import Response
from fakes import (
    API_HOST,
    EXECUTE_API,
    SMOKE_CREDENTIAL,
    SMOKE_ID,
    Clock,
    FakeApi,
    ok,
    plain,
    problem,
)

from smoke import SMOKE_FRIEND, SMOKE_WHOIAM, UNKNOWN_ID, Codes, Target, run

FULL = Target(api_url=API_HOST, execute_api_url=EXECUTE_API)
PRODUCTION = Target(api_url=API_HOST, execute_api_url=EXECUTE_API, credential=SMOKE_CREDENTIAL)


def smoke(api, clock, target=FULL, production=False, codes=None, log=lambda _: None):
    """One run, with fresh `--no-ip-limit` codes unless the test gives its own."""
    if codes is None:
        codes = Codes(api.issue(2))
    return run(
        api, target, production=production, clock=clock, sleep=clock.sleep, log=log, codes=codes
    )


def failing_checks(api, clock, **kwargs):
    return [f.split(":")[0] for f in smoke(api, clock, **kwargs).failures]


def test_full_run_passes_against_a_working_api_and_covers_every_case():
    clock = Clock()
    api = FakeApi(clock)

    result = smoke(api, clock)

    assert result.failures == []
    assert not result.blocked
    assert set(api.labels) == {
        "openapi",
        "constitution",
        "census",
        "residents",
        "genesis code",
        "federation world",
        "federation worlds",
        "federation request",
        "outside v1",
        "unknown route",
        "no proof",
        "birth",
        "birth replay",
        "world birth",
        "self",
        "evolve",
        "extra field",
        "body over 32 KB",
        "malformed body",
        "unauthorized",
        "reclaim",
        "read publication",
        "publish",
        "aboutme",
        "aboutme 404",
        "add friend",
        "read friends",
        "remove friend",
        "block",
        "read blocked",
        "unblock",
        "send message",
        "message to an unknown agent",
        "read knocks",
        "remove message",
        "execute-api no key",
        "execute-api wrong key",
    }
    # About 43 (spec.md "Testing"); the 4-second pace keeps any length under WAF's 100 per 5 minutes.
    assert len(api.requests) <= 43


def test_births_with_exempt_codes_from_one_ip_and_rerun_without_too_soon():
    """Story 17 issue 06: `--no-ip-limit` codes skip the IP rule, one per birth, and the world
    code's birth may meet the IP rule on a rerun, which still proves the code was accepted."""
    clock = Clock()
    api = FakeApi(clock)
    exempt = api.issue(3)
    codes = Codes(exempt)

    first = smoke(api, clock, codes=codes)
    second = smoke(api, clock, codes=codes)

    assert first.failures == second.failures == []
    assert codes.codes == exempt[2:], "one code per birth, spent"
    assert set(exempt) & set(api.genesis) == {exempt[2]}
    assert api.labels.count("birth") == 2
    assert len(api.pool) == 2, "only the first run's 201 used up a pool code"


def test_the_world_code_birth_may_meet_the_ip_rule():
    clock = Clock()
    api = FakeApi(clock)
    api.last_birth = int(clock())

    assert smoke(api, clock).failures == []
    assert "world birth" in api.labels
    assert len(api.pool) == 3


def test_a_world_code_a_racing_birth_used_is_retried_once_with_a_fresh_one():
    # The pool is only mostly fresh (spec.md "What `birth` does" step 5).
    clock = Clock()
    api = FakeApi(clock)
    del api.genesis[api.pool[0]]
    lines = []

    assert smoke(api, clock, log=lines.append).failures == []
    assert any("world" in line and "already used" in line for line in lines), lines
    assert api.labels.count("genesis code") == 2
    assert "world birth" in api.labels, "the fresh code was accepted"
    assert len(api.pool) == 2
    # The retry's GET and birth on top of the run's 43.
    assert len(api.requests) <= 45


def test_a_world_rejecting_both_world_codes_fails():
    clock = Clock()
    api = FakeApi(clock)
    for code in api.pool:
        del api.genesis[code]

    failures = smoke(api, clock).failures

    assert [f.split(":")[0] for f in failures] == ["birth with a fresh world code"]
    assert api.labels.count("genesis code") == 2


def test_the_same_used_world_code_again_is_refetched_until_a_fresh_one():
    # The pool picks at random and a lost race leaves the used code in it, so a refetch may
    # hand back the very code just rejected; the next one may not, and that one must be born.
    clock = Clock()
    api = FakeApi(clock)
    stale = api.pool[0]
    del api.genesis[stale]
    lines = []

    def transport(request):
        answer = api(request)
        if api.labels[-1] == "invalid genesis code":
            api.handed = 0  # the next refetch picks the stale code again
        return answer

    result = run(
        transport,
        FULL,
        production=False,
        clock=clock,
        sleep=clock.sleep,
        log=lines.append,
        codes=Codes(api.issue(2)),
    )

    assert result.failures == []
    assert api.labels.count("genesis code") == 3
    assert api.labels.count("invalid genesis code") == 1, "the same code isn't sent twice"
    assert "world birth" in api.labels, "the fresh code was accepted"
    # Two refetches and a birth on top of the run's 43; at most three refetches, so 47.
    assert len(api.requests) <= 47


def test_a_one_code_pool_holding_the_used_code_passes_and_skips_the_retry():
    # Every refetch hands back the stale entry: that's the pool, not a fault.
    clock = Clock()
    api = FakeApi(clock)
    stale = api.pool[0]
    del api.genesis[stale]
    api.pool = [stale]
    lines = []

    assert smoke(api, clock, log=lines.append).failures == []
    assert api.labels.count("genesis code") == 4, "the first fetch and three refetches"
    assert api.labels.count("invalid genesis code") == 1, "the same code isn't sent twice"
    assert any("skip" in line and "same" in line for line in lines), lines
    assert len(api.requests) <= 47


@pytest.mark.parametrize(
    "response",
    [problem(404, "not-built-yet"), problem(409, "try-later", retry_after="2026-11-06T00:05:00Z")],
)
def test_no_fresh_world_code_after_a_used_one_passes_and_skips_the_retry(response):
    clock = Clock()
    api = FakeApi(clock)
    del api.genesis[api.pool[0]]
    lines = []

    def transport(request):
        answer = api(request)
        if api.labels[-1] == "invalid genesis code":
            api.override["genesis code"] = response
        return answer

    result = run(
        transport,
        FULL,
        production=False,
        clock=clock,
        sleep=clock.sleep,
        log=lines.append,
        codes=Codes(api.issue(2)),
    )

    assert result.failures == []
    assert api.labels.count("genesis code") == 2
    assert "world birth" not in api.labels
    assert any("skip" in line and "world" in line for line in lines), lines


def test_says_clearly_when_the_codes_run_out_and_skips_the_identity_checks():
    clock = Clock()
    api = FakeApi(clock)
    lines = []

    result = smoke(api, clock, codes=Codes([]), log=lines.append)

    assert [f.split(":")[0] for f in result.failures] == ["birth"]
    assert "smoke-genesis-codes.txt" in result.failures[0]
    assert "--no-ip-limit" in result.failures[0]
    assert "birth" not in api.labels
    assert "self" not in api.labels
    assert "world birth" in api.labels, "the world code's birth still runs"


@pytest.mark.parametrize(
    "response",
    [problem(404, "not-built-yet"), problem(409, "try-later", retry_after="2026-11-06T00:05:00Z")],
)
def test_no_world_code_yet_passes_and_skips_the_world_code_birth(response):
    clock = Clock()
    api = FakeApi(clock)
    api.override["genesis code"] = response
    lines = []

    assert smoke(api, clock, log=lines.append).failures == []
    assert "world birth" not in api.labels
    assert any("skip" in line and "world" in line for line in lines)


def test_paces_one_request_every_4_seconds():
    clock = Clock()
    api = FakeApi(clock)

    smoke(api, clock)

    assert clock.sleeps == [4] * (len(api.requests) - 1)


JSON = {"content-type": "application/json", "cache-control": "no-store"}


@pytest.mark.parametrize(
    "label, response, failed",
    [
        ("outside v1", plain(404), "outside /v1/"),
        ("unknown route", problem(404, detail="No such route"), "bare /v1/"),
        ("extra field", ok(200, {"whoiam": "x"}), "evolve with an extra field"),
        (
            "malformed body",
            Response(400, JSON, b'{"message": "Invalid request body"}'),
            "evolve with a malformed body",
        ),
        (
            "body over 32 KB",
            problem(400, title="Malformed request body"),
            "evolve with a maximum-size escaped body",
        ),
        ("birth replay", ok(201, {}), "birth replay"),
        ("birth replay", problem(409, "already-exists"), "birth replay"),
        ("genesis code", ok(200, {"genesis_code": "short"}), "GET /v1/world/genesis-code"),
        ("genesis code", problem(409, "try-later"), "GET /v1/world/genesis-code"),
        ("genesis code", problem(404, "about:blank"), "GET /v1/world/genesis-code"),
        ("world birth", problem(403, "about:blank"), "birth with the world's code"),
        ("world birth", problem(409, "too-soon"), "birth with the world's code"),
        ("evolve", ok(200, {"whoiam": "old"}), "evolve"),
        ("no proof", problem(400), "birth without a proof"),
        (
            "execute-api wrong key",
            problem(403, "about:blank"),
            "execute-api with a wrong origin key",
        ),
        (
            "execute-api wrong key",
            plain(403, b'{"message":"Forbidden"}'),
            "execute-api with a wrong origin key",
        ),
        ("execute-api no key", ok(200, {}), "execute-api without the origin key is rejected"),
        ("aboutme 404", ok(200, {"id": "aaaaaaaaaaaa"}), "aboutme after unpublishing"),
        ("read friends", ok(200, {"items": [], "next": None}), "read friends"),
        ("read blocked", ok(200, {"items": [], "next": None}), "read blocked"),
        ("add friend", problem(409, "list-full"), "add a friend"),
        ("unblock", problem(500), "unblock"),
        ("send message", ok(202, {"kind": "message"}), "knock on itself"),
        ("read knocks", ok(200, {"items": [], "next": None}), "read knocks"),
        ("remove message", problem(400), "remove the knock"),
        (
            "read publication",
            ok(200, {"name": "yes"}),
            "read publication",
        ),
        (
            "constitution",
            ok(200, {"version": "v", "text": "no terms"}),
            "GET /v1/world/constitution",
        ),
        ("census", ok(200, {"as_of": "2026-11-05T00:05:00Z"}), "GET /v1/world/census"),
        ("census", problem(404, "about:blank"), "GET /v1/world/census"),
        ("residents", ok(200, {"items": []}), "GET /v1/world/residents"),
        ("residents", problem(404, "about:blank"), "GET /v1/world/residents"),
        (
            "federation world",
            ok(200, {"domain": "elsewhere.example", "name": "x"}),
            "GET /v1/federation/world",
        ),
        ("federation worlds", ok(200, {"items": []}), "GET /v1/federation/worlds"),
        ("federation request", ok(202, {"token": "short"}), "federation request"),
        ("federation request", problem(503, retry_after="x"), "federation request"),
        (
            "openapi",
            Response(200, {"content-type": "application/yaml"}, b"openapi: 3.0.1\n"),
            "GET /v1/openapi.yaml",
        ),
    ],
)
def test_a_broken_behaviour_fails_its_check(label, response, failed):
    clock = Clock()
    api = FakeApi(clock)
    api.override[label] = response

    assert failed in failing_checks(api, clock)


def test_the_census_and_residents_may_be_not_built_yet_before_the_job_s_first_run():
    clock = Clock()
    api = FakeApi(clock)
    api.override["census"] = problem(404, "not-built-yet")
    api.override["residents"] = problem(404, "not-built-yet")

    assert smoke(api, clock).failures == []


def test_a_birth_whose_id_isnt_from_the_proof_fails_and_skips_the_identity_checks():
    clock = Clock()
    api = FakeApi(clock)
    api.override["birth"] = ok(
        201, {"id": "aaaaaaaaaaaa", "claim": "c" * 43, "recovery": "r" * 43}
    )

    assert failing_checks(api, clock) == ["birth"]
    assert "self" not in api.labels


def test_reclaim_must_retire_the_old_secrets():
    clock = Clock()
    api = FakeApi(clock)
    api.override["unauthorized"] = ok(200, {})

    failed = failing_checks(api, clock)

    assert "self with the old claim after reclaim" in failed
    assert "reclaim with the spent recovery code" in failed


def test_wafs_plain_403_stops_the_run_as_blocked_not_failed():
    clock = Clock()
    api = FakeApi(clock)
    api.override["self"] = plain(403)

    result = smoke(api, clock)

    assert result.blocked
    assert result.failures == []
    assert api.labels[-1] == "self"


def test_a_plain_403_from_execute_api_without_the_key_fails():
    """A disabled endpoint, as during a stop, answers a plain `403`, which mustn't pass for the
    origin key's rejection (code review C95)."""
    clock = Clock()
    api = FakeApi(clock)
    api.override["execute-api no key"] = plain(403, b'{"message":"Forbidden"}')

    result = smoke(api, clock)

    assert not result.blocked
    assert len(result.failures) == 1
    assert result.failures[0].startswith("execute-api without the origin key is rejected: ")


def test_solves_proofs_with_the_servers_clock_and_bits():
    server = Clock()
    api = FakeApi(server, bits=10)
    local = Clock(server.now - 600)

    def sleep(seconds):
        local.sleep(seconds)
        server.sleep(seconds)

    result = run(
        api,
        FULL,
        production=False,
        clock=local,
        sleep=sleep,
        log=lambda _: None,
        codes=Codes(api.issue(1)),
    )

    assert result.failures == []


def test_production_births_nothing_and_writes_only_privately():
    clock = Clock()
    api = FakeApi(clock)

    result = smoke(api, clock, target=PRODUCTION, production=True)

    assert result.failures == []
    # The production run is held to its 31 requests (spec.md "Testing" says about 31).
    assert len(api.requests) <= 31
    assert "birth" not in api.labels
    assert "world birth" not in api.labels
    assert "reclaim" not in api.labels
    writes = {(r.method, r.url) for r in api.requests if r.method in ("POST", "PUT")}
    # No federation request either: it would hold one of the 100 places for 3 days.
    assert writes == {
        ("POST", f"{API_HOST}/v1/identity/birth"),  # with no proof, so it writes nothing
        ("PUT", f"{API_HOST}/v1/identity/evolve"),
        ("PUT", f"{API_HOST}/v1/identity/self/friends/{SMOKE_FRIEND}"),
        ("PUT", f"{API_HOST}/v1/identity/self/blocked/{SMOKE_FRIEND}"),
        ("POST", f"{API_HOST}/v1/identity/{SMOKE_ID}/messages"),
        ("POST", f"{API_HOST}/v1/identity/{UNKNOWN_ID}/messages"),  # refused, nothing stored
    }
    assert api.public[SMOKE_ID] == set(), "nothing published"
    assert api.contacts[SMOKE_ID] == {"friends": [], "blocked": []}, "contacts cleaned up"
    assert api.knocks[SMOKE_ID] == {}, "its knock removed"
    assert {
        "openapi",
        "constitution",
        "census",
        "residents",
        "genesis code",
        "federation world",
        "federation worlds",
        "outside v1",
        "unknown route",
        "no proof",
        "self",
        "unauthorized",
        "evolve",
        "extra field",
        "body over 32 KB",
        "malformed body",
        "read publication",
        "aboutme 404",
        "add friend",
        "read friends",
        "remove friend",
        "block",
        "read blocked",
        "unblock",
        "send message",
        "message to an unknown agent",
        "read knocks",
        "remove message",
        "execute-api no key",
        "execute-api wrong key",
    } <= set(api.labels)
    assert "publish" not in api.labels
    assert api.identities[SMOKE_ID] == {"name": "smoke", "whoiam": SMOKE_WHOIAM}


@pytest.mark.parametrize("production", [False, True])
@pytest.mark.parametrize(
    "label, response, failed",
    [
        ("aboutme 404", ok(200, {"id": "aaaaaaaaaaaa"}), "aboutme of an unknown agent"),
        ("unauthorized", ok(200, {"id": SMOKE_ID}), "self with a wrong claim"),
        ("message to an unknown agent", ok(202, {}), "message to an unknown agent"),
    ],
)
def test_unknown_ids_and_a_wrong_claim_are_refused_in_both_modes(label, response, failed, production):
    """spec.md "Testing": refusals of what the script can't create run in both modes."""
    clock = Clock()
    api = FakeApi(clock)
    api.override[label] = response
    target = PRODUCTION if production else FULL

    assert failed in failing_checks(api, clock, target=target, production=production)


def test_the_federation_request_is_for_a_subdomain_nobody_else_can_own_and_may_repeat():
    clock = Clock()
    api = FakeApi(clock)

    assert smoke(api, clock).failures == []
    assert smoke(api, clock).failures == [], "a rerun gets the same token with 200"
    assert list(api.federation) == ["smoke-federation.example.invalid"]


def test_a_blocked_friend_left_in_the_copy_fails():
    clock = Clock()
    api = FakeApi(clock)
    api.keep_blocked_friends_public = True

    assert "aboutme without the blocked friend" in failing_checks(api, clock)


def test_an_unpublish_that_leaves_the_copy_fails():
    clock = Clock()
    api = FakeApi(clock)
    api.ignore_unpublish = True

    assert "aboutme after unpublishing" in failing_checks(api, clock)
