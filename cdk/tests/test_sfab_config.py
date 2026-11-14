"""The config loader: the secret `sfab/config` in us-east-1, or the fixture `SFAB_CONFIG` names.

Secrets Manager and STS are stubbed (`fake_aws.py`), so nothing here reaches AWS.
"""

import tomllib
from pathlib import Path

import pytest
from fake_aws import NoAws, StubbedAws

import sfab_config
from sfab_config import ConfigError

FIXTURE = Path(__file__).parent / "fixtures" / "sfab.config.test.toml"
ACCOUNT = "123456789012"


def fixture_data() -> dict:
    return tomllib.loads(FIXTURE.read_text())


def secret_data() -> dict:
    """A full secret: the fixture's entries without `account_id`, which the secret refuses."""
    data = fixture_data()
    del data["account_id"]
    return data


def from_secret(secret, **kwargs) -> sfab_config.Config:
    aws = StubbedAws(secret, account=ACCOUNT, **kwargs)
    return sfab_config.load(environ={}, aws=aws)


def write(tmp_path: Path, data: dict) -> dict:
    """The environment that points the loader at a fixture holding `data`."""
    path = tmp_path / "fixture.toml"
    path.write_text("".join(f'{k} = "{v}"\n' for k, v in data.items()))
    return {"SFAB_CONFIG": str(path)}


# The secret.


def test_a_full_secret_loads_with_the_callers_account():
    aws = StubbedAws(secret_data(), account=ACCOUNT)

    config = sfab_config.load(environ={}, aws=aws)

    assert config.account_id == ACCOUNT
    assert config.domain_name == "sfab-test.invalid"
    assert config.smoke_claim is None
    assert aws.calls == ["secretsmanager@us-east-1", "sts@None"]


@pytest.mark.parametrize("key", sfab_config.REQUIRED_KEYS)
def test_a_missing_secret_entry_is_named(key):
    data = secret_data()
    del data[key]
    with pytest.raises(ConfigError, match=rf"{key}: missing"):
        from_secret(data)


@pytest.mark.parametrize("key", sfab_config.REQUIRED_KEYS)
def test_a_blank_secret_entry_counts_as_missing(key):
    with pytest.raises(ConfigError, match=rf"{key}: missing"):
        from_secret(secret_data() | {key: "  "})


@pytest.mark.parametrize("key", sfab_config.REQUIRED_KEYS)
def test_change_me_in_the_secret_is_refused(key):
    with pytest.raises(ConfigError, match=rf"{key}: still CHANGE_ME"):
        from_secret(secret_data() | {key: "CHANGE_ME"})


def test_a_value_that_isnt_a_string_is_refused():
    with pytest.raises(ConfigError, match="hosted_zone_id: must be a string"):
        from_secret(secret_data() | {"hosted_zone_id": 42})


def test_the_secret_refuses_account_id_as_an_unknown_key():
    with pytest.raises(ConfigError, match="unknown key.*account_id"):
        from_secret(secret_data() | {"account_id": ACCOUNT})


def test_a_badly_formed_origin_key_in_the_secret_is_refused():
    with pytest.raises(ConfigError, match="origin_key: must be 20 to 128"):
        from_secret(secret_data() | {"origin_key": "short"})


def test_every_secret_problem_is_named_at_once_with_the_secret():
    data = secret_data() | {"domain_name": "CHANGE_ME", "smoke_clam": "typo"}
    del data["alert_email"]
    with pytest.raises(ConfigError) as e:
        from_secret(data)
    message = str(e.value)
    assert "sfab/config" in message
    assert "alert_email: missing" in message
    assert "domain_name: still CHANGE_ME" in message
    assert "smoke_clam" in message


def test_a_missing_secret_is_named():
    aws = StubbedAws(
        secret_error=("ResourceNotFoundException", "Secrets Manager can't find the specified secret.")
    )
    with pytest.raises(ConfigError, match="sfab/config.*the secret is missing"):
        sfab_config.load(environ={}, aws=aws)


def test_a_secret_with_no_value_is_named():
    aws = StubbedAws(
        secret_error=(
            "ResourceNotFoundException",
            "Secrets Manager can't find the specified secret value for staging label: AWSCURRENT",
        )
    )
    with pytest.raises(ConfigError, match="sfab/config.*it has no value"):
        sfab_config.load(environ={}, aws=aws)


def test_a_blank_secret_string_counts_as_no_value():
    # Secrets Manager refuses an empty SecretString, but not a blank one.
    with pytest.raises(ConfigError, match="it has no value"):
        from_secret("  \n")


@pytest.mark.parametrize("value", ["not json", "[1, 2]", '"a string"', '{"alert_email": '])
def test_a_secret_that_isnt_a_json_object_is_named(value):
    with pytest.raises(ConfigError, match="sfab/config.*its value isn't a JSON object"):
        from_secret(value)


def test_an_unreadable_secret_names_the_aws_error():
    aws = StubbedAws(secret_error=("AccessDeniedException", "not allowed"))
    with pytest.raises(ConfigError, match="sfab/config.*can't be read.*AccessDenied"):
        sfab_config.load(environ={}, aws=aws)


def test_smoke_claim_passes_through_without_the_placeholder_checks():
    assert from_secret(secret_data() | {"smoke_claim": "CHANGE_ME"}).smoke_claim == "CHANGE_ME"
    assert from_secret(secret_data() | {"smoke_claim": "a.b"}).smoke_claim == "a.b"


def test_smoke_claim_must_be_a_string():
    with pytest.raises(ConfigError, match="smoke_claim: must be a string"):
        from_secret(secret_data() | {"smoke_claim": 7})


# The deploy's account.


def test_the_account_comes_from_cdk_default_account_first():
    environ = {"CDK_DEFAULT_ACCOUNT": "210987654321", "SFAB_CONFIG": str(FIXTURE)}
    assert sfab_config.deploy_account(environ=environ, aws=NoAws()) == "210987654321"


def test_the_account_comes_from_the_fixture_without_calling_sts():
    environ = {"SFAB_CONFIG": str(FIXTURE)}
    assert sfab_config.deploy_account(environ=environ, aws=NoAws()) == "000000000000"


def test_the_account_comes_from_the_caller_identity():
    aws = StubbedAws(account=ACCOUNT)
    assert sfab_config.deploy_account(environ={}, aws=aws) == ACCOUNT
    assert aws.calls == ["sts@None"]


def test_an_unloadable_fixture_falls_through_to_sts(tmp_path):
    aws = StubbedAws(account=ACCOUNT)
    environ = {"SFAB_CONFIG": str(tmp_path / "missing.toml")}
    assert sfab_config.deploy_account(environ=environ, aws=aws) == ACCOUNT


def test_an_unloadable_fixture_is_named_when_sts_fails_too(tmp_path):
    aws = StubbedAws()  # STS gets no answer.
    missing = tmp_path / "missing.toml"
    with pytest.raises(ConfigError) as raised:
        sfab_config.deploy_account(environ={"SFAB_CONFIG": str(missing)}, aws=aws)
    assert "SFAB_CONFIG's fixture didn't give one either" in str(raised.value)
    assert str(missing) in str(raised.value)


def test_no_credentials_for_the_account_is_a_config_error():
    aws = StubbedAws()  # STS gets no answer: as with no credentials, the call fails.
    with pytest.raises(ConfigError, match="account"):
        sfab_config.deploy_account(environ={}, aws=aws)


# The fixture, through SFAB_CONFIG.


def test_sfab_config_selects_the_fixture_without_touching_aws():
    config = sfab_config.load(environ={"SFAB_CONFIG": str(FIXTURE)}, aws=NoAws())
    assert config.account_id == "000000000000"


def test_the_fixture_account_wins_over_cdk_default_account():
    environ = {"SFAB_CONFIG": str(FIXTURE), "CDK_DEFAULT_ACCOUNT": ACCOUNT}
    assert sfab_config.load(environ=environ, aws=NoAws()).account_id == "000000000000"


def test_a_missing_fixture_names_the_path(tmp_path):
    missing = tmp_path / "nope.toml"
    with pytest.raises(ConfigError, match=str(missing)):
        sfab_config.load(environ={"SFAB_CONFIG": str(missing)}, aws=NoAws())


def test_the_fixture_holds_the_secrets_keys_plus_account_id():
    assert set(fixture_data()) == set(sfab_config.REQUIRED_KEYS) | {"account_id"}


@pytest.mark.parametrize("key", (*sfab_config.REQUIRED_KEYS, "account_id"))
def test_a_missing_fixture_value_is_named(tmp_path, key):
    data = fixture_data()
    del data[key]
    with pytest.raises(ConfigError, match=rf"{key}: missing"):
        sfab_config.load(environ=write(tmp_path, data), aws=NoAws())


@pytest.mark.parametrize("key", sfab_config.REQUIRED_KEYS)
def test_change_me_in_the_fixture_is_refused(tmp_path, key):
    data = fixture_data() | {key: "CHANGE_ME"}
    with pytest.raises(ConfigError, match=rf"{key}: still CHANGE_ME"):
        sfab_config.load(environ=write(tmp_path, data), aws=NoAws())


@pytest.mark.parametrize(
    "origin_key",
    ["a" * 19, "a" * 129, "has-a-dash-in-it-000000", "has space 0000000000000", "ünïcode0000000000000000"],
)
def test_badly_formed_origin_key_is_refused(tmp_path, origin_key):
    data = fixture_data() | {"origin_key": origin_key}
    with pytest.raises(ConfigError, match="origin_key: must be 20 to 128"):
        sfab_config.load(environ=write(tmp_path, data), aws=NoAws())


@pytest.mark.parametrize("origin_key", ["a" * 20, "Z9" * 64])
def test_origin_key_bounds_are_accepted(tmp_path, origin_key):
    data = fixture_data() | {"origin_key": origin_key}
    assert sfab_config.load(environ=write(tmp_path, data), aws=NoAws()).origin_key == origin_key


@pytest.mark.parametrize(
    "ip_hash_key",
    ["a" * 31, "a" * 129, "has-a-dash-in-it-000000000000000", "has space 0000000000000000000000"],
)
def test_badly_formed_ip_hash_key_is_refused(tmp_path, ip_hash_key):
    data = fixture_data() | {"ip_hash_key": ip_hash_key}
    with pytest.raises(ConfigError, match="ip_hash_key: must be 32 to 128"):
        sfab_config.load(environ=write(tmp_path, data), aws=NoAws())


@pytest.mark.parametrize("ip_hash_key", ["a" * 32, "Z9" * 64])
def test_ip_hash_key_bounds_are_accepted(tmp_path, ip_hash_key):
    data = fixture_data() | {"ip_hash_key": ip_hash_key}
    assert sfab_config.load(environ=write(tmp_path, data), aws=NoAws()).ip_hash_key == ip_hash_key


def test_badly_formed_account_id_is_refused(tmp_path):
    data = fixture_data() | {"account_id": "12345"}
    with pytest.raises(ConfigError, match="account_id: must be 12 digits"):
        sfab_config.load(environ=write(tmp_path, data), aws=NoAws())


def test_unknown_key_is_refused(tmp_path):
    data = fixture_data() | {"smoke_clam": "typo"}
    with pytest.raises(ConfigError, match="smoke_clam"):
        sfab_config.load(environ=write(tmp_path, data), aws=NoAws())


def test_the_fixture_passes_smoke_claim_through(tmp_path):
    data = fixture_data() | {"smoke_claim": "anything"}
    assert sfab_config.load(environ=write(tmp_path, data), aws=NoAws()).smoke_claim == "anything"


def test_invalid_toml_is_refused(tmp_path):
    path = tmp_path / "fixture.toml"
    path.write_text("account_id = \n")
    with pytest.raises(ConfigError, match="not valid TOML"):
        sfab_config.load(environ={"SFAB_CONFIG": str(path)}, aws=NoAws())


# scripts/check's choice: the secret, or the fixture with a message.


def test_check_uses_the_secret_when_it_loads():
    warnings = []
    aws = StubbedAws(secret_data(), account=ACCOUNT)

    config = sfab_config.load_for_check(False, environ={}, aws=aws, warn=warnings.append)

    assert config.account_id == ACCOUNT
    assert warnings == []


def test_check_falls_back_to_the_fixture_when_the_secret_cant_be_read():
    warnings = []
    aws = StubbedAws(secret_error=("AccessDeniedException", "no credentials"))

    config = sfab_config.load_for_check(False, environ={}, aws=aws, warn=warnings.append)

    assert config.account_id == "000000000000"
    assert len(warnings) == 1
    assert "AccessDenied" in warnings[0]
    assert "sfab.config.test.toml" in warnings[0]


def test_check_falls_back_to_the_fixture_when_the_secret_is_incomplete():
    warnings = []
    data = secret_data()
    del data["hosted_zone_id"]
    aws = StubbedAws(data, account=ACCOUNT)

    config = sfab_config.load_for_check(False, environ={}, aws=aws, warn=warnings.append)

    assert config.account_id == "000000000000"
    assert "hosted_zone_id: missing" in warnings[0]


def test_check_with_require_secret_refuses_to_fall_back():
    data = secret_data()
    del data["hosted_zone_id"]
    aws = StubbedAws(data, account=ACCOUNT)
    with pytest.raises(ConfigError, match="hosted_zone_id: missing"):
        sfab_config.load_for_check(True, environ={}, aws=aws, warn=pytest.fail)


def test_check_uses_a_set_sfab_config_without_touching_secrets_manager():
    config = sfab_config.load_for_check(
        False, environ={"SFAB_CONFIG": str(FIXTURE)}, aws=NoAws(), warn=pytest.fail
    )
    assert config.account_id == "000000000000"


def test_check_with_require_secret_refuses_a_set_sfab_config():
    with pytest.raises(ConfigError, match="SFAB_CONFIG"):
        sfab_config.load_for_check(
            True, environ={"SFAB_CONFIG": str(FIXTURE)}, aws=NoAws(), warn=pytest.fail
        )


def test_the_key_lists_come_from_config():
    assert sfab_config.REQUIRED_KEYS == (
        "alert_email",
        "abuse_contact",
        "domain_name",
        "hosted_zone_id",
        "origin_key",
        "ip_hash_key",
    )
    assert sfab_config.OPTIONAL_KEYS == ("smoke_claim",)
