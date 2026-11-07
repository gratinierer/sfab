//! The kill switch's unit tests, from `.stories/2-identity-api/spec.md` "Testing", against a fake
//! of the AWS calls it makes.

use std::cell::RefCell;
use std::collections::BTreeSet;

use serde_json::json;

use sfab_killswitch::{Controls, Event, Failure, Limits, Spend, Step, handle};

const LIMITS: Limits = Limits {
    daily_calls: 100_000,
};

/// AWS as the kill switch sees it: which steps are in place, the day's calls, the stop budget,
/// and what failed.
#[derive(Default)]
struct Fake {
    in_place: RefCell<BTreeSet<Step>>,
    calls_today: Option<f64>,
    spend: Option<Spend>,
    failing: BTreeSet<Step>,
    applied: RefCell<Vec<Step>>,
    emails: RefCell<Vec<(String, String)>>,
}

impl Fake {
    fn running(calls_today: f64) -> Self {
        Fake {
            calls_today: Some(calls_today),
            spend: Some(Spend {
                actual: 1.0,
                limit: 35.0,
            }),
            ..Fake::default()
        }
    }

    fn stopped() -> Self {
        let fake = Fake::running(0.0);
        fake.in_place
            .borrow_mut()
            .extend([Step::Waf, Step::ApiLambda, Step::ExecuteApi]);
        fake
    }

    fn applied(&self) -> Vec<Step> {
        self.applied.borrow().clone()
    }

    fn emails(&self) -> Vec<(String, String)> {
        self.emails.borrow().clone()
    }
}

impl Controls for Fake {
    async fn in_place(&self, step: Step) -> Result<bool, Failure> {
        Ok(self.in_place.borrow().contains(&step))
    }

    async fn apply(&self, step: Step) -> Result<(), Failure> {
        self.applied.borrow_mut().push(step);
        if self.failing.contains(&step) {
            return Err(Failure(format!("{step:?} failed")));
        }
        self.in_place.borrow_mut().insert(step);
        Ok(())
    }

    async fn calls_today(&self) -> Result<f64, Failure> {
        self.calls_today
            .ok_or_else(|| Failure("metric read failed".into()))
    }

    async fn spend(&self) -> Result<Spend, Failure> {
        self.spend
            .ok_or_else(|| Failure("budget read failed".into()))
    }

    async fn email(&self, subject: &str, body: &str) -> Result<(), Failure> {
        self.emails
            .borrow_mut()
            .push((subject.to_owned(), body.to_owned()));
        Ok(())
    }
}

const ALL: [Step; 3] = [Step::Waf, Step::ApiLambda, Step::ExecuteApi];

#[tokio::test]
async fn over_the_daily_limit_stops_the_api_and_emails_once_naming_the_daily_limit() {
    let aws = Fake::running(100_001.0);

    handle(&aws, Event::DailyLimitCheck, LIMITS).await.unwrap();

    assert_eq!(aws.applied(), ALL);
    let emails = aws.emails();
    assert_eq!(emails.len(), 1);
    assert!(emails[0].0.contains("daily limit"), "{emails:?}");
}

#[tokio::test]
async fn at_the_daily_limit_the_api_keeps_running() {
    let aws = Fake::running(100_000.0);

    handle(&aws, Event::DailyLimitCheck, LIMITS).await.unwrap();

    assert_eq!(aws.applied(), []);
    assert_eq!(aws.emails(), []);
}

#[tokio::test]
async fn spend_at_the_budget_limit_stops_the_api_and_emails_naming_the_spend_check() {
    let mut aws = Fake::running(0.0);
    aws.spend = Some(Spend {
        actual: 35.0,
        limit: 35.0,
    });

    handle(&aws, Event::SpendCheck, LIMITS).await.unwrap();

    assert_eq!(aws.applied(), ALL);
    let emails = aws.emails();
    assert_eq!(emails.len(), 1);
    assert!(emails[0].0.contains("daily spend check"), "{emails:?}");
}

#[tokio::test]
async fn a_message_on_the_stop_topic_stops_the_api_and_emails_naming_the_budget_alert() {
    let aws = Fake::running(0.0);

    handle(&aws, Event::StopTopic, LIMITS).await.unwrap();

    assert_eq!(aws.applied(), ALL);
    let emails = aws.emails();
    assert_eq!(emails.len(), 1);
    assert!(emails[0].0.contains("budget alert"), "{emails:?}");
}

#[tokio::test]
async fn a_second_trigger_during_a_stop_sends_no_second_email() {
    let aws = Fake::running(100_001.0);
    handle(&aws, Event::DailyLimitCheck, LIMITS).await.unwrap();

    handle(&aws, Event::StopTopic, LIMITS).await.unwrap();
    handle(&aws, Event::DailyLimitCheck, LIMITS).await.unwrap();

    assert_eq!(aws.emails().len(), 1);
    assert_eq!(aws.applied(), ALL, "every step was already in place");
}

#[tokio::test]
async fn with_a_stop_in_force_a_check_repeats_only_the_missing_step() {
    for missing in [Step::Waf, Step::ExecuteApi] {
        let aws = Fake::stopped();
        aws.in_place.borrow_mut().remove(&missing);

        handle(&aws, Event::DailyLimitCheck, LIMITS).await.unwrap();

        assert_eq!(aws.applied(), [missing]);
        assert_eq!(aws.emails(), []);
    }
}

#[tokio::test]
async fn with_a_stop_in_force_a_check_needs_no_metric_read() {
    let mut aws = Fake::stopped();
    aws.calls_today = None;
    aws.spend = None;

    handle(&aws, Event::DailyLimitCheck, LIMITS).await.unwrap();
    handle(&aws, Event::SpendCheck, LIMITS).await.unwrap();

    assert_eq!(aws.applied(), []);
}

#[tokio::test]
async fn a_failed_step_still_attempts_the_others_and_is_a_function_error() {
    for failing in ALL {
        let mut aws = Fake::running(100_001.0);
        aws.failing.insert(failing);

        let result = handle(&aws, Event::DailyLimitCheck, LIMITS).await;

        assert!(result.is_err(), "{failing:?}");
        assert_eq!(aws.applied(), ALL, "{failing:?}");
        assert_eq!(aws.emails().len(), 1, "{failing:?}");
    }
}

#[tokio::test]
async fn the_next_check_retries_a_failed_step() {
    let mut aws = Fake::running(100_001.0);
    aws.failing.insert(Step::ExecuteApi);
    assert!(handle(&aws, Event::DailyLimitCheck, LIMITS).await.is_err());
    aws.failing.clear();
    aws.applied.borrow_mut().clear();

    handle(&aws, Event::DailyLimitCheck, LIMITS).await.unwrap();

    assert_eq!(aws.applied(), [Step::ExecuteApi]);
    assert_eq!(aws.emails().len(), 1);
}

#[tokio::test]
async fn a_failed_metric_read_is_an_error_that_does_not_stop_the_api() {
    let mut aws = Fake::running(0.0);
    aws.calls_today = None;

    let result = handle(&aws, Event::DailyLimitCheck, LIMITS).await;

    assert!(result.is_err());
    assert_eq!(aws.applied(), []);
    assert_eq!(aws.emails(), []);
}

#[tokio::test]
async fn a_failed_budget_read_is_an_error_that_does_not_stop_the_api() {
    let mut aws = Fake::running(0.0);
    aws.spend = None;

    let result = handle(&aws, Event::SpendCheck, LIMITS).await;

    assert!(result.is_err());
    assert_eq!(aws.applied(), []);
    assert_eq!(aws.emails(), []);
}

#[test]
fn the_schedules_and_the_stop_topic_are_told_apart() {
    assert_eq!(
        Event::parse(&json!({"check": "daily-limit"})),
        Ok(Event::DailyLimitCheck)
    );
    assert_eq!(
        Event::parse(&json!({"check": "spend"})),
        Ok(Event::SpendCheck)
    );
    // The shape SNS delivers; any message on the stop topic stops the API.
    let sns = json!({"Records": [{"EventSource": "aws:sns", "Sns": {"Message": "anything"}}]});
    assert_eq!(Event::parse(&sns), Ok(Event::StopTopic));
}

#[test]
fn an_unknown_event_is_refused() {
    for event in [
        json!({}),
        json!({"check": "other"}),
        json!({"Records": []}),
        json!({"Records": [{"EventSource": "aws:s3"}]}),
    ] {
        assert!(Event::parse(&event).is_err(), "{event}");
    }
}

#[tokio::test]
async fn the_email_says_how_to_restart() {
    let aws = Fake::running(100_001.0);

    handle(&aws, Event::DailyLimitCheck, LIMITS).await.unwrap();

    let body = &aws.emails()[0].1;
    assert!(body.contains("docs/restart-after-a-stop.md"), "{body}");
}

#[tokio::test]
async fn spend_under_the_budget_limit_keeps_the_api_running() {
    let mut aws = Fake::running(0.0);
    aws.spend = Some(Spend {
        actual: 34.99,
        limit: 35.0,
    });

    handle(&aws, Event::SpendCheck, LIMITS).await.unwrap();

    assert_eq!(aws.applied(), []);
    assert_eq!(aws.emails(), []);
}
