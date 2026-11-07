//! The owner's takedown program, run by hand with the owner's AWS credentials:
//! `.stories/2-identity-api/spec.md` "Storage". A dry run until `--execute`.

use std::process::ExitCode;
use std::time::Duration;

use sfab_core::s3::S3Store;
use sfab_takedown::{USAGE, execute, parse_args, plan};

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
    let args = parse_args(&line)?;
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let store = S3Store::new(
        aws_sdk_s3::Client::new(&config),
        env("IDENTITIES_BUCKET")?,
        env("CLAIMS_BUCKET")?,
    );

    let targets = plan(&store, &args.mode)
        .await
        .map_err(|e| format!("listing failed: {e:?}"))?;
    for t in &targets {
        let state = if t.born.taken_down { "marker" } else { "live" };
        let born_at = t.born.born_at.as_deref().unwrap_or("no born_at");
        println!("{} {state} born {born_at}", t.born.id);
        for (_, key) in &t.found {
            println!("  {key}");
        }
    }
    let live = targets.iter().filter(|t| !t.born.taken_down).count();
    println!(
        "{} agents, {live} live and {} already markers",
        targets.len(),
        targets.len() - live
    );
    if !args.execute {
        println!("Dry run: nothing removed. Add --execute to remove these.");
        return Ok(());
    }
    if targets.is_empty() {
        return Ok(());
    }
    let wait = async {
        println!(
            "Claims replaced with markers; waiting {} s for writes in flight",
            args.wait_seconds
        );
        tokio::time::sleep(Duration::from_secs(args.wait_seconds)).await;
    };
    execute(&store, &targets, wait)
        .await
        .map_err(|e| format!("stopped, run it again to finish: {e:?}"))?;
    println!("Done.");
    Ok(())
}
