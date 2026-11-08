//! The daily world job Lambda, run by a schedule just after 00:00 UTC. The CDK sets the
//! environment, the permissions, the 900-second timeout, a reserved concurrency of 1 and no
//! retries. An error is a function error, which fires its errors alarm.

use lambda_runtime::{LambdaEvent, service_fn};
use sfab_core::s3::S3Store;
use sfab_federation::Notify;
use sfab_genesis::Daily;
use sfab_job::outbound::Outbound;

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} must be set by the CDK"))
}

/// A whole-number setting from the CDK code.
fn number<T: std::str::FromStr>(name: &str) -> T {
    env(name)
        .parse()
        .unwrap_or_else(|_| panic!("{name} must be a whole number"))
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("the clock is after 1970")
        .as_secs() as i64
}

/// The owner's email: a message on the alerts topic, which only the owner subscribes to.
struct AlertsTopic {
    sns: aws_sdk_sns::Client,
    topic_arn: String,
}

impl Notify for AlertsTopic {
    async fn notify(&self, subject: &str, message: &str) -> Result<(), String> {
        self.sns
            .publish()
            .topic_arn(&self.topic_arn)
            .subject(subject)
            .message(message)
            .send()
            .await
            .map(|_| ())
            .map_err(|e| format!("sns Publish: {:?}", e.into_service_error()))
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), lambda_runtime::Error> {
    tracing_subscriber::fmt()
        .with_ansi(false)
        .without_time()
        .with_max_level(tracing::Level::INFO)
        .init();
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let store = S3Store::new(
        aws_sdk_s3::Client::new(&config),
        env("IDENTITIES_BUCKET"),
        env("CLAIMS_BUCKET"),
    );
    let alerts = AlertsTopic {
        sns: aws_sdk_sns::Client::new(&config),
        topic_arn: env("ALERTS_TOPIC_ARN"),
    };
    let genesis = Daily {
        codes_per_day: number("GENESIS_CODES_PER_DAY"),
        ip_window_hours: number("GENESIS_IP_WINDOW_HOURS"),
    };
    let outbound = Outbound::production();
    let (store, alerts, outbound, genesis) = (&store, &alerts, &outbound, &genesis);
    lambda_runtime::run(service_fn(
        move |_: LambdaEvent<serde_json::Value>| async move {
            match sfab_job::run(store, outbound, alerts, genesis, now()).await {
                Ok(report) => {
                    let (census, federation) = (report.census, report.federation);
                    tracing::info!(
                        residents = census.residents,
                        pages = census.pages,
                        written = census.written,
                        deleted = census.deleted,
                        genesis_issued = report.genesis.issued,
                        genesis_expired = report.genesis.expired,
                        genesis_ips_deleted = report.genesis.ips_deleted,
                        federation_checked = federation.checked,
                        federation_verified = federation.verified,
                        federation_emailed = federation.emailed,
                        federation_expired = federation.expired,
                        "daily world job done"
                    );
                    Ok(serde_json::json!({}))
                }
                Err(e) => {
                    // Errors and metadata only (spec.md "Logging"); world files hold no secret.
                    tracing::error!(error = ?e, "daily world job failed");
                    Err(lambda_runtime::Error::from(format!("{e:?}")))
                }
            }
        },
    ))
    .await
}
