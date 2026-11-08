"""The client side the smoke script and the WAF check share: proof solving and reading responses.

Expected values come from the worked example in `openapi/sfab.yaml`, which the Lambda's own
tests also check, so this client and the server agree on the rule."""

from api_client import Response, agent_id, is_problem_json, is_waf_block, solve_proof

WORKED_BODY = (
    b'{"name":"Wren","whoiam":"A small agent looking for a home.",'
    b'"genesis_code":"k7m2xq4tnb5w3hpa"}'
)


def test_solves_the_worked_example():
    proof, digest = solve_proof(
        "POST", "/v1/identity/birth", WORKED_BODY, caller="-", now=1793836800, bits=20
    )
    assert proof == "1793836800:696785"
    assert digest.hex() == "00000a95c3622796c2c8f0e1f1583e874e50aefe89cbffac6cbecb0958526fe4"
    assert agent_id(digest) == "zpwlbfmfe37e"


def test_zero_bits_take_the_first_nonce():
    proof, _ = solve_proof("PUT", "/v1/identity/evolve", b"", caller="abc", now=5, bits=0)
    assert proof == "5:0"


def _response(status, content_type, body=b""):
    return Response(status=status, headers={"content-type": content_type}, body=body)


def test_problem_json_needs_the_type_and_a_json_object():
    good = _response(400, "application/problem+json", b'{"type":"about:blank","title":"x"}')
    assert is_problem_json(good)
    assert is_problem_json(_response(400, "application/problem+json; charset=utf-8", good.body))
    assert not is_problem_json(_response(400, "application/json", good.body))
    assert not is_problem_json(_response(400, "application/problem+json", b"not json"))
    assert not is_problem_json(_response(400, "application/problem+json", b'"a string"'))
    assert not is_problem_json(_response(400, "application/problem+json", b'{"title":"x"}'))


def test_a_waf_block_is_a_403_that_isnt_problem_json():
    assert is_waf_block(_response(403, "text/html", b"<html>Forbidden</html>"))
    assert not is_waf_block(
        _response(403, "application/problem+json", b'{"type":"origin-key-replaced","title":"x"}')
    )
    assert not is_waf_block(_response(404, "text/html"))
