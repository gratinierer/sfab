//! The kill switch: `.stories/2-identity-api/spec.md` "Kill switch". One full stop, whatever
//! triggers it, against [`Controls`], the narrow set of AWS calls it needs, so the rules here are
//! tested without AWS.

use std::future::Future;

/// The steps of a stop, in the order they run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Step {
    /// The CloudFront WAF's default action is Block.
    Waf,
    /// The API Lambda's reserved concurrency is 0. This is what "a stop is in force" means.
    ApiLambda,
    /// `disableExecuteApiEndpoint` is set on the REST API, and the stage is deployed with it.
    ExecuteApi,
    /// The daily world job's reserved concurrency is 0, because its cost grows with what's
    /// stored, not with traffic, so it would otherwise keep spending during a stop.
    WorldJob,
}

pub const STEPS: [Step; 4] = [Step::Waf, Step::ApiLambda, Step::ExecuteApi, Step::WorldJob];

/// Why the function was invoked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The 5-minute schedule.
    DailyLimitCheck,
    /// The 12:00 UTC schedule.
    SpendCheck,
    /// Any message on the stop topic, which only the stop budget's 100% alert publishes to.
    StopTopic,
}

impl Event {
    /// The schedules send a fixed `{"check": ...}`, which the CDK sets; SNS sends `Records`.
    pub fn parse(event: &serde_json::Value) -> Result<Event, Failure> {
        let check = event.get("check").and_then(|c| c.as_str());
        let source = event
            .pointer("/Records/0/EventSource")
            .and_then(|s| s.as_str());
        match (check, source) {
            (Some("daily-limit"), None) => Ok(Event::DailyLimitCheck),
            (Some("spend"), None) => Ok(Event::SpendCheck),
            (None, Some("aws:sns")) => Ok(Event::StopTopic),
            _ => Err(Failure(format!("unknown event: {event}"))),
        }
    }
}

/// The CloudFront web ACL, from its CloudFormation `Ref`, `name|id|scope`, which the CDK passes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebAclRef {
    pub name: String,
    pub id: String,
}

impl WebAclRef {
    pub fn parse(r: &str) -> Option<WebAclRef> {
        let mut parts = r.split('|');
        match (parts.next(), parts.next(), parts.next(), parts.next()) {
            (Some(name), Some(id), Some(_scope), None) if !name.is_empty() && !id.is_empty() => {
                Some(WebAclRef {
                    name: name.to_owned(),
                    id: id.to_owned(),
                })
            }
            _ => None,
        }
    }
}

/// The web ACL's description as the WAF step sends it back: `None` for none or an empty one,
/// since `GetWebACL` returns an ACL with no description as `""`, which `UpdateWebACL` rejects
/// (`.stories/2-identity-api/` issue 21).
pub fn kept_description(description: Option<String>) -> Option<String> {
    description.filter(|d| !d.is_empty())
}

/// Where the day's count starts: midnight UTC of the day holding `now`, both in Unix seconds.
pub fn utc_midnight(now: i64) -> i64 {
    now - now.rem_euclid(86_400)
}

/// The statistic of API Gateway's `Count` that gives the number of calls, as AWS documents it
/// (`docs/aws-facts.md`, the row on API Gateway `Count`).
pub const COUNT_STATISTIC: &str = "SampleCount";

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// API Gateway calls per UTC day; more than this stops the API.
    pub daily_calls: u64,
}

/// The stop budget's actual spend this month and its limit, in USD.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spend {
    pub actual: f64,
    pub limit: f64,
}

/// Something that went wrong, as the errors alarm's log line will show it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure(pub String);

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Failure {}

/// What started a stop, as the owner's email names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    DailyLimit,
    BudgetAlert,
    DailySpendCheck,
}

impl Trigger {
    pub fn name(self) -> &'static str {
        match self {
            Trigger::DailyLimit => "daily limit",
            Trigger::BudgetAlert => "budget alert",
            Trigger::DailySpendCheck => "daily spend check",
        }
    }
}

/// The AWS calls the kill switch makes. Each must be safe to repeat.
pub trait Controls {
    /// Whether a step is already in place.
    fn in_place(&self, step: Step) -> impl Future<Output = Result<bool, Failure>>;
    /// Puts a step in place.
    fn apply(&self, step: Step) -> impl Future<Output = Result<(), Failure>>;
    /// API Gateway's `Count` since midnight UTC.
    fn calls_today(&self) -> impl Future<Output = Result<f64, Failure>>;
    /// The stop budget, read through the Budgets API.
    fn spend(&self) -> impl Future<Output = Result<Spend, Failure>>;
    /// Publishes to the alerts topic, which the owner's email subscribes to.
    fn email(&self, subject: &str, body: &str) -> impl Future<Output = Result<(), Failure>>;
}

/// Where the restart list lives, which the stop email points to.
pub const RESTART_DOC: &str = "docs/restart-after-a-stop.md";

/// Handles one invocation. With a stop in force, it makes sure every step is in place. Otherwise
/// it stops the API when the event calls for it. An `Err` is a function error, which fires the
/// errors alarm: a step that failed, so the stop is incomplete, or a read that failed, so nothing
/// was decided.
pub async fn handle<C: Controls>(c: &C, event: Event, limits: Limits) -> Result<(), Failure> {
    let mut failures = Vec::new();
    // Unknown counts as not in force: a stop then repeats every step and emails, which is safe.
    let in_force = match c.in_place(Step::ApiLambda).await {
        Ok(in_force) => in_force,
        Err(e) => {
            failures.push(e);
            false
        }
    };

    let trigger = if in_force {
        None
    } else {
        match decide(c, event, limits).await {
            Ok(trigger) => trigger,
            Err(e) => {
                failures.push(e);
                None
            }
        }
    };

    if in_force || trigger.is_some() {
        put_in_place(c, in_force, &mut failures).await;
    }
    if let Some(trigger) = trigger
        && let Err(e) = c.email(&subject(trigger), &body(trigger)).await
    {
        failures.push(e);
    }

    if failures.is_empty() {
        Ok(())
    } else {
        let all: Vec<String> = failures.into_iter().map(|f| f.0).collect();
        Err(Failure(all.join("; ")))
    }
}

/// Whether this event starts a stop, and if so why.
async fn decide<C: Controls>(
    c: &C,
    event: Event,
    limits: Limits,
) -> Result<Option<Trigger>, Failure> {
    Ok(match event {
        Event::StopTopic => Some(Trigger::BudgetAlert),
        Event::DailyLimitCheck => {
            let calls = c.calls_today().await?;
            (calls > limits.daily_calls as f64).then_some(Trigger::DailyLimit)
        }
        Event::SpendCheck => {
            let spend = c.spend().await?;
            (spend.actual >= spend.limit).then_some(Trigger::DailySpendCheck)
        }
    })
}

/// Applies each step that isn't in place. A failure doesn't skip the others.
async fn put_in_place<C: Controls>(c: &C, in_force: bool, failures: &mut Vec<Failure>) {
    for step in STEPS {
        let needed = match step {
            Step::ApiLambda => !in_force,
            _ => !matches!(c.in_place(step).await, Ok(true)),
        };
        if needed && let Err(e) = c.apply(step).await {
            failures.push(Failure(format!("{step:?}: {e}")));
        }
    }
}

fn subject(trigger: Trigger) -> String {
    format!("sfab: the API is stopped ({})", trigger.name())
}

fn body(trigger: Trigger) -> String {
    format!(
        "The kill switch stopped the API. Trigger: {}.\n\n\
         It stays down until you restart it by hand. Look at what happened first, then follow \
         the restart list in {RESTART_DOC}.\n\n\
         If this email is followed by an errors alarm for the kill switch, the stop is \
         incomplete, and the next 5-minute check retries the missing steps.\n",
        trigger.name()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_day_starts_at_midnight_utc() {
        let midnight = 1_793_836_800; // 2026-11-05T00:00:00Z
        assert_eq!(utc_midnight(midnight), midnight);
        assert_eq!(utc_midnight(midnight + 86_399), midnight);
        assert_eq!(utc_midnight(midnight - 1), midnight - 86_400);
    }

    #[test]
    fn a_web_acl_ref_is_name_id_and_scope() {
        assert_eq!(
            WebAclRef::parse("sfab-acl|0123-abcd|CLOUDFRONT"),
            Some(WebAclRef {
                name: "sfab-acl".into(),
                id: "0123-abcd".into()
            })
        );
        for bad in [
            "",
            "sfab-acl",
            "sfab-acl|0123-abcd",
            "|0123|CLOUDFRONT",
            "a|b|c|d",
        ] {
            assert_eq!(WebAclRef::parse(bad), None, "{bad:?}");
        }
    }
}
