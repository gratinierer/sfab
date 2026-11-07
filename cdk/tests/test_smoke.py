"""The smoke script, against a fake of the deployed API (spec.md "Testing")."""

from fakes import API_HOST, EXECUTE_API, SMOKE_CREDENTIAL, Clock, FakeApi

from smoke import Target, run

FULL = Target(api_url=API_HOST, execute_api_url=EXECUTE_API)
PRODUCTION = Target(api_url=API_HOST, execute_api_url=EXECUTE_API, credential=SMOKE_CREDENTIAL)


def smoke(api, clock, target=FULL, production=False):
    return run(
        api, target, production=production, clock=clock, sleep=clock.sleep, log=lambda _: None
    )


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
        "execute-api no key",
        "execute-api wrong key",
    }
    assert len(api.requests) <= 35


def test_paces_one_request_every_4_seconds():
    clock = Clock()
    api = FakeApi(clock)

    smoke(api, clock)

    assert clock.sleeps == [4] * (len(api.requests) - 1)
