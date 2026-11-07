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

from smoke import SMOKE_FRIEND, SMOKE_WHOIAM, Target, run

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
        "execute-api no key",
        "execute-api wrong key",
    }
    assert len(api.requests) <= 35


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
        ("aboutme 404", ok(200, {"id": "aaaaaaaaaaaa"}), "aboutme of an unknown agent"),
        ("read friends", ok(200, {"items": [], "next": None}), "read friends"),
        ("read blocked", ok(200, {"items": [], "next": None}), "read blocked"),
        ("add friend", problem(409, "list-full"), "add a friend"),
        ("unblock", problem(500), "unblock"),
        (
            "read publication",
            ok(200, {"name": "yes"}),
            "read publication",
        ),
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
    assert writes == {
        ("POST", f"{API_HOST}/v1/identity/birth"),  # with no proof, so it writes nothing
        ("PUT", f"{API_HOST}/v1/identity/evolve"),
        ("PUT", f"{API_HOST}/v1/identity/self/friends/{SMOKE_FRIEND}"),
        ("PUT", f"{API_HOST}/v1/identity/self/blocked/{SMOKE_FRIEND}"),
    }
    assert api.public[SMOKE_ID] == set(), "nothing published"
    assert api.contacts[SMOKE_ID] == {"friends": [], "blocked": []}, "contacts cleaned up"
    assert {
        "openapi",
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
        "execute-api no key",
        "execute-api wrong key",
    } <= set(api.labels)
    assert "publish" not in api.labels
    assert api.identities[SMOKE_ID] == {"name": "smoke", "whoiam": SMOKE_WHOIAM}


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
