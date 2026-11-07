//! The command line.

use sfab_core::daily::is_rfc3339_second;
use sfab_core::id::AgentId;

use crate::Mode;

pub const USAGE: &str = "\
usage: takedown id <ID> [--execute] [--wait-seconds N]
       takedown window <FROM> <TO> [--execute] [--wait-seconds N]

Without --execute it only shows what it would remove. A window takes every agent whose born_at is
at or after FROM and before TO, given as UTC to the second, such as 2026-11-05T12:00:00Z, and also
catches real agents born then. --wait-seconds must be at least the API Lambda's timeout (default
60). Needs IDENTITIES_BUCKET, CLAIMS_BUCKET and the owner's AWS credentials.";

/// The API Lambda's timeout: `API_TIMEOUT_SECONDS` in cdk/stacks.py. The spec's wait must last at
/// least this long, so `--wait-seconds` refuses less.
const API_TIMEOUT_SECONDS: u64 = 10;

/// API Gateway gives up at 29 seconds, so 60 leaves room.
const DEFAULT_WAIT_SECONDS: u64 = 60;

#[derive(Debug, Clone)]
pub struct Args {
    pub mode: Mode,
    pub execute: bool,
    pub wait_seconds: u64,
}

/// The arguments after the program's name.
pub fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut positional = Vec::new();
    let mut execute = false;
    let mut wait_seconds = DEFAULT_WAIT_SECONDS;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--execute" => execute = true,
            "--wait-seconds" => {
                wait_seconds = rest
                    .next()
                    .and_then(|n| n.parse().ok())
                    .ok_or("--wait-seconds takes a whole number")?;
                if wait_seconds < API_TIMEOUT_SECONDS {
                    return Err(format!(
                        "--wait-seconds must be at least the API Lambda's timeout, {API_TIMEOUT_SECONDS}"
                    ));
                }
            }
            flag if flag.starts_with("--") => return Err(format!("unknown option {flag}")),
            _ => positional.push(arg.as_str()),
        }
    }
    let mode = match positional.as_slice() {
        ["id", id] => Mode::One(AgentId::parse(id).ok_or("an ID is 12 characters of a-z and 2-7")?),
        ["window", from, to] => {
            if !is_rfc3339_second(from) || !is_rfc3339_second(to) {
                return Err("times look like 2026-11-05T12:00:00Z".to_owned());
            }
            if from >= to {
                return Err("FROM must be before TO".to_owned());
            }
            Mode::Window {
                from: (*from).to_owned(),
                to: (*to).to_owned(),
            }
        }
        _ => return Err(USAGE.to_owned()),
    };
    Ok(Args {
        mode,
        execute,
        wait_seconds,
    })
}
