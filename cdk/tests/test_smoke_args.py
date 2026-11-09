"""The smoke script's command line and config: the API's host comes from `domain_name`, and
`--production` needs `smoke_claim`, which only this script checks (spec.md "Local config").
Each test points the loader at a fixture through SFAB_CONFIG, so none reads the secret."""

from pathlib import Path

import pytest
from fakes import EXECUTE_API, SMOKE_CREDENTIAL

import smoke

FIXTURE = Path(__file__).parent / "fixtures" / "sfab.config.test.toml"


@pytest.fixture
def config(tmp_path, monkeypatch):
    """Points the loader at a copy of the fixture with `extra` added, through SFAB_CONFIG, so
    nothing reads the secret."""

    def make(extra=""):
        path = tmp_path / "fixture.toml"
        path.write_text(FIXTURE.read_text() + extra)
        monkeypatch.setenv("SFAB_CONFIG", str(path))
        return path

    return make


def target(*argv):
    return smoke.load_target(smoke.parse_args(["--execute-api", EXECUTE_API, *argv]))


def never_called(request):
    raise AssertionError(f"no request should be sent: {request}")


def test_the_api_host_comes_from_domain_name(config):
    config()
    found = target()

    assert found == smoke.Target("https://api.sfab-test.invalid", EXECUTE_API, None)


def test_api_url_overrides_the_config(tmp_path, monkeypatch):
    monkeypatch.setenv("SFAB_CONFIG", str(tmp_path / "missing.toml"))
    found = target("--api-url", "https://api.other.invalid/")

    assert found.api_url == "https://api.other.invalid"


def test_production_reads_the_smoke_identity(config):
    config(f'smoke_claim = "{SMOKE_CREDENTIAL}"\n')

    assert target("--production").credential == SMOKE_CREDENTIAL


def test_production_without_smoke_claim_stops_with_a_clear_message(config, capsys):
    config()

    code = smoke.main(["--execute-api", EXECUTE_API, "--production"], never_called)

    assert code == 2
    assert "--production needs smoke_claim" in capsys.readouterr().err


@pytest.mark.parametrize("value", ["CHANGE_ME", "c" * 43, "smokesmokesm.short", "UPPER.x"])
def test_production_refuses_a_smoke_claim_that_isnt_id_dot_claim(config, value):
    config(f'smoke_claim = "{value}"\n')

    with pytest.raises(smoke.UsageError):
        target("--production")


def test_a_placeholder_smoke_claim_doesnt_block_a_run_without_production(config):
    config('smoke_claim = "CHANGE_ME"\n')

    assert target().credential is None


def test_the_execute_api_url_is_required_and_https(config):
    config()
    with pytest.raises(SystemExit):
        smoke.parse_args([])
    with pytest.raises(smoke.UsageError):
        smoke.load_target(smoke.parse_args(["--execute-api", "http://abc.example.invalid/v1/"]))


def test_a_broken_config_stops_before_any_request(tmp_path, monkeypatch, capsys):
    path = tmp_path / "fixture.toml"
    path.write_text('domain_name = "CHANGE_ME"\n')
    monkeypatch.setenv("SFAB_CONFIG", str(path))

    assert smoke.main(["--execute-api", EXECUTE_API], never_called) == 2
    assert "domain_name" in capsys.readouterr().err


def test_a_full_run_with_no_codes_left_stops_before_any_request(tmp_path, config, capsys):
    config()
    codes = tmp_path / "smoke-genesis-codes.txt"
    codes.write_text("\n")

    code = smoke.main(["--execute-api", EXECUTE_API], never_called, codes)

    assert code == 2
    err = capsys.readouterr().err
    assert "no --no-ip-limit genesis codes left" in err
    assert "issue --label smoke --count <n> --no-ip-limit" in err


def test_the_code_file_gives_one_code_per_line_and_drops_spent_ones(tmp_path):
    path = tmp_path / "smoke-genesis-codes.txt"
    path.write_text("aaaaaaaaaaaaaaaa\n\nbbbbbbbbbbbbbbbb\ncccccccccccccccc\n")
    codes = smoke.CodeFile(path)

    assert codes.next() == "aaaaaaaaaaaaaaaa"
    codes.spend("aaaaaaaaaaaaaaaa")

    assert path.read_text() == "bbbbbbbbbbbbbbbb\ncccccccccccccccc\n"
    assert smoke.CodeFile(path).next() == "bbbbbbbbbbbbbbbb"
    assert smoke.CodeFile(tmp_path / "missing.txt").next() is None
