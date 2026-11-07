"""What the API's and the website's front doors share: a web ACL with one rate rule, a Free
flat-rate plan, and the alias records. See .stories/2-identity-api/spec.md "Architecture".
"""

from __future__ import annotations

from aws_cdk import aws_cloudfront as cloudfront
from aws_cdk import aws_pricingplanmanager as pricingplanmanager
from aws_cdk import aws_route53 as route53
from aws_cdk import aws_route53_targets as targets
from aws_cdk import aws_wafv2 as wafv2
from constructs import Construct


def waf_visibility(metric_name: str) -> wafv2.CfnWebACL.VisibilityConfigProperty:
    # Metrics give blocked counts for free; samples stay off (spec.md "Logging"), because they
    # return each header for 3 hours, which could expose live claims.
    return wafv2.CfnWebACL.VisibilityConfigProperty(
        cloud_watch_metrics_enabled=True,
        metric_name=metric_name,
        sampled_requests_enabled=False,
    )


def rate_limited_web_acl(
    scope: Construct, *, metric_name: str, limit: int, window_seconds: int
) -> wafv2.CfnWebACL:
    """A CloudFront web ACL, `WebAcl`, that allows by default and blocks an IP past `limit`
    requests per `window_seconds`. No custom response: WAF's plain 403 is what agents read as
    "blocked"."""
    return wafv2.CfnWebACL(
        scope,
        "WebAcl",
        scope="CLOUDFRONT",
        default_action=wafv2.CfnWebACL.DefaultActionProperty(allow={}),
        visibility_config=waf_visibility(metric_name),
        rules=[
            wafv2.CfnWebACL.RuleProperty(
                name="RatePerIp",
                priority=0,
                statement=wafv2.CfnWebACL.StatementProperty(
                    rate_based_statement=wafv2.CfnWebACL.RateBasedStatementProperty(
                        limit=limit,
                        evaluation_window_sec=window_seconds,
                        aggregate_key_type="IP",
                    )
                ),
                action=wafv2.CfnWebACL.RuleActionProperty(block={}),
                visibility_config=waf_visibility(f"{metric_name}RatePerIp"),
            )
        ],
    )


def free_plan(scope: Construct, resource_arns: list[str]) -> None:
    """`FlatRatePlan`: the Free flat-rate plan, with no overage charges and WAF included, for
    one distribution, its web ACL and optionally a hosted zone."""
    pricingplanmanager.CfnSubscription(
        scope,
        "FlatRatePlan",
        plan_family="CloudFront",
        plan_tier="FREE",
        usage_level="DEFAULT",
        resource_arns=resource_arns,
    )


def alias_records(
    scope: Construct,
    id_prefix: str,
    *,
    zone: route53.IHostedZone,
    record_name: str,
    distribution: cloudfront.Distribution,
) -> None:
    """`<id_prefix>A` and `<id_prefix>Aaaa`, both pointing `record_name` at the distribution."""
    target = route53.RecordTarget.from_alias(targets.CloudFrontTarget(distribution))
    route53.ARecord(scope, f"{id_prefix}A", zone=zone, record_name=record_name, target=target)
    route53.AaaaRecord(scope, f"{id_prefix}Aaaa", zone=zone, record_name=record_name, target=target)
