"""CDK assertion tests for the website stack, from spec.md "Architecture" (website) and
"Testing", and issue 12.

Built from the committed fixture config and a stand-in website folder, so they need no
generate step.
"""

import json
import subprocess

import pytest
from aws_cdk.assertions import Template


# AWS's managed cache policy ID.
CACHING_OPTIMIZED = "658327ea-f89d-4fab-a63d-7e88639e58f6"


@pytest.fixture(scope="module")
def website(built):
    return Template.from_stack(built.website)


@pytest.fixture(scope="module")
def base(built):
    return Template.from_stack(built.base)


def only(template: Template, kind: str) -> tuple[str, dict]:
    found = template.find_resources(kind)
    assert len(found) == 1, (kind, list(found))
    logical_id, resource = next(iter(found.items()))
    return logical_id, resource["Properties"]


def test_the_website_stack_targets_the_config_account_in_us_east_1(built, config):
    assert built.website.account == config.account_id
    assert built.website.region == "us-east-1"


def test_the_website_bucket_is_private_and_read_only_by_its_distribution(website):
    bucket_id, bucket = only(website, "AWS::S3::Bucket")
    assert bucket["PublicAccessBlockConfiguration"] == {
        "BlockPublicAcls": True,
        "BlockPublicPolicy": True,
        "IgnorePublicAcls": True,
        "RestrictPublicBuckets": True,
    }
    assert "WebsiteConfiguration" not in bucket
    _, policy = only(website, "AWS::S3::BucketPolicy")
    dist_id, _ = only(website, "AWS::CloudFront::Distribution")
    allows = [s for s in policy["PolicyDocument"]["Statement"] if s["Effect"] == "Allow"]
    assert len(allows) == 1
    (allow,) = allows
    assert allow["Principal"] == {"Service": "cloudfront.amazonaws.com"}
    assert allow["Action"] == "s3:GetObject"
    assert dist_id in json.dumps(allow["Condition"])
    # Origin access control, not a legacy origin access identity.
    _, oac = only(website, "AWS::CloudFront::OriginAccessControl")
    assert oac["OriginAccessControlConfig"]["OriginAccessControlOriginType"] == "s3"
    assert website.find_resources("AWS::CloudFront::CloudFrontOriginAccessIdentity") == {}
    assert bucket_id in json.dumps(policy["Bucket"])


def test_the_distribution_serves_the_apex_domain_over_https_with_the_base_certificate(
    website, config
):
    _, props = only(website, "AWS::CloudFront::Distribution")
    dist = props["DistributionConfig"]
    assert dist["Aliases"] == [config.domain_name]
    assert "Certificate" in json.dumps(dist["ViewerCertificate"]["AcmCertificateArn"])
    behavior = dist["DefaultCacheBehavior"]
    assert behavior["ViewerProtocolPolicy"] == "redirect-to-https"
    assert behavior["CachePolicyId"] == CACHING_OPTIMIZED
    acl_id, _ = only(website, "AWS::WAFv2::WebACL")
    assert dist["WebACLId"] == {"Fn::GetAtt": [acl_id, "Arn"]}
    (association,) = behavior["FunctionAssociations"]
    function_id = association["FunctionARN"]["Fn::GetAtt"][0]
    assert function_id in website.find_resources("AWS::CloudFront::Function")
    assert behavior["FunctionAssociations"] == [
        {"EventType": "viewer-request", "FunctionARN": {"Fn::GetAtt": [function_id, "FunctionARN"]}}
    ]


def test_the_web_acl_blocks_an_ip_over_1000_requests_in_5_minutes_without_samples(website):
    _, acl = only(website, "AWS::WAFv2::WebACL")
    assert acl["Scope"] == "CLOUDFRONT"
    assert acl["DefaultAction"] == {"Allow": {}}
    assert acl["VisibilityConfig"]["SampledRequestsEnabled"] is False
    (rule,) = acl["Rules"]
    assert rule["Statement"] == {
        "RateBasedStatement": {"Limit": 1000, "EvaluationWindowSec": 300, "AggregateKeyType": "IP"}
    }
    assert rule["Action"] == {"Block": {}}
    assert rule["VisibilityConfig"]["SampledRequestsEnabled"] is False


def test_the_website_has_its_own_free_flat_rate_plan_without_the_zone(website, config):
    _, plan = only(website, "AWS::PricingPlanManager::Subscription")
    assert plan["PlanFamily"] == "CloudFront"
    assert plan["PlanTier"] == "FREE"
    assert plan["UsageLevel"] == "DEFAULT"
    dist_id, _ = only(website, "AWS::CloudFront::Distribution")
    acl_id, _ = only(website, "AWS::WAFv2::WebACL")
    arns = json.dumps(plan["ResourceArns"])
    assert len(plan["ResourceArns"]) == 2
    assert dist_id in arns and acl_id in arns
    # The zone is in the API stack's plan; a resource can be in only one.
    assert "hostedzone" not in arns


def test_apex_dns_records_alias_the_website_distribution(website, config):
    dist_id, _ = only(website, "AWS::CloudFront::Distribution")
    records = [r["Properties"] for r in website.find_resources("AWS::Route53::RecordSet").values()]
    assert sorted(r["Type"] for r in records) == ["A", "AAAA"]
    for record in records:
        assert record["Name"] == f"{config.domain_name}."
        assert record["HostedZoneId"] == config.hosted_zone_id
        assert dist_id in json.dumps(record["AliasTarget"]["DNSName"])


def test_the_bucket_deployment_copies_the_pages_and_refreshes_the_distribution(website):
    found = website.find_resources("Custom::CDKBucketDeployment")
    assert len(found) == 1
    (deployment,) = found.values()
    props = deployment["Properties"]
    bucket_id, _ = only(website, "AWS::S3::Bucket")
    dist_id, _ = only(website, "AWS::CloudFront::Distribution")
    assert props["DestinationBucketName"] == {"Ref": bucket_id}
    assert props["DistributionId"] == {"Ref": dist_id}
    assert props["DistributionPaths"] == ["/*"]
    # Pruning removes a deleted page from the bucket; CDK leaves the key out when on.
    assert props.get("Prune", True) is True
    assert len(props["SourceObjectKeys"]) == 1


def test_the_website_stack_has_no_topics_or_budgets(website):
    for kind in ("AWS::SNS::Topic", "AWS::Budgets::Budget"):
        assert website.find_resources(kind) == {}, kind


def test_the_base_certificate_covers_the_apex_domain(base, config):
    _, cert = only(base, "AWS::CertificateManager::Certificate")
    assert cert["DomainName"] == config.domain_name


# The viewer-request function


def run_viewer_function(code: str, uri: str) -> dict:
    script = code + "\nconsole.log(JSON.stringify(handler({request: {uri: %s, method: 'GET'}})));" % (
        json.dumps(uri)
    )
    out = subprocess.run(["node", "-e", script], capture_output=True, text=True, check=True)
    return json.loads(out.stdout)


def function_by_event(website: Template, behavior: dict, event: str) -> dict:
    (association,) = [a for a in behavior["FunctionAssociations"] if a["EventType"] == event]
    function_id = association["FunctionARN"]["Fn::GetAtt"][0]
    return website.find_resources("AWS::CloudFront::Function")[function_id]["Properties"]


def viewer_code(website: Template) -> str:
    _, dist = only(website, "AWS::CloudFront::Distribution")
    behavior = dist["DistributionConfig"]["DefaultCacheBehavior"]
    props = function_by_event(website, behavior, "viewer-request")
    assert props["FunctionConfig"]["Runtime"] == "cloudfront-js-2.0"
    return props["FunctionCode"]


@pytest.mark.parametrize(
    "uri, expected",
    [("/", "/index.html"), ("/agents/", "/agents/index.html"), ("/a/b/", "/a/b/index.html")],
)
def test_a_path_ending_in_a_slash_gets_index_html(website, uri, expected):
    assert run_viewer_function(viewer_code(website), uri) == {"uri": expected, "method": "GET"}


@pytest.mark.parametrize("uri", ["/agents", "/a/b", "/well-known/x"])
def test_an_extensionless_path_without_a_slash_is_redirected_to_its_folder(website, uri):
    response = run_viewer_function(viewer_code(website), uri)
    assert response["statusCode"] == 301
    assert response["headers"]["location"] == {"value": uri + "/"}


@pytest.mark.parametrize(
    "uri",
    [
        "/style.css",
        "/agents/index.html",
        "/shelter-icon.svg",
        "/.well-known/api-catalog",
        "/.well-known/sfab-federation/abc123",
    ],
)
def test_every_other_path_is_left_alone(website, uri):
    assert run_viewer_function(viewer_code(website), uri) == {"uri": uri, "method": "GET"}


# The API catalog's cache behaviour, from .stories/10-website/ issue 03

CATALOG_TYPE = 'application/linkset+json; profile="https://www.rfc-editor.org/info/rfc9727"'


def catalog_behavior(website: Template) -> dict:
    _, props = only(website, "AWS::CloudFront::Distribution")
    behaviors = props["DistributionConfig"]["CacheBehaviors"]
    (behavior,) = [b for b in behaviors if b["PathPattern"] == "/.well-known/api-catalog"]
    return behavior


def run_response_function(code: str, method: str) -> dict:
    event = {
        "request": {"uri": "/.well-known/api-catalog", "method": method},
        "response": {
            "statusCode": 200,
            "headers": {"content-type": {"value": "binary/octet-stream"}, "etag": {"value": '"x"'}},
        },
    }
    script = code + "\nconsole.log(JSON.stringify(handler(%s)));" % json.dumps(event)
    out = subprocess.run(["node", "-e", script], capture_output=True, text=True, check=True)
    return json.loads(out.stdout)


def test_the_api_catalog_has_its_own_cached_behaviour_from_the_same_bucket(website):
    _, props = only(website, "AWS::CloudFront::Distribution")
    dist = props["DistributionConfig"]
    assert [b["PathPattern"] for b in dist["CacheBehaviors"]] == ["/.well-known/api-catalog"]
    behavior = catalog_behavior(website)
    # The managed policy, since Free allows no custom one.
    assert behavior["CachePolicyId"] == CACHING_OPTIMIZED
    assert behavior["ViewerProtocolPolicy"] == "redirect-to-https"
    assert sorted(behavior.get("AllowedMethods", ["GET", "HEAD"])) == ["GET", "HEAD"]
    # The file deploys with the rest of the site, so it comes from the pages' origin.
    assert behavior["TargetOriginId"] == dist["DefaultCacheBehavior"]["TargetOriginId"]
    assert [a["EventType"] for a in behavior["FunctionAssociations"]] == ["viewer-response"]


@pytest.mark.parametrize("method", ["GET", "HEAD"])
def test_the_catalog_response_gets_its_linkset_type_and_link_header(website, config, method):
    props = function_by_event(website, catalog_behavior(website), "viewer-response")
    assert props["FunctionConfig"]["Runtime"] == "cloudfront-js-2.0"
    response = run_response_function(props["FunctionCode"], method)
    assert response["statusCode"] == 200
    assert response["headers"]["content-type"] == {"value": CATALOG_TYPE}
    assert response["headers"]["link"] == {
        "value": f'<https://{config.domain_name}/.well-known/api-catalog>; rel="api-catalog"'
    }
    # Other headers pass through.
    assert response["headers"]["etag"] == {"value": '"x"'}
