"""Story 2's website stack: the placeholder pages at the apex domain.

See .stories/2-identity-api/spec.md "Architecture" (website).
"""

from __future__ import annotations

from pathlib import Path

import aws_cdk as cdk
from aws_cdk import aws_cloudfront as cloudfront
from aws_cdk import aws_cloudfront_origins as origins
from aws_cdk import aws_route53 as route53
from aws_cdk import aws_s3 as s3
from aws_cdk import aws_s3_deployment as s3deploy
from constructs import Construct

import front_door
from sfab_config import Config

# Per IP per 5 minutes: far above normal browsing, and stops the simplest flood for free.
WEBSITE_RATE_LIMIT = 1000
WEBSITE_RATE_WINDOW_SECONDS = 300

# A private bucket doesn't add index.html to a folder path, so this does. An extensionless
# path without a trailing slash, such as /agents, is sent to its folder, so each page has one
# URL and its relative links work. /.well-known/ is left alone, so an extensionless file there,
# such as story 10's api-catalog, is served as is.
VIEWER_FUNCTION = """function handler(event) {
  var request = event.request;
  var uri = request.uri;
  if (uri.endsWith('/')) {
    request.uri = uri + 'index.html';
    return request;
  }
  if (uri.indexOf('/.well-known/') === 0) {
    return request;
  }
  var last = uri.substring(uri.lastIndexOf('/') + 1);
  if (last.indexOf('.') !== -1) {
    return request;
  }
  return {
    statusCode: 301,
    statusDescription: 'Moved Permanently',
    headers: { location: { value: uri + '/' } }
  };
}
"""


class WebsiteStack(cdk.Stack):
    """A private bucket behind its own CloudFront, web ACL and Free plan, at the apex domain.
    It stays up during a stop: the kill switch touches only the API."""

    def __init__(
        self,
        scope: Construct,
        id: str,
        *,
        config: Config,
        certificate,
        hosted_zone: route53.IHostedZone,
        source: Path,
        **kwargs,
    ) -> None:
        super().__init__(scope, id, **kwargs)

        # Its pages come from the repo on every deploy, so it isn't in the base stack. The
        # default removal policy keeps it on a stack delete, which the owner empties by hand.
        bucket = s3.Bucket(
            self,
            "Bucket",
            block_public_access=s3.BlockPublicAccess.BLOCK_ALL,
            encryption=s3.BucketEncryption.S3_MANAGED,
            enforce_ssl=True,
        )

        web_acl = front_door.rate_limited_web_acl(
            self,
            metric_name="sfabWebsite",
            limit=WEBSITE_RATE_LIMIT,
            window_seconds=WEBSITE_RATE_WINDOW_SECONDS,
        )

        folder_pages = cloudfront.Function(
            self,
            "Paths",
            runtime=cloudfront.FunctionRuntime.JS_2_0,
            code=cloudfront.FunctionCode.from_inline(VIEWER_FUNCTION),
            comment="Adds index.html to a folder path and redirects /agents to /agents/",
        )

        distribution = cloudfront.Distribution(
            self,
            "Distribution",
            domain_names=[config.domain_name],
            certificate=certificate,
            web_acl_id=web_acl.attr_arn,
            default_behavior=cloudfront.BehaviorOptions(
                origin=origins.S3BucketOrigin.with_origin_access_control(bucket),
                viewer_protocol_policy=cloudfront.ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
                cache_policy=cloudfront.CachePolicy.CACHING_OPTIMIZED,
                function_associations=[
                    cloudfront.FunctionAssociation(
                        function=folder_pages, event_type=cloudfront.FunctionEventType.VIEWER_REQUEST
                    )
                ],
            ),
        )

        # Prunes by default, so a page deleted from website/ leaves the bucket too, and a file
        # copied in by hand is removed on the next deploy.
        s3deploy.BucketDeployment(
            self,
            "Pages",
            sources=[s3deploy.Source.asset(str(source))],
            destination_bucket=bucket,
            distribution=distribution,
            distribution_paths=["/*"],
        )

        # Its own Free plan: each plan covers one distribution. The zone is in the API's plan.
        front_door.free_plan(self, [distribution.distribution_arn, web_acl.attr_arn])
        front_door.alias_records(
            self, "Apex", zone=hosted_zone, record_name=config.domain_name, distribution=distribution
        )
