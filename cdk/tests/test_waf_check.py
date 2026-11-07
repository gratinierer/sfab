"""The WAF check: about one `birth` with no proof per second, for up to 5 minutes, passing at
the first plain `403` (spec.md "Testing")."""

from fakes import Clock, Recorder, plain, problem

from waf_check import run

API = "https://api.example.invalid"


def test_passes_at_the_first_plain_403():
    clock = Clock()
    answers = iter([problem(400, required_bits=20)] * 3 + [plain(403)])
    transport = Recorder(lambda _: next(answers))

    assert run(transport, API, clock=clock, sleep=clock.sleep, log=lambda _: None)

    assert len(transport.requests) == 4
    first = transport.requests[0]
    assert (first.method, first.url) == ("POST", f"{API}/v1/identity/birth")
    assert "Proof" not in first.headers
    assert clock.sleeps == [1, 1, 1]


def test_fails_when_no_block_arrives_in_5_minutes():
    clock = Clock()
    start = clock.now
    transport = Recorder(lambda _: problem(400, required_bits=20))

    assert not run(transport, API, clock=clock, sleep=clock.sleep, log=lambda _: None)

    assert clock.now - start >= 300
    assert 290 <= len(transport.requests) <= 301


def test_a_problem_json_403_is_not_a_block():
    clock = Clock()
    transport = Recorder(lambda _: problem(403, "origin-key-replaced"))

    assert not run(transport, API, clock=clock, sleep=clock.sleep, log=lambda _: None)
