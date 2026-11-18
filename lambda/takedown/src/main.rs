//! The owner's takedown program, run by hand with the owner's AWS credentials:
//! `.stories/2-identity-api/spec.md` "Storage" and `.stories/4-life-in-the-world/spec.md`
//! "Takedown". A dry run until `--execute`.

use std::process::ExitCode;
use std::time::Duration;

use sfab_core::s3::S3Store;
use sfab_core::storage::Store;
use sfab_takedown::{
    Command, Mode, USAGE, execute, parse_args, plan, plan_writing, remove_writing, writing,
};

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
    match &args.command {
        Command::Writing(key) => remove_one(&store, key, args.execute).await,
        Command::Agents(mode) => take_down(&store, mode, args.execute, args.wait_seconds).await,
    }
}

/// One writing, by its key, leaving its author.
async fn remove_one<S: Store>(store: &S, key: &str, execute_it: bool) -> Result<(), String> {
    let locations = writing(key).ok_or("not a writing")?;
    let found = plan_writing(store, &locations)
        .await
        .map_err(|e| format!("reading failed: {e:?}"))?;
    if found.is_empty() {
        println!("{key}: nothing left to remove; check the key");
        return Ok(());
    }
    for (_, k) in &found {
        println!("  {k}");
    }
    if !execute_it {
        println!("Dry run: nothing removed. Add --execute to remove these.");
        return Ok(());
    }
    remove_writing(store, &locations)
        .await
        .map_err(|e| format!("stopped, run it again to finish: {e:?}"))?;
    println!("Done.");
    Ok(())
}

/// Agents, with everything of theirs.
async fn take_down<S: Store>(
    store: &S,
    mode: &Mode,
    execute_it: bool,
    wait_seconds: u64,
) -> Result<(), String> {
    let targets = plan(store, mode)
        .await
        .map_err(|e| format!("listing failed: {e:?}"))?;
    for t in targets.iter().filter(|t| t.found.is_empty()) {
        println!("{} has no claim, marker or data: check the ID", t.born.id);
    }
    let targets: Vec<_> = targets
        .into_iter()
        .filter(|t| !t.found.is_empty())
        .collect();
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
    if !execute_it {
        println!("Dry run: nothing removed. Add --execute to remove these.");
        return Ok(());
    }
    if targets.is_empty() {
        println!("Nothing to remove.");
        return Ok(());
    }
    let wait = async {
        println!("Claims replaced with markers; waiting {wait_seconds} s for writes in flight");
        tokio::time::sleep(Duration::from_secs(wait_seconds)).await;
    };
    execute(store, &targets, wait)
        .await
        .map_err(|e| format!("stopped, run it again to finish: {e:?}"))?;
    println!(
        "Done. Run it again after the next daily run, which finds what the takedown index didn't \
         name yet."
    );
    Ok(())
}
