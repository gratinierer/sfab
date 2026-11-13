"""Contract tests on openapi/sfab.yaml and the public copy generated from it.

See .stories/2-identity-api/spec.md "Testing" and docs/api-design.md.
"""

import copy
import json
import re
from datetime import date
from pathlib import Path

import pytest
import yaml

import generate_public_openapi as gen
import sfab_config

FIXTURE = Path(__file__).parent / "fixtures" / "sfab.config.test.toml"
SPECS = [
    sfab_config.REPO_ROOT / ".stories" / "2-identity-api" / "spec.md",
    sfab_config.REPO_ROOT / ".stories" / "4-life-in-the-world" / "spec.md",
    sfab_config.REPO_ROOT / ".stories" / "17-genesis-codes" / "spec.md",
]
WRITES = ("POST", "PUT")
_ROW = re.compile(r"^\| `(GET|POST|PUT|DELETE) (/[^`]+)` \| ([^|]+) \| ([^|]*) \|", re.M)


@pytest.fixture(scope="module")
def config():
    # SFAB_CONFIG set for this call alone, so a plain pytest never reads the secret.
    return sfab_config.load(environ={"SFAB_CONFIG": str(FIXTURE)})


@pytest.fixture(scope="module")
def source():
    return gen.load_yaml(gen.SOURCE)


@pytest.fixture(scope="module")
def security():
    return gen.load_yaml(gen.PUBLIC_SECURITY)


@pytest.fixture(scope="module")
def public_text(config):
    return gen.generate(config)


@pytest.fixture(scope="module")
def public(public_text):
    return yaml.safe_load(public_text)


def route_table() -> dict[str, tuple[str, str]]:
    """`<METHOD> <path>` -> (auth, does), from both stories' route tables."""
    rows = {}
    for spec in SPECS:
        for method, path, auth, does in _ROW.findall(spec.read_text()):
            rows[f"{method} {path}"] = (auth.strip(), does.strip())
    return rows


def resolve(doc: dict, node):
    while isinstance(node, dict) and "$ref" in node:
        target = doc
        for part in node["$ref"].removeprefix("#/").split("/"):
            target = target[part]
        node = target
    return node


def schema_properties(doc: dict, schema) -> dict:
    schema = resolve(doc, schema)
    props = dict(schema.get("properties", {}))
    for part in schema.get("allOf", []):
        props |= schema_properties(doc, part)
    return props


def walk_schemas(doc: dict):
    """Every schema object in the document, inline or under components."""

    def walk(node, path):
        if isinstance(node, dict):
            if "type" in node and isinstance(node["type"], str) and "$ref" not in node:
                yield path, node
            for k, v in node.items():
                yield from walk(v, f"{path}/{k}")
        elif isinstance(node, list):
            for i, v in enumerate(node):
                yield from walk(v, f"{path}/{i}")

    yield from walk(doc.get("paths", {}), "#/paths")
    yield from walk(doc.get("components", {}), "#/components")


# --- the source document -------------------------------------------------------------


def test_info_version_is_a_date(source):
    date.fromisoformat(source["info"]["version"])


def test_every_string_has_a_max_length(source):
    missing = [p for p, s in walk_schemas(source) if s["type"] == "string" and "maxLength" not in s]
    assert not missing


def test_every_array_has_max_items(source):
    missing = [p for p, s in walk_schemas(source) if s["type"] == "array" and "maxItems" not in s]
    assert not missing


def test_every_operation_requires_the_origin_key(source):
    for method, path, op in gen.operations(source):
        assert {"origin_key": []} in op.get("security", []), f"{method} {path}"
    assert source["components"]["securitySchemes"]["origin_key"]["type"] == "apiKey"
    assert "security" not in source, "root-level security is ignored by API Gateway"


def test_every_operation_goes_to_the_api_lambda(source):
    for method, path, op in gen.operations(source):
        integration = op.get("x-amazon-apigateway-integration", {})
        assert integration.get("type") == "aws_proxy", f"{method} {path}"
        assert "{{api_lambda_arn}}" in integration["uri"]


def test_every_write_takes_the_optional_proof_header(source):
    for method, path, op in gen.operations(source):
        params = [resolve(source, p) for p in op.get("parameters", [])]
        proof = [p for p in params if p["in"] == "header" and p["name"] == "Proof"]
        if method in WRITES:
            assert proof, f"{method} {path} lacks the Proof header"
            assert proof[0]["required"] is False, "a required Proof would be refused at the gateway"
        else:
            assert not proof, f"{method} {path} must not take a Proof"


def test_every_write_has_the_required_bits_400(source):
    for method, path, op in gen.operations(source):
        if method not in WRITES:
            continue
        bad = resolve(source, op["responses"].get("400", {}))
        schema = bad.get("content", {}).get("application/problem+json", {}).get("schema", {})
        props = schema_properties(source, schema)
        assert {"required_bits", "server_time"} <= set(props), f"{method} {path}"


def test_every_list_route_declares_page(source):
    table = route_table()
    for method, path, op in gen.operations(source):
        ok = resolve(source, op["responses"].get("200", {}))
        schema = ok.get("content", {}).get("application/json", {}).get("schema", {})
        is_list = "next" in schema_properties(source, schema)
        paged = "paged" in table.get(f"{method} {path}", ("", ""))[1].lower()
        if is_list or paged:
            names = [resolve(source, p)["name"] for p in op.get("parameters", [])]
            assert "page" in names, f"{method} {path} lacks ?page="


def test_request_bodies_are_strict_and_validated_for_every_content_type(source):
    def objects(schema):
        schema = resolve(source, schema)
        if schema.get("type") == "object":
            yield schema
            for prop in schema.get("properties", {}).values():
                yield from objects(prop)
        if schema.get("type") == "array":
            yield from objects(schema["items"])

    for method, path, op in gen.operations(source):
        body = op.get("requestBody")
        if not body:
            continue
        content = body["content"]
        assert {"application/json", "$default"} <= set(content), f"{method} {path}"
        for media in content.values():
            for obj in objects(media["schema"]):
                assert obj.get("additionalProperties") is False, f"{method} {path}"


def test_the_request_validator_is_on_by_default(source):
    name = source["x-amazon-apigateway-request-validator"]
    validator = source["x-amazon-apigateway-request-validators"][name]
    assert validator == {"validateRequestBody": True, "validateRequestParameters": True}


def test_every_operation_is_in_a_route_table_with_matching_auth(source, security):
    table = route_table()
    bearer = set(security["bearer"])
    for method, path, _ in gen.operations(source):
        name = f"{method} {path}"
        assert name in table, f"{name} isn't in any story's route table"
        auth = table[name][0]
        needs_credentials = auth.startswith(("Bearer", "Recovery code"))
        assert needs_credentials == (name in bearer), f"{name}: {auth!r} vs public-security.yaml"
        assert ("with `Proof`" in auth) == (method in WRITES), f"{name}: {auth!r}"


# --- gateway responses ---------------------------------------------------------------


def render(template: str) -> dict:
    """Fills the $context variables the way API Gateway would, and parses the JSON."""
    filled = template.replace("$context.error.messageString", json.dumps('a "quoted" message'))
    # From memory, AWS's validation messages hold double quotes, so a quoted substitution is unsafe.
    filled = filled.replace(
        "$context.error.validationErrorString", 'object has missing required properties (["name"])'
    )
    filled = filled.replace("{{api_host}}", "api.example.invalid")
    return json.loads(filled)


def test_gateway_responses_are_problem_json(source):
    responses = source["x-amazon-apigateway-gateway-responses"]
    expected = {
        "DEFAULT_4XX", "DEFAULT_5XX", "THROTTLED", "BAD_REQUEST_BODY",
        "INVALID_API_KEY", "MISSING_AUTHENTICATION_TOKEN",
    }
    assert expected <= set(responses)
    for name, response in responses.items():
        params = response["responseParameters"]
        assert params["gatewayresponse.header.Content-Type"] == "'application/problem+json'", name
        assert params["gatewayresponse.header.Cache-Control"] == "'no-store'", name
        body = render(response["responseTemplates"]["application/json"])
        assert body["type"] and body["title"], name
        if "statusCode" in response:
            assert body["status"] == response["statusCode"], name


def test_a_wrong_origin_key_gets_a_retryable_403(source):
    response = source["x-amazon-apigateway-gateway-responses"]["INVALID_API_KEY"]
    assert response["statusCode"] == 403
    body = render(response["responseTemplates"]["application/json"])
    assert body["type"] == "origin-key-replaced"
    assert body["status"] == 403


def test_an_unknown_route_gets_a_404_pointing_to_the_description(source):
    response = source["x-amazon-apigateway-gateway-responses"]["MISSING_AUTHENTICATION_TOKEN"]
    assert response["statusCode"] == 404
    body = render(response["responseTemplates"]["application/json"])
    assert body["detail"].endswith("https://api.example.invalid/v1/openapi.yaml")


# --- the public copy -----------------------------------------------------------------


def test_public_copy_passes_its_own_checks(public_text, config, security):
    assert gen.check_public(public_text, config, security["bearer"]) == []


def test_public_copy_has_nothing_private(public_text, public, config):
    assert "x-amazon-" not in public_text
    assert "arn:aws" not in public_text
    assert config.origin_key not in public_text
    assert config.account_id not in public_text
    assert "{{" not in public_text
    assert "$default" not in public_text
    schemes = public["components"]["securitySchemes"]
    assert all(s["type"] != "apiKey" for s in schemes.values())
    assert "origin_key" not in public_text


def test_public_copy_has_contact_and_relative_server(public, config):
    assert public["info"]["contact"]["email"] == config.abuse_contact
    assert public["servers"] == [{"url": "/v1"}]


def test_public_copy_declares_bearer_exactly_where_needed(public, security):
    bearer = set(security["bearer"])
    for method, path, op in gen.operations(public):
        declared = any("bearer" in r for r in op.get("security", []))
        assert declared == (f"{method} {path}" in bearer), f"{method} {path}"


@pytest.mark.parametrize(
    "leak, problem",
    [
        ("x-amazon-apigateway-integration", "x-amazon-"),
        ("arn:aws:lambda:us-east-1", "ARN"),
        ("testOriginKey0000000000000000000", "origin key"),
        ("123456789012", "account ID"),
        ("{{api_host}}", "placeholder"),
    ],
)
def test_check_public_catches_leaks(public_text, config, security, leak, problem):
    leaked = public_text + f"\n# {leak}\n"
    problems = gen.check_public(leaked, config, security["bearer"])
    assert any(problem in p for p in problems), problems


def test_check_public_catches_a_missing_contact_apikey_or_bearer(public, config, security):
    broken = copy.deepcopy(public)
    del broken["info"]["contact"]
    broken["components"]["securitySchemes"]["k"] = {"type": "apiKey", "name": "k", "in": "header"}
    broken["paths"]["/identity/self"]["get"].pop("security")
    problems = gen.check_public(gen.dump(broken), config, security["bearer"])
    assert any("info.contact" in p for p in problems)
    assert any("apiKey" in p for p in problems)
    assert any("GET /identity/self" in p for p in problems)


def test_generator_refuses_an_unknown_bearer_operation(source, security, config):
    bad = copy.deepcopy(security)
    bad["bearer"].append("GET /no/such/route")
    with pytest.raises(gen.GenerateError, match="/no/such/route"):
        gen.build_public(source, bad, config.abuse_contact)
