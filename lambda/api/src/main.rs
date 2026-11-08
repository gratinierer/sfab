//! The API Lambda: one Rust function behind API Gateway, with every route
//! (`docs/adr/0001-initial-architecture.md`). The CDK sets the environment.

use std::sync::Arc;

use sfab_core::s3::S3Store;

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} must be set by the CDK"))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), lambda_http::Error> {
    tracing_subscriber::fmt()
        .with_ansi(false)
        .without_time()
        .with_max_level(tracing::Level::INFO)
        .init();
    let required_bits = env("REQUIRED_BITS")
        .parse()
        .expect("REQUIRED_BITS is a whole number");
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let store = S3Store::new(
        aws_sdk_s3::Client::new(&config),
        env("IDENTITIES_BUCKET"),
        env("CLAIMS_BUCKET"),
    );
    let here = sfab_federation::Here {
        domain: env("DOMAIN_NAME"),
        contact: env("ABUSE_CONTACT"),
    };
    let window_hours = env("GENESIS_IP_WINDOW_HOURS")
        .parse()
        .expect("GENESIS_IP_WINDOW_HOURS is a whole number");
    let ip_rule = sfab_genesis::IpRule::new(&env("IP_HASH_KEY"), window_hours);
    let app = sfab_api::app(
        Arc::new(store),
        required_bits,
        ip_rule,
        sfab_api::system_now,
        here,
    );
    lambda_http::run(app).await
}
