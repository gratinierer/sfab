//! The owner's genesis-codes tool, run by hand with the owner's AWS credentials:
//! `.stories/17-genesis-codes/spec.md` "Two sources" (the owner) and "Tracking".
//!
//! `issue` prints only the codes to standard output, one a line, and its report to standard
//! error, so `genesis-codes issue ... >> smoke-genesis-codes.txt` keeps a clean file.

use std::process::ExitCode;

use sfab_core::daily::rfc3339;
use sfab_core::s3::S3Store;
use sfab_genesis_codes::{Command, USAGE, issue_codes, parse_args, status};

fn env(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|_| format!("{name} must be set to the bucket's name"))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let line: Vec<String> = std::env::args().skip(1).collect();
    if line.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        return Ok(());
    }
    let command = parse_args(&line)?;
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let store = S3Store::new(
        aws_sdk_s3::Client::new(&config),
        env("IDENTITIES_BUCKET")?,
        env("CLAIMS_BUCKET")?,
    );
    match command {
        Command::Issue { count, how } => {
            let codes = issue_codes(&store, count, &how).await.map_err(|e| {
                format!("stopped: {e:?}. Codes uploaded before the error work but weren't printed")
            })?;
            for code in &codes {
                println!("{code}");
            }
            let expires = how.expires.map_or("never expire".to_owned(), |t| {
                format!("stop at {}", rfc3339(t))
            });
            let ip_rule = if how.no_ip_limit {
                "skip the IP rule"
            } else {
                "keep the IP rule"
            };
            eprintln!(
                "{} codes added to batch {}; they {expires} and {ip_rule}.",
                codes.len(),
                how.label
            );
        }
        Command::Status => {
            let text = status(&store)
                .await
                .map_err(|e| format!("reading the summary failed: {e:?}"))?;
            print!("{text}");
        }
    }
    Ok(())
}
