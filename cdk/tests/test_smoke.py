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

from smoke import SMOKE_FRIEND, SMOKE_WHOIAM, UNKNOWN_ID, Target, run

FULL = Target(api_url=API_HOST, execute_api_url=EXECUTE_API)
PRODUCTION = Target(api_url=API_HOST, execute_api_url=EXECUTE_API, credential=SMOKE_CREDENTIAL)


def smoke(api, clock, target=FULL, production=False):
    return run(
        api, target, production=production, clock=clock, sleep=clock.sleep, log=lambda _: None
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
        "federation world",
        "federation worlds",
        "federation request",
        "outside v1",
        "unknown route",
        "no proof",
        "birth",
        "birth replay",
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
    # About 41 (spec.md "Testing"); the 4-second pace keeps any length under WAF's 100 per 5 minutes.
    assert len(api.requests) <= 41


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
        ("evolve", ok(200, {"whoiam": "old"}), "evolve"),
        ("no proof", problem(400), "birth without a proof"),
        (
            "execute-api wrong key",
            problem(403, "about:blank"),
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


def test_a_plain_403_from_execute_api_is_a_rejection_not_a_block():
    clock = Clock()
    api = FakeApi(clock)
    api.override["execute-api no key"] = plain(403, b'{"message":"Forbidden"}')

    result = smoke(api, clock)

    assert not result.blocked
    assert result.failures == []


def test_solves_proofs_with_the_servers_clock_and_bits():
    server = Clock()
    api = FakeApi(server, bits=10)
    local = Clock(server.now - 600)

    def sleep(seconds):
        local.sleep(seconds)
        server.sleep(seconds)

    result = run(api, FULL, production=False, clock=local, sleep=sleep, log=lambda _: None)

    assert result.failures == []


def test_production_births_nothing_and_writes_only_privately():
    clock = Clock()
    api = FakeApi(clock)

    result = smoke(api, clock, target=PRODUCTION, production=True)

    assert result.failures == []
    assert "birth" not in api.labels
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
