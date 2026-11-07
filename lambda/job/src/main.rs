//! The daily world job Lambda, run by a schedule just after 00:00 UTC. The CDK sets the
//! environment, the permissions, the 900-second timeout, a reserved concurrency of 1 and no
//! retries. An error is a function error, which fires its errors alarm.

use lambda_runtime::{LambdaEvent, service_fn};
use sfab_core::s3::S3Store;

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} must be set by the CDK"))
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("the clock is after 1970")
        .as_secs() as i64
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
    let store = &store;
    lambda_runtime::run(service_fn(
        move |_: LambdaEvent<serde_json::Value>| async move {
            match sfab_job::run(store, now()).await {
                Ok(report) => {
                    tracing::info!(
                        residents = report.residents,
                        pages = report.pages,
                        written = report.written,
                        deleted = report.deleted,
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
