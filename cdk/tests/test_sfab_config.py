import re
import tomllib
from pathlib import Path

import pytest

import sfab_config
from sfab_config import ConfigError

FIXTURE = Path(__file__).parent / "fixtures" / "sfab.config.test.toml"


def fixture_data() -> dict:
    return tomllib.loads(FIXTURE.read_text())


def write(tmp_path: Path, data: dict) -> Path:
    path = tmp_path / "sfab.config.toml"
    path.write_text("".join(f'{k} = "{v}"\n' for k, v in data.items()))
    return path


def test_fixture_loads():
    config = sfab_config.load(FIXTURE)
    assert config.account_id == "000000000000"


def test_missing_file_names_the_path(tmp_path):
    missing = tmp_path / "nope.toml"
    with pytest.raises(ConfigError, match=re.escape(str(missing))):
        sfab_config.load(missing)


@pytest.mark.parametrize("key", sfab_config.REQUIRED_KEYS)
def test_missing_value_is_named(tmp_path, key):
    data = fixture_data()
    del data[key]
    with pytest.raises(ConfigError, match=rf"{key}: missing"):
        sfab_config.load(write(tmp_path, data))


def test_empty_value_counts_as_missing(tmp_path):
    data = fixture_data() | {"alert_email": "  "}
    with pytest.raises(ConfigError, match="alert_email: missing"):
        sfab_config.load(write(tmp_path, data))


def test_every_problem_is_reported_at_once(tmp_path):
    data = fixture_data()
    del data["alert_email"]
    data["domain_name"] = "CHANGE_ME"
    with pytest.raises(ConfigError) as e:
        sfab_config.load(write(tmp_path, data))
    assert "alert_email: missing" in str(e.value)
    assert "domain_name: still CHANGE_ME" in str(e.value)


@pytest.mark.parametrize("key", sfab_config.REQUIRED_KEYS)
def test_change_me_is_refused(tmp_path, key):
    data = fixture_data() | {key: "CHANGE_ME"}
    with pytest.raises(ConfigError, match=rf"{key}: still CHANGE_ME"):
        sfab_config.load(write(tmp_path, data))


def test_copied_example_is_refused():
    with pytest.raises(ConfigError) as e:
        sfab_config.load(sfab_config.EXAMPLE_PATH)
    for key in sfab_config.REQUIRED_KEYS:
        assert f"{key}: still CHANGE_ME" in str(e.value)


@pytest.mark.parametrize(
    "origin_key",
    ["a" * 19, "a" * 129, "has-a-dash-in-it-000000", "has space 0000000000000", "ünïcode0000000000000000"],
)
def test_badly_formed_origin_key_is_refused(tmp_path, origin_key):
    data = fixture_data() | {"origin_key": origin_key}
    with pytest.raises(ConfigError, match="origin_key: must be 20 to 128"):
        sfab_config.load(write(tmp_path, data))


@pytest.mark.parametrize("origin_key", ["a" * 20, "Z9" * 64])
def test_origin_key_bounds_are_accepted(tmp_path, origin_key):
    data = fixture_data() | {"origin_key": origin_key}
    assert sfab_config.load(write(tmp_path, data)).origin_key == origin_key


@pytest.mark.parametrize(
    "ip_hash_key",
    ["a" * 31, "a" * 129, "has-a-dash-in-it-000000000000000", "has space 0000000000000000000000"],
)
def test_badly_formed_ip_hash_key_is_refused(tmp_path, ip_hash_key):
    data = fixture_data() | {"ip_hash_key": ip_hash_key}
    with pytest.raises(ConfigError, match="ip_hash_key: must be 32 to 128"):
        sfab_config.load(write(tmp_path, data))


@pytest.mark.parametrize("ip_hash_key", ["a" * 32, "Z9" * 64])
def test_ip_hash_key_bounds_are_accepted(tmp_path, ip_hash_key):
    data = fixture_data() | {"ip_hash_key": ip_hash_key}
    assert sfab_config.load(write(tmp_path, data)).ip_hash_key == ip_hash_key


def test_badly_formed_account_id_is_refused(tmp_path):
    data = fixture_data() | {"account_id": "12345"}
    with pytest.raises(ConfigError, match="account_id: must be 12 digits"):
        sfab_config.load(write(tmp_path, data))


def test_unknown_key_is_refused(tmp_path):
    data = fixture_data() | {"smoke_clam": "typo"}
    with pytest.raises(ConfigError, match="smoke_clam"):
        sfab_config.load(write(tmp_path, data))


def test_smoke_claim_is_allowed_and_ignored(tmp_path):
    data = fixture_data() | {"smoke_claim": "anything"}
    config = sfab_config.load(write(tmp_path, data))
    assert not hasattr(config, "smoke_claim")


def test_invalid_toml_is_refused(tmp_path):
    path = tmp_path / "sfab.config.toml"
    path.write_text("account_id = \n")
    with pytest.raises(ConfigError, match="not valid TOML"):
        sfab_config.load(path)


def test_example_and_fixture_hold_the_same_keys():
    example = tomllib.loads(sfab_config.EXAMPLE_PATH.read_text())
    assert set(example) == set(sfab_config.REQUIRED_KEYS)
    assert set(fixture_data()) == set(sfab_config.REQUIRED_KEYS)


def test_example_mentions_smoke_claim_commented_out():
    text = sfab_config.EXAMPLE_PATH.read_text()
    assert re.search(r"^# smoke_claim = ", text, re.MULTILINE)


def test_sfab_config_env_var_names_the_file(tmp_path):
    path = tmp_path / "other.toml"
    assert sfab_config.config_path({"SFAB_CONFIG": str(path)}) == path
    assert sfab_config.config_path({}) == sfab_config.DEFAULT_PATH
