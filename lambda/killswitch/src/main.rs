//! The kill switch Lambda: the AWS side of [`sfab_killswitch::Controls`]. The CDK sets the
//! environment and the permissions, and the rules live in the library, where they're tested.

use std::time::{SystemTime, UNIX_EPOCH};

use aws_sdk_apigateway::types::{Op, PatchOperation};
use aws_sdk_cloudwatch::primitives::DateTime;
use aws_sdk_cloudwatch::types::{Dimension, Metric, MetricDataQuery, MetricStat};
use aws_sdk_wafv2::types::{BlockAction, DefaultAction, Scope};
use lambda_runtime::{LambdaEvent, service_fn};
use sfab_killswitch::{
    COUNT_STATISTIC, Controls, Event, Failure, Limits, Spend, Step, WebAclRef, handle,
    kept_description, utc_midnight,
};

/// The description the kill switch gives its stage deployment, which is how a later check knows
/// the stage was deployed after `disableExecuteApiEndpoint` was set.
const STOP_DEPLOYMENT: &str = "sfab kill switch: execute-api endpoint disabled";

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} must be set by the CDK"))
}

fn fail(what: &str, e: impl std::fmt::Display) -> Failure {
    Failure(format!("{what}: {e}"))
}

struct Aws {
    waf: aws_sdk_wafv2::Client,
    lambda: aws_sdk_lambda::Client,
    apigateway: aws_sdk_apigateway::Client,
    cloudwatch: aws_sdk_cloudwatch::Client,
    budgets: aws_sdk_budgets::Client,
    sns: aws_sdk_sns::Client,
    web_acl: WebAclRef,
    api_function: String,
    job_function: String,
    rest_api_id: String,
    stage: String,
    account_id: String,
    stop_budget: String,
    alerts_topic: String,
}

impl Aws {
    async fn web_acl(&self) -> Result<(aws_sdk_wafv2::types::WebAcl, String), Failure> {
        let out = self
            .waf
            .get_web_acl()
            .name(&self.web_acl.name)
            .id(&self.web_acl.id)
            .scope(Scope::Cloudfront)
            .send()
            .await
            .map_err(|e| fail("wafv2 GetWebACL", e.into_service_error()))?;
        match (out.web_acl, out.lock_token) {
            (Some(acl), Some(token)) => Ok((acl, token)),
            _ => Err(Failure("wafv2 GetWebACL: no web ACL or lock token".into())),
        }
    }

    async fn rest_api(
        &self,
    ) -> Result<aws_sdk_apigateway::operation::get_rest_api::GetRestApiOutput, Failure> {
        self.apigateway
            .get_rest_api()
            .rest_api_id(&self.rest_api_id)
            .send()
            .await
            .map_err(|e| fail("apigateway GetRestApi", e.into_service_error()))
    }

    async fn execute_api_closed(&self) -> Result<bool, Failure> {
        let api = self.rest_api().await?;
        if !api.disable_execute_api_endpoint {
            return Ok(false);
        }
        let stage = self
            .apigateway
            .get_stage()
            .rest_api_id(&self.rest_api_id)
            .stage_name(&self.stage)
            .send()
            .await
            .map_err(|e| fail("apigateway GetStage", e.into_service_error()))?;
        let Some(deployment_id) = stage.deployment_id else {
            return Ok(false);
        };
        let deployment = self
            .apigateway
            .get_deployment()
            .rest_api_id(&self.rest_api_id)
            .deployment_id(deployment_id)
            .send()
            .await
            .map_err(|e| fail("apigateway GetDeployment", e.into_service_error()))?;
        Ok(deployment.description.as_deref() == Some(STOP_DEPLOYMENT))
    }
}

impl Aws {
    async fn concurrency_is_zero(&self, function: &str) -> Result<bool, Failure> {
        let out = self
            .lambda
            .get_function_concurrency()
            .function_name(function)
            .send()
            .await
            .map_err(|e| fail("lambda GetFunctionConcurrency", e.into_service_error()))?;
        Ok(out.reserved_concurrent_executions == Some(0))
    }

    async fn zero_concurrency(&self, function: &str) -> Result<(), Failure> {
        self.lambda
            .put_function_concurrency()
            .function_name(function)
            .reserved_concurrent_executions(0)
            .send()
            .await
            .map_err(|e| fail("lambda PutFunctionConcurrency", e.into_service_error()))?;
        Ok(())
    }
}

impl Controls for Aws {
    async fn in_place(&self, step: Step) -> Result<bool, Failure> {
        match step {
            Step::Waf => {
                let (acl, _) = self.web_acl().await?;
                Ok(acl.default_action.is_some_and(|a| a.block.is_some()))
            }
            Step::ApiLambda => self.concurrency_is_zero(&self.api_function).await,
            Step::ExecuteApi => self.execute_api_closed().await,
            Step::WorldJob => self.concurrency_is_zero(&self.job_function).await,
        }
    }

    async fn apply(&self, step: Step) -> Result<(), Failure> {
        match step {
            Step::Waf => {
                // UpdateWebACL replaces the whole web ACL, so everything else is sent back as is: a
                // setting left out may be reset (`docs/aws-facts.md`, the `UpdateWebACL` rows).
                let (acl, lock_token) = self.web_acl().await?;
                self.waf
                    .update_web_acl()
                    .name(&self.web_acl.name)
                    .id(&self.web_acl.id)
                    .scope(Scope::Cloudfront)
                    .lock_token(lock_token)
                    .default_action(
                        DefaultAction::builder()
                            .block(BlockAction::builder().build())
                            .build(),
                    )
                    .set_description(kept_description(acl.description))
                    .set_rules(acl.rules)
                    .set_visibility_config(acl.visibility_config)
                    .set_data_protection_config(acl.data_protection_config)
                    .set_custom_response_bodies(acl.custom_response_bodies)
                    .set_captcha_config(acl.captcha_config)
                    .set_challenge_config(acl.challenge_config)
                    .set_token_domains(acl.token_domains)
                    .set_association_config(acl.association_config)
                    .set_on_source_d_do_s_protection_config(acl.on_source_d_do_s_protection_config)
                    .send()
                    .await
                    .map_err(|e| fail("wafv2 UpdateWebACL", e.into_service_error()))?;
                Ok(())
            }
            Step::ApiLambda => self.zero_concurrency(&self.api_function).await,
            Step::WorldJob => self.zero_concurrency(&self.job_function).await,
            Step::ExecuteApi => {
                // AWS applies the flag only after a stage update, so the stage is always deployed.
                self.apigateway
                    .update_rest_api()
                    .rest_api_id(&self.rest_api_id)
                    .patch_operations(
                        PatchOperation::builder()
                            .op(Op::Replace)
                            .path("/disableExecuteApiEndpoint")
                            .value("true")
                            .build(),
                    )
                    .send()
                    .await
                    .map_err(|e| fail("apigateway UpdateRestApi", e.into_service_error()))?;
                self.apigateway
                    .create_deployment()
                    .rest_api_id(&self.rest_api_id)
                    .stage_name(&self.stage)
                    .description(STOP_DEPLOYMENT)
                    .send()
                    .await
                    .map_err(|e| fail("apigateway CreateDeployment", e.into_service_error()))?;
                Ok(())
            }
        }
    }

    async fn calls_today(&self) -> Result<f64, Failure> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("the clock is after 1970")
            .as_secs() as i64;
        let midnight = utc_midnight(now);
        // The ApiName dimension is the REST API's name as API Gateway holds it, which is read
        // rather than assumed, because a wrong name would silently count nothing.
        let api = self.rest_api().await?;
        let api_name = api
            .name
            .ok_or_else(|| Failure("apigateway GetRestApi: no name".into()))?;
        let query = MetricDataQuery::builder()
            .id("calls")
            .metric_stat(
                MetricStat::builder()
                    .metric(
                        Metric::builder()
                            .namespace("AWS/ApiGateway")
                            .metric_name("Count")
                            .dimensions(
                                Dimension::builder().name("ApiName").value(api_name).build(),
                            )
                            .build(),
                    )
                    .period(300)
                    .stat(COUNT_STATISTIC)
                    .build(),
            )
            .build();
        let mut total = 0.0;
        let mut next_token = None;
        loop {
            let out = self
                .cloudwatch
                .get_metric_data()
                .metric_data_queries(query.clone())
                .start_time(DateTime::from_secs(midnight))
                .end_time(DateTime::from_secs(now))
                .set_next_token(next_token)
                .send()
                .await
                .map_err(|e| fail("cloudwatch GetMetricData", e.into_service_error()))?;
            for result in out.metric_data_results.unwrap_or_default() {
                total += result.values.unwrap_or_default().iter().sum::<f64>();
            }
            next_token = out.next_token;
            if next_token.is_none() {
                return Ok(total);
            }
        }
    }

    async fn spend(&self) -> Result<Spend, Failure> {
        let out = self
            .budgets
            .describe_budget()
            .account_id(&self.account_id)
            .budget_name(&self.stop_budget)
            .send()
            .await
            .map_err(|e| fail("budgets DescribeBudget", e.into_service_error()))?;
        let budget = out
            .budget
            .ok_or_else(|| Failure("budgets DescribeBudget: no budget".into()))?;
        let amount = |what: &str, spend: Option<&aws_sdk_budgets::types::Spend>| {
            spend
                .and_then(|s| s.amount.parse::<f64>().ok())
                .ok_or_else(|| Failure(format!("budgets DescribeBudget: no {what}")))
        };
        // No actual spend yet this month, as just after a budget changes, reads as nothing spent.
        let actual = match budget.calculated_spend.as_ref() {
            Some(c) => amount("actual spend", c.actual_spend.as_ref())?,
            None => 0.0,
        };
        let limit = amount("limit", budget.budget_limit.as_ref())?;
        Ok(Spend { actual, limit })
    }

    async fn email(&self, subject: &str, body: &str) -> Result<(), Failure> {
        self.sns
            .publish()
            .topic_arn(&self.alerts_topic)
            .subject(subject)
            .message(body)
            .send()
            .await
            .map_err(|e| fail("sns Publish", e.into_service_error()))?;
        Ok(())
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), lambda_runtime::Error> {
    tracing_subscriber::fmt()
        .with_ansi(false)
        .without_time()
        .with_max_level(tracing::Level::INFO)
        .init();
    let web_acl =
        WebAclRef::parse(&env("WEB_ACL")).expect("WEB_ACL is the web ACL's Ref, name|id|scope");
    let limits = Limits {
        daily_calls: env("DAILY_CALL_LIMIT")
            .parse()
            .expect("DAILY_CALL_LIMIT is a whole number"),
    };
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let aws = Aws {
        waf: aws_sdk_wafv2::Client::new(&config),
        lambda: aws_sdk_lambda::Client::new(&config),
        apigateway: aws_sdk_apigateway::Client::new(&config),
        cloudwatch: aws_sdk_cloudwatch::Client::new(&config),
        budgets: aws_sdk_budgets::Client::new(&config),
        sns: aws_sdk_sns::Client::new(&config),
        web_acl,
        api_function: env("API_FUNCTION_NAME"),
        job_function: env("JOB_FUNCTION_NAME"),
        rest_api_id: env("REST_API_ID"),
        stage: env("STAGE_NAME"),
        account_id: env("ACCOUNT_ID"),
        stop_budget: env("STOP_BUDGET_NAME"),
        alerts_topic: env("ALERTS_TOPIC_ARN"),
    };
    let aws = &aws;
    lambda_runtime::run(service_fn(
        move |event: LambdaEvent<serde_json::Value>| async move {
            let result = match Event::parse(&event.payload) {
                Ok(e) => handle(aws, e, limits).await,
                Err(e) => Err(e),
            };
            // Errors and metadata only (spec.md "Logging"); an error fires the errors alarm.
            if let Err(e) = &result {
                tracing::error!("kill switch: {e}");
            }
            result
                .map(|()| serde_json::json!({}))
                .map_err(lambda_runtime::Error::from)
        },
    ))
    .await
}
