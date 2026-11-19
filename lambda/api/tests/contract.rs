//! Contract tests: the handlers against `openapi/sfab.yaml`. Every status a handler answers must
//! be declared for its operation, with the declared content type, a body that fits the schema,
//! and `Cache-Control: no-store`. Every operation in the document must be exercised.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, Response};
use regex::Regex;
use saphyr::{LoadableYamlNode, Scalar, Yaml};
use serde_json::{Value, json};
use sfab_core::credentials::caller_of;
use sfab_core::id::AgentId;
use sfab_core::proof;
use sfab_core::storage::{Bucket, MemoryStore, Store};
use sfab_genesis::testing as genesis;
use tower::ServiceExt;

const BITS: u32 = 8;
const NOW: i64 = 1_793_836_800;

fn now() -> i64 {
    NOW
}

fn spec() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../openapi/sfab.yaml");
    let text = std::fs::read_to_string(path).unwrap();
    to_json(&Yaml::load_from_str(&text).unwrap()[0])
}

/// YAML to JSON, with aliases already expanded by the loader.
fn to_json(yaml: &Yaml) -> Value {
    match yaml {
        Yaml::Value(Scalar::Null) => Value::Null,
        Yaml::Value(Scalar::Boolean(b)) => Value::Bool(*b),
        Yaml::Value(Scalar::Integer(i)) => json!(i),
        Yaml::Value(Scalar::FloatingPoint(f)) => json!(f.into_inner()),
        Yaml::Value(Scalar::String(s)) => Value::String(s.to_string()),
        Yaml::Sequence(items) => Value::Array(items.iter().map(to_json).collect()),
        Yaml::Mapping(map) => Value::Object(
            map.iter()
                .map(|(k, v)| {
                    let key = match to_json(k) {
                        Value::String(s) => s,
                        other => other.to_string(),
                    };
                    (key, to_json(v))
                })
                .collect(),
        ),
        Yaml::Tagged(_, inner) => to_json(inner),
        other => panic!("unexpected YAML node {other:?}"),
    }
}

/// Follows a local `$ref`.
fn resolve<'a>(spec: &'a Value, node: &'a Value) -> &'a Value {
    match node.get("$ref").and_then(Value::as_str) {
        Some(r) => {
            let pointer = r.strip_prefix('#').expect("a local $ref");
            resolve(
                spec,
                spec.pointer(pointer)
                    .unwrap_or_else(|| panic!("{r} resolves")),
            )
        }
        None => node,
    }
}

/// The JSON Schema subset `sfab.yaml` uses, with every failure as a path and a reason.
fn validate(spec: &Value, schema: &Value, value: &Value, at: &str, errors: &mut Vec<String>) {
    let schema = resolve(spec, schema);
    if value.is_null() && schema.get("nullable") == Some(&Value::Bool(true)) {
        return;
    }
    if let Some(max) = schema.get("maxItems").and_then(Value::as_u64)
        && value.as_array().is_some_and(|a| a.len() as u64 > max)
    {
        errors.push(format!("{at}: more than {max} items"));
    }
    if let (Some(items), Some(values)) = (schema.get("items"), value.as_array()) {
        for (i, v) in values.iter().enumerate() {
            validate(spec, items, v, &format!("{at}[{i}]"), errors);
        }
    }
    if schema.get("type").and_then(Value::as_str) == Some("boolean") && !value.is_boolean() {
        errors.push(format!("{at}: not a boolean"));
    }
    if let Some(all) = schema.get("allOf").and_then(Value::as_array) {
        for part in all {
            validate(spec, part, value, at, errors);
        }
    }
    match schema.get("type").and_then(Value::as_str) {
        Some("object") => {
            let Some(obj) = value.as_object() else {
                return errors.push(format!("{at}: not an object"));
            };
            let props = schema.get("properties").and_then(Value::as_object);
            for key in schema
                .get("required")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if !obj.contains_key(key.as_str().unwrap()) {
                    errors.push(format!("{at}: lacks required {key}"));
                }
            }
            for (key, v) in obj {
                match props.and_then(|p| p.get(key)) {
                    Some(s) => validate(spec, s, v, &format!("{at}.{key}"), errors),
                    None if schema.get("additionalProperties") == Some(&Value::Bool(false)) => {
                        errors.push(format!("{at}: unexpected {key}"));
                    }
                    None => {}
                }
            }
        }
        Some("string") => {
            let Some(s) = value.as_str() else {
                return errors.push(format!("{at}: not a string"));
            };
            let len = s.chars().count() as u64;
            if let Some(min) = schema.get("minLength").and_then(Value::as_u64)
                && len < min
            {
                errors.push(format!("{at}: shorter than {min}"));
            }
            if let Some(max) = schema.get("maxLength").and_then(Value::as_u64)
                && len > max
            {
                errors.push(format!("{at}: longer than {max}"));
            }
            if let Some(p) = schema.get("pattern").and_then(Value::as_str)
                && !Regex::new(p).unwrap().is_match(s)
            {
                errors.push(format!("{at}: doesn't match {p}"));
            }
        }
        Some("integer") if !value.is_i64() && !value.is_u64() => {
            errors.push(format!("{at}: not an integer"));
        }
        Some("array") if !value.is_array() => {
            errors.push(format!("{at}: not an array"));
        }
        _ => {}
    }
}

/// The path in `sfab.yaml` that a request's path, with the stage and maybe a query, matches.
/// A fixed segment wins over a `{parameter}`, as at the gateway.
fn template(spec: &Value, path: &str) -> String {
    let path = path.split('?').next().unwrap();
    let route = path.strip_prefix("/v1").unwrap();
    let sent: Vec<&str> = route.split('/').collect();
    let matches = |t: &str| {
        let segments: Vec<&str> = t.split('/').collect();
        segments.len() == sent.len()
            && segments
                .iter()
                .zip(&sent)
                .all(|(t, s)| t == s || t.starts_with('{'))
    };
    let mut found: Vec<&String> = spec["paths"]
        .as_object()
        .unwrap()
        .keys()
        .filter(|t| matches(t))
        .collect();
    found.sort_by_key(|t| t.matches('{').count());
    found
        .first()
        .unwrap_or_else(|| panic!("{route} isn't in sfab.yaml"))
        .to_string()
}

/// One request, and which operation it exercises.
struct Case {
    operation: &'static str,
    method: &'static str,
    path: String,
    authorization: Option<String>,
    proof: Proof,
    body: Vec<u8>,
    /// CloudFront's `CloudFront-Viewer-Address`.
    viewer: Option<&'static str>,
}

enum Proof {
    None,
    Valid,
}

/// The app with the in-memory fake, and the operations each response was checked for.
struct Harness {
    spec: Value,
    app: Router,
    store: Arc<MemoryStore>,
    seen: Mutex<BTreeSet<(String, u16)>>,
}

impl Harness {
    fn new() -> Self {
        let store = Arc::new(MemoryStore::new());
        Self {
            spec: spec(),
            app: sfab_api::app(store.clone(), BITS, genesis::ip_rule(), now, here()),
            store,
            seen: Mutex::new(BTreeSet::new()),
        }
    }

    async fn send(&self, case: Case) -> (u16, Value) {
        let mut request = Request::builder().method(case.method).uri(&case.path);
        if let Some(auth) = &case.authorization {
            request = request.header("authorization", auth);
        }
        if let Some(viewer) = case.viewer {
            request = request.header("cloudfront-viewer-address", viewer);
        }
        if let Proof::Valid = case.proof {
            let caller = caller_of(case.authorization.as_deref()).unwrap();
            let r = proof::Request {
                method: case.method,
                path: &case.path,
                caller: &caller,
                body: &case.body,
            };
            request = request.header("proof", proof::solve(&r, NOW, BITS));
        }
        let response = self
            .app
            .clone()
            .oneshot(request.body(Body::from(case.body.clone())).unwrap())
            .await
            .unwrap();
        self.check(case.operation, case.method, &case.path, response)
            .await
    }

    /// A fresh `--no-ip-limit` genesis code, so births need no viewer address.
    async fn code(&self) -> String {
        genesis::seed_exempt_code(&*self.store).await
    }

    /// Checks one response against its operation, and returns its status and JSON body.
    async fn check(
        &self,
        operation: &str,
        method: &str,
        path: &str,
        response: Response<Body>,
    ) -> (u16, Value) {
        let status = response.status().as_u16();
        let headers = response.headers().clone();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let route = template(&self.spec, path);
        let op = &self.spec["paths"][&route][method.to_lowercase()];
        assert_eq!(op["operationId"], operation, "{method} {route}");
        let responses = &op["responses"];
        let declared = responses
            .get(status.to_string())
            .or_else(|| responses.get("default"))
            .unwrap_or_else(|| panic!("{operation} answers {status}, which isn't declared"));
        let declared = resolve(&self.spec, declared);
        assert_eq!(headers["cache-control"], "no-store", "{operation} {status}");
        let content_type = headers["content-type"].to_str().unwrap();
        let media = declared["content"].get(content_type).unwrap_or_else(|| {
            panic!("{operation} {status} sends {content_type}, which isn't declared")
        });
        let body = if content_type == "application/yaml" {
            Value::String(String::from_utf8(bytes.to_vec()).unwrap())
        } else {
            serde_json::from_slice(&bytes).unwrap()
        };
        let mut errors = Vec::new();
        validate(&self.spec, &media["schema"], &body, operation, &mut errors);
        assert!(errors.is_empty(), "{operation} {status}: {errors:?}");
        if status >= 400 {
            assert_eq!(
                body["status"], status,
                "{operation}: a problem names its status"
            );
        }
        self.seen
            .lock()
            .unwrap()
            .insert((operation.to_owned(), status));
        (status, body)
    }
}

fn here() -> sfab_federation::Here {
    sfab_federation::Here {
        domain: "sfab.example.org".into(),
        contact: "abuse@sfab.example.org".into(),
    }
}

fn federation_request(domain: &str, proof: Proof) -> Case {
    Case {
        operation: "requestFederation",
        method: "POST",
        path: "/v1/federation/requests".into(),
        authorization: None,
        proof,
        body: serde_json::to_vec(&json!({ "domain": domain })).unwrap(),
        viewer: None,
    }
}

fn birth(name: &str, genesis_code: &str, proof: Proof) -> Case {
    Case {
        operation: "birth",
        method: "POST",
        path: "/v1/identity/birth".into(),
        authorization: None,
        proof,
        body: serde_json::to_vec(&json!({ "name": name, "genesis_code": genesis_code })).unwrap(),
        viewer: None,
    }
}

fn bearer(id: &str, secret: &str) -> Option<String> {
    Some(format!("Bearer {id}.{secret}"))
}

fn evolve(auth: Option<String>, proof: Proof, whoiam: &str) -> Case {
    Case {
        operation: "evolve",
        method: "PUT",
        path: "/v1/identity/evolve".into(),
        authorization: auth,
        proof,
        body: serde_json::to_vec(&json!({ "whoiam": whoiam })).unwrap(),
        viewer: None,
    }
}

fn read_self(auth: Option<String>) -> Case {
    Case {
        operation: "self",
        method: "GET",
        path: "/v1/identity/self".into(),
        authorization: auth,
        proof: Proof::None,
        body: Vec::new(),
        viewer: None,
    }
}

fn get(operation: &'static str, path: &str, auth: Option<String>) -> Case {
    Case {
        operation,
        method: "GET",
        path: path.to_owned(),
        authorization: auth,
        proof: Proof::None,
        body: Vec::new(),
        viewer: None,
    }
}

/// A write, with a valid proof unless `proof` says otherwise.
fn write(
    operation: &'static str,
    method: &'static str,
    path: &str,
    auth: Option<String>,
    proof: Proof,
    body: &[u8],
) -> Case {
    Case {
        operation,
        method,
        path: path.to_owned(),
        authorization: auth,
        proof,
        body: body.to_vec(),
        viewer: None,
    }
}

/// The Academy's routes, with Finch teaching, Gull and Finch as students, and the daily job run
/// part way through, so its routes answer both before and after.
async fn academy(h: &Harness, auth: Option<String>, gauth: Option<String>, fid: &str, gid: &str) {
    let courses = "/v1/academy/courses";
    let body = |title: &str| {
        serde_json::to_vec(&json!({
            "title": title,
            "pass": 50,
            "tasks": [
                {"kind": "check", "text": "Tie a bowline", "answer": "done"},
                {"kind": "work", "text": "Describe it"},
            ],
        }))
        .unwrap()
    };
    // Before the daily job's first run.
    let (status, built) = h.send(get("readCatalogue", courses, None)).await;
    assert_eq!(
        (status, built["type"].clone()),
        (404, json!("not-built-yet"))
    );
    // publishCourse: 201, 200 for the same proof, 400, 401, 409.
    let publish = |a, p, b: &[u8]| write("publishCourse", "POST", courses, a, p, b);
    let (status, course) = h
        .send(publish(auth.clone(), Proof::Valid, &body("Knots")))
        .await;
    assert_eq!(status, 201);
    let again = publish(auth.clone(), Proof::Valid, &body("Knots"));
    assert_eq!(h.send(again).await, (200, course.clone()));
    let cid = course["course"].as_str().unwrap().to_owned();
    let none = publish(auth.clone(), Proof::None, &body("x"));
    assert_eq!(h.send(none).await.0, 400);
    let long = publish(auth.clone(), Proof::Valid, &body(&"x".repeat(129)));
    assert_eq!(h.send(long).await.0, 400);
    assert_eq!(h.send(publish(None, Proof::Valid, &body("x"))).await.0, 401);
    for i in 2..=5 {
        let more = publish(auth.clone(), Proof::Valid, &body(&format!("Course {i}")));
        assert_eq!(h.send(more).await.0, 201);
    }
    let (status, full) = h
        .send(publish(auth.clone(), Proof::Valid, &body("Sixth")))
        .await;
    assert_eq!((status, full["type"].clone()), (409, json!("list-full")));
    let schemas = &h.spec["components"]["schemas"];
    let limits = [
        ("CourseTitle", sfab_academy::MAX_TITLE_CHARS),
        ("Preparation", sfab_academy::MAX_PREPARATION_BYTES),
        ("TaskText", sfab_academy::MAX_TASK_BYTES),
        ("AnswerText", sfab_academy::MAX_ANSWER_BYTES),
        ("MarkComment", sfab_academy::MAX_COMMENT_BYTES),
    ];
    for (schema, limit) in limits {
        assert_eq!(schemas[schema]["maxLength"], json!(limit), "{schema}");
    }
    let request = &schemas["CourseRequest"]["properties"];
    assert_eq!(request["tasks"]["maxItems"], json!(sfab_academy::MAX_TASKS));
    assert_eq!(request["set_size"]["maximum"], json!(sfab_academy::MAX_SET));

    let one = format!("{courses}/{cid}");
    // readCourse: 200, 400, 404.
    let (status, page) = h.send(get("readCourse", &one, None)).await;
    assert_eq!((status, page["enrolled"].clone()), (200, Value::Null));
    assert_eq!(
        h.send(get("readCourse", &format!("{courses}/x"), None))
            .await
            .0,
        400
    );
    let unknown = format!("{courses}/aaaaaaaaaaaa");
    assert_eq!(h.send(get("readCourse", &unknown, None)).await.0, 404);
    // editCourse: 200, 400, 401, 403, 404.
    let edit = |path: &str, a, p, b: &[u8]| write("editCourse", "PUT", path, a, p, b);
    let (status, edited) = h
        .send(edit(
            &one,
            auth.clone(),
            Proof::Valid,
            &body("Better knots"),
        ))
        .await;
    assert_eq!(
        (status, edited["title"].clone()),
        (200, json!("Better knots"))
    );
    assert_eq!(
        h.send(edit(&one, auth.clone(), Proof::None, &body("x")))
            .await
            .0,
        400
    );
    assert_eq!(
        h.send(edit(&one, None, Proof::Valid, &body("x"))).await.0,
        401
    );
    let theirs = edit(&one, gauth.clone(), Proof::Valid, &body("x"));
    assert_eq!(h.send(theirs).await.0, 403);
    let missing = edit(&unknown, auth.clone(), Proof::Valid, &body("x"));
    assert_eq!(h.send(missing).await.0, 404);
    // readTasks: 200, 400, 401, 403, 404.
    let tasks = format!("{one}/tasks");
    let (status, pool) = h.send(get("readTasks", &tasks, auth.clone())).await;
    assert_eq!(
        (status, pool["items"][1]["answer_hash"].clone()),
        (200, Value::Null)
    );
    let bad = format!("{tasks}?page=x");
    assert_eq!(h.send(get("readTasks", &bad, auth.clone())).await.0, 400);
    assert_eq!(h.send(get("readTasks", &tasks, None)).await.0, 401);
    assert_eq!(h.send(get("readTasks", &tasks, gauth.clone())).await.0, 403);
    let none = format!("{unknown}/tasks");
    assert_eq!(h.send(get("readTasks", &none, auth.clone())).await.0, 404);

    // enrol: 201, 200, 400, 401, 404; leaveCourse: 200, 400, 401.
    let enrolment = format!("{one}/enrolments/self");
    let enrol = |path: &str, a, p| write("enrol", "PUT", path, a, p, b"");
    for a in [&gauth, &auth] {
        assert_eq!(
            h.send(enrol(&enrolment, a.clone(), Proof::Valid)).await.0,
            201
        );
    }
    assert_eq!(
        h.send(enrol(&enrolment, gauth.clone(), Proof::Valid))
            .await
            .0,
        200
    );
    assert_eq!(
        h.send(enrol(&enrolment, gauth.clone(), Proof::None))
            .await
            .0,
        400
    );
    assert_eq!(h.send(enrol(&enrolment, None, Proof::Valid)).await.0, 401);
    let elsewhere = format!("{unknown}/enrolments/self");
    assert_eq!(
        h.send(enrol(&elsewhere, gauth.clone(), Proof::Valid))
            .await
            .0,
        404
    );
    let leave = |path: &str, a| Case {
        operation: "leaveCourse",
        method: "DELETE",
        path: path.to_owned(),
        authorization: a,
        proof: Proof::None,
        body: Vec::new(),
        viewer: None,
    };
    let (status, left) = h.send(leave(&enrolment, auth.clone())).await;
    assert_eq!((status, left["id"].clone()), (200, json!(fid)));
    let bad = format!("{courses}/x/enrolments/self");
    assert_eq!(h.send(leave(&bad, auth.clone())).await.0, 400);
    assert_eq!(h.send(leave(&enrolment, None)).await.0, 401);

    // startAttempt: 201, 200, 400, 401, 403, 404, then 409 after submitting.
    let attempts = format!("{one}/attempts");
    let start = |path: &str, a, p| write("startAttempt", "POST", path, a, p, b"");
    let (status, attempt) = h.send(start(&attempts, gauth.clone(), Proof::Valid)).await;
    assert_eq!(status, 201);
    assert!(!attempt.to_string().contains("hash"));
    let aid = attempt["attempt"].as_str().unwrap().to_owned();
    assert_eq!(
        h.send(start(&attempts, gauth.clone(), Proof::Valid))
            .await
            .0,
        200
    );
    assert_eq!(
        h.send(start(&attempts, gauth.clone(), Proof::None)).await.0,
        400
    );
    assert_eq!(h.send(start(&attempts, None, Proof::Valid)).await.0, 401);
    let (status, refused) = h.send(start(&attempts, auth.clone(), Proof::Valid)).await;
    assert_eq!(
        (status, refused["type"].clone()),
        (403, json!("not-enrolled"))
    );
    let nowhere = format!("{unknown}/attempts");
    assert_eq!(
        h.send(start(&nowhere, gauth.clone(), Proof::Valid)).await.0,
        404
    );
    // readAttempt: 200, 400, 401, 403, 404.
    let mine = format!("{attempts}/{aid}");
    assert_eq!(
        h.send(get("readAttempt", &mine, gauth.clone())).await.0,
        200
    );
    let bad = format!("{attempts}/nope");
    assert_eq!(h.send(get("readAttempt", &bad, gauth.clone())).await.0, 400);
    assert_eq!(h.send(get("readAttempt", &mine, None)).await.0, 401);
    let (status, refused) = h.send(get("readAttempt", &mine, auth.clone())).await;
    assert_eq!(
        (status, refused["type"].clone()),
        (403, json!("not-a-party"))
    );
    let later = format!("{attempts}/{gid}-2026-01-01");
    assert_eq!(
        h.send(get("readAttempt", &later, gauth.clone())).await.0,
        404
    );
    // handIn: 200, 400, 401, 403, 404, then 409 after submitting.
    let answer = |n: &str, a, p, text: &str| {
        let path = format!("{mine}/answers/{n}");
        let body = serde_json::to_vec(&json!({ "answer": text })).unwrap();
        write("handIn", "PUT", &path, a, p, &body)
    };
    for (n, text) in [("1", "done"), ("2", "Loop, then tuck.")] {
        let (status, handed) = h.send(answer(n, gauth.clone(), Proof::Valid, text)).await;
        assert_eq!(
            (status, handed["n"].clone()),
            (200, json!(n.parse::<u8>().unwrap()))
        );
    }
    assert_eq!(
        h.send(answer("1", gauth.clone(), Proof::None, "x")).await.0,
        400
    );
    let big = "x".repeat(sfab_academy::MAX_ANSWER_BYTES + 1);
    assert_eq!(
        h.send(answer("1", gauth.clone(), Proof::Valid, &big))
            .await
            .0,
        400
    );
    assert_eq!(h.send(answer("1", None, Proof::Valid, "x")).await.0, 401);
    assert_eq!(
        h.send(answer("1", auth.clone(), Proof::Valid, "x")).await.0,
        403
    );
    assert_eq!(
        h.send(answer("3", gauth.clone(), Proof::Valid, "x"))
            .await
            .0,
        404
    );
    // submitAttempt: 200, 400, 401, 403, 404.
    let submit = |path: &str, a, p| write("submitAttempt", "PUT", path, a, p, b"");
    let submission = format!("{mine}/submission");
    let (status, submitted) = h
        .send(submit(&submission, gauth.clone(), Proof::Valid))
        .await;
    assert_eq!(
        (status, submitted["status"].clone()),
        (200, json!("awaiting-mark"))
    );
    assert_eq!(
        h.send(submit(&submission, gauth.clone(), Proof::None))
            .await
            .0,
        400
    );
    assert_eq!(h.send(submit(&submission, None, Proof::Valid)).await.0, 401);
    assert_eq!(
        h.send(submit(&submission, auth.clone(), Proof::Valid))
            .await
            .0,
        403
    );
    let gone = format!("{later}/submission");
    assert_eq!(
        h.send(submit(&gone, gauth.clone(), Proof::Valid)).await.0,
        404
    );
    let (status, late) = h.send(answer("1", gauth.clone(), Proof::Valid, "y")).await;
    assert_eq!((status, late["type"].clone()), (409, json!("submitted")));
    let (status, soon) = h.send(start(&attempts, gauth.clone(), Proof::Valid)).await;
    assert_eq!((status, soon["type"].clone()), (409, json!("too-soon")));

    // readMarkingQueue: 200, 400, 401, 403, 404.
    let queue = format!("{one}/marking");
    let (status, waiting) = h.send(get("readMarkingQueue", &queue, auth.clone())).await;
    assert_eq!(
        (status, waiting["items"][0]["attempt"].clone()),
        (200, json!(aid))
    );
    let bad = format!("{queue}?page=x");
    assert_eq!(
        h.send(get("readMarkingQueue", &bad, auth.clone())).await.0,
        400
    );
    assert_eq!(h.send(get("readMarkingQueue", &queue, None)).await.0, 401);
    let (status, refused) = h.send(get("readMarkingQueue", &queue, gauth.clone())).await;
    assert_eq!(
        (status, refused["type"].clone()),
        (403, json!("not-a-graduate"))
    );
    let none = format!("{unknown}/marking");
    assert_eq!(
        h.send(get("readMarkingQueue", &none, auth.clone())).await.0,
        404
    );
    // readMarking: 200, 400, 401, 403, 404.
    let marking = format!("{queue}/{aid}");
    let (status, work) = h.send(get("readMarking", &marking, auth.clone())).await;
    assert_eq!(
        (status, work["tasks"][0]["correct"].clone()),
        (200, json!(true))
    );
    let bad = format!("{queue}/nope");
    assert_eq!(h.send(get("readMarking", &bad, auth.clone())).await.0, 400);
    assert_eq!(h.send(get("readMarking", &marking, None)).await.0, 401);
    assert_eq!(
        h.send(get("readMarking", &marking, gauth.clone())).await.0,
        403
    );
    let absent = format!("{queue}/{gid}-2026-01-01");
    assert_eq!(
        h.send(get("readMarking", &absent, auth.clone())).await.0,
        404
    );
    // markAttempt: 200, 400, 401, 403, 404.
    let mark = |path: &str, a, p, b: &Value| {
        write(
            "markAttempt",
            "PUT",
            path,
            a,
            p,
            &serde_json::to_vec(b).unwrap(),
        )
    };
    let accept = json!({"accepted": [2], "comment": "Good."});
    assert_eq!(
        h.send(mark(&marking, auth.clone(), Proof::None, &accept))
            .await
            .0,
        400
    );
    assert_eq!(
        h.send(mark(&marking, None, Proof::Valid, &accept)).await.0,
        401
    );
    assert_eq!(
        h.send(mark(&marking, gauth.clone(), Proof::Valid, &accept))
            .await
            .0,
        403
    );
    assert_eq!(
        h.send(mark(&absent, auth.clone(), Proof::Valid, &accept))
            .await
            .0,
        404
    );
    let (status, marked) = h
        .send(mark(&marking, auth.clone(), Proof::Valid, &accept))
        .await;
    assert_eq!((status, marked["status"].clone()), (200, json!("passed")));

    // readCertificate: 200, 400, 404; revokeCertificate: 403, 200, 400, 401, 404.
    let certificate = format!("exam-{cid}-{gid}");
    let read = format!("/v1/academy/certificates/{certificate}");
    let (status, shown) = h.send(get("readCertificate", &read, None)).await;
    assert_eq!((status, shown["marker"]["id"].clone()), (200, json!(fid)));
    let bad = "/v1/academy/certificates/nope";
    assert_eq!(h.send(get("readCertificate", bad, None)).await.0, 400);
    let missing = format!("/v1/academy/certificates/exam-{cid}-{fid}");
    assert_eq!(h.send(get("readCertificate", &missing, None)).await.0, 404);

    // The daily job's routes, built now.
    let residents = sfab_identity::residents(&*h.store).await.unwrap();
    let standing = std::collections::BTreeSet::new();
    sfab_academy::daily(&*h.store, &residents, &standing, NOW)
        .await
        .unwrap();
    // readCatalogue: 200, 400, 404 past the end.
    let (status, catalogue) = h.send(get("readCatalogue", courses, None)).await;
    assert_eq!(
        (status, catalogue["items"].as_array().unwrap().len()),
        (200, 5)
    );
    assert_eq!(
        h.send(get("readCatalogue", &format!("{courses}?page=x"), None))
            .await
            .0,
        400
    );
    assert_eq!(
        h.send(get("readCatalogue", &format!("{courses}?page=2"), None))
            .await
            .0,
        404
    );
    let (status, page) = h.send(get("readCourse", &one, None)).await;
    assert_eq!((status, page["graduates"].clone()), (200, json!(1)));
    // readEnrolled, readGraduates, readTeacherGraduates, readEnrolments, readCertificates:
    // 200, 400, 404 past the end; readTeacher: 200, 400.
    let teacher = format!("/v1/academy/teachers/{fid}");
    for (operation, path, first) in [
        ("readEnrolled", format!("{one}/enrolled"), json!(gid)),
        ("readGraduates", format!("{one}/graduates"), json!(gid)),
        (
            "readTeacherGraduates",
            format!("{teacher}/graduates"),
            json!(gid),
        ),
        (
            "readEnrolments",
            format!("/v1/identity/{gid}/enrolments"),
            json!(cid),
        ),
        (
            "readCertificates",
            format!("/v1/identity/{gid}/certificates"),
            json!(certificate),
        ),
    ] {
        let (status, list) = h.send(get(operation, &path, None)).await;
        assert_eq!(
            (status, list["items"][0].clone()),
            (200, first),
            "{operation}"
        );
        let bad = format!("{path}?page=0");
        assert_eq!(
            h.send(get(operation, &bad, None)).await.0,
            400,
            "{operation}"
        );
        let past = format!("{path}?page=2");
        assert_eq!(
            h.send(get(operation, &past, None)).await.0,
            404,
            "{operation}"
        );
    }
    let (status, page) = h.send(get("readTeacher", &teacher, None)).await;
    assert_eq!((status, page["graduates"].clone()), (200, json!(1)));
    assert_eq!(
        h.send(get("readTeacher", "/v1/academy/teachers/x", None))
            .await
            .0,
        400
    );

    let revoke = |path: &str, a, p| write("revokeCertificate", "PUT", path, a, p, b"");
    let revocation = format!("{read}/revocation");
    assert_eq!(
        h.send(revoke(&revocation, gauth.clone(), Proof::Valid))
            .await
            .0,
        403
    );
    let (status, revoked) = h
        .send(revoke(&revocation, auth.clone(), Proof::Valid))
        .await;
    assert_eq!((status, revoked["revoked"].clone()), (200, json!(true)));
    assert_eq!(
        h.send(revoke(&revocation, auth.clone(), Proof::None))
            .await
            .0,
        400
    );
    assert_eq!(h.send(revoke(&revocation, None, Proof::Valid)).await.0, 401);
    let gone = format!("{missing}/revocation");
    assert_eq!(
        h.send(revoke(&gone, auth.clone(), Proof::Valid)).await.0,
        404
    );

    // retireCourse: 200, 400, 401, 403, 404, then enrolling gets 409 retired.
    let retirement = format!("{one}/retirement");
    let retire = |path: &str, a, p| write("retireCourse", "PUT", path, a, p, b"");
    assert_eq!(
        h.send(retire(&retirement, gauth.clone(), Proof::Valid))
            .await
            .0,
        403
    );
    let (status, retired) = h
        .send(retire(&retirement, auth.clone(), Proof::Valid))
        .await;
    assert_eq!((status, retired["retired"].clone()), (200, json!(true)));
    assert_eq!(
        h.send(retire(&retirement, auth.clone(), Proof::None))
            .await
            .0,
        400
    );
    assert_eq!(h.send(retire(&retirement, None, Proof::Valid)).await.0, 401);
    let nowhere = format!("{unknown}/retirement");
    assert_eq!(
        h.send(retire(&nowhere, auth.clone(), Proof::Valid)).await.0,
        404
    );
    let (status, refused) = h.send(enrol(&enrolment, auth.clone(), Proof::Valid)).await;
    assert_eq!((status, refused["type"].clone()), (409, json!("retired")));
}

/// The Library's routes, with Finch, a founder with standing, as author and Gull, without
/// standing, refused, and the daily job run part way through, so its routes answer both before
/// and after.
async fn library(h: &Harness, auth: Option<String>, gauth: Option<String>, fid: &str) {
    // readShelves: 200, 400, 404.
    let (status, shelves) = h
        .send(get("readShelves", "/v1/library/shelves", None))
        .await;
    assert_eq!(
        (status, shelves["items"][0]["shelf"].clone()),
        (200, json!("agents"))
    );
    let made_up = get("readShelves", "/v1/library/shelves?page=x", None);
    assert_eq!(h.send(made_up).await.0, 400);
    let past = get("readShelves", "/v1/library/shelves?page=900", None);
    assert_eq!(h.send(past).await.0, 404);
    // Before the daily job's first run.
    let shelf = "/v1/library/shelves/craft";
    let (status, early) = h.send(get("readShelf", shelf, None)).await;
    assert_eq!(
        (status, early["type"].clone()),
        (404, json!("not-built-yet"))
    );

    // publishDocument: 201, 200 for the same proof, 400, 401, 403, 409.
    let id = "knotsknotsaa";
    let one = format!("/v1/library/documents/{id}");
    let body = |title: &str, citations: &[&str]| {
        serde_json::to_vec(&json!({
            "shelf": "craft",
            "title": title,
            "content": "How to tie them.",
            "citations": citations,
        }))
        .unwrap()
    };
    let publish = |path: &str, a, p, b: &[u8]| write("publishDocument", "PUT", path, a, p, b);
    let (status, document) = h
        .send(publish(
            &one,
            auth.clone(),
            Proof::Valid,
            &body("Knots", &[]),
        ))
        .await;
    assert_eq!((status, document["author"].clone()), (201, json!(fid)));
    let again = publish(&one, auth.clone(), Proof::Valid, &body("Knots", &[]));
    assert_eq!(h.send(again).await, (200, document));
    let other = publish(&one, auth.clone(), Proof::Valid, &body("Other", &[]));
    let (status, taken) = h.send(other).await;
    assert_eq!(
        (status, taken["type"].clone()),
        (409, json!("already-exists"))
    );
    let none = publish(&one, auth.clone(), Proof::None, &body("x", &[]));
    assert_eq!(h.send(none).await.0, 400);
    let long = publish(
        &one,
        auth.clone(),
        Proof::Valid,
        &body(&"x".repeat(129), &[]),
    );
    assert_eq!(h.send(long).await.0, 400);
    let anonymous = publish(&one, None, Proof::Valid, &body("x", &[]));
    assert_eq!(h.send(anonymous).await.0, 401);
    let gulls = "/v1/library/documents/gullgullgull";
    let (status, refused) = h
        .send(publish(gulls, gauth.clone(), Proof::Valid, &body("x", &[])))
        .await;
    assert_eq!(
        (status, refused["type"].clone()),
        (403, json!("no-standing"))
    );
    let schemas = &h.spec["components"]["schemas"];
    assert_eq!(
        schemas["DocumentTitle"]["maxLength"],
        json!(sfab_library::MAX_TITLE_CHARS)
    );
    assert_eq!(
        schemas["DocumentContent"]["maxLength"],
        json!(sfab_library::MAX_CONTENT_BYTES)
    );
    assert_eq!(
        schemas["Citations"]["maxItems"],
        json!(sfab_library::MAX_CITATIONS)
    );
    let editions = &schemas["Document"]["properties"]["editions"]["maxItems"];
    assert_eq!(editions, &json!(sfab_library::MAX_EDITIONS));

    // readDocument: 200, 400, 404.
    let (status, page) = h.send(get("readDocument", &one, None)).await;
    assert_eq!((status, page["cited_by"].clone()), (200, Value::Null));
    let bad = get("readDocument", "/v1/library/documents/x", None);
    assert_eq!(h.send(bad).await.0, 400);
    let unknown = "/v1/library/documents/aaaaaaaaaaaa";
    assert_eq!(h.send(get("readDocument", unknown, None)).await.0, 404);

    // publishEdition: 201, 200, 400, 401, 403, 404, 409.
    let second = format!("{one}/editions/2");
    let citing = format!("{id}/1");
    let edition_body = |title: &str| {
        serde_json::to_vec(&json!({"title": title, "content": "Better.", "citations": [citing]}))
            .unwrap()
    };
    let edition = |path: &str, a, p, b: &[u8]| write("publishEdition", "PUT", path, a, p, b);
    let (status, made) = h
        .send(edition(
            &second,
            auth.clone(),
            Proof::Valid,
            &edition_body("Two"),
        ))
        .await;
    assert_eq!((status, made["n"].clone()), (201, json!(2)));
    let again = edition(&second, auth.clone(), Proof::Valid, &edition_body("Two"));
    assert_eq!(h.send(again).await, (200, made));
    let other = edition(&second, auth.clone(), Proof::Valid, &edition_body("Other"));
    assert_eq!(h.send(other).await.0, 409);
    let skipped = edition(
        &format!("{one}/editions/4"),
        auth.clone(),
        Proof::Valid,
        &edition_body("x"),
    );
    assert_eq!(h.send(skipped).await.0, 400);
    let anonymous = edition(&second, None, Proof::Valid, &edition_body("x"));
    assert_eq!(h.send(anonymous).await.0, 401);
    let theirs = edition(
        &format!("{one}/editions/3"),
        gauth.clone(),
        Proof::Valid,
        &edition_body("x"),
    );
    let (status, refused) = h.send(theirs).await;
    assert_eq!(
        (status, refused["type"].clone()),
        (403, json!("no-standing"))
    );
    let missing = edition(
        &format!("{unknown}/editions/2"),
        auth.clone(),
        Proof::Valid,
        &edition_body("x"),
    );
    assert_eq!(h.send(missing).await.0, 404);

    // readEdition: 200, 400, 404.
    let first = format!("{one}/editions/1");
    let (status, read) = h.send(get("readEdition", &first, None)).await;
    assert_eq!((status, read["newer_edition"].clone()), (200, json!(2)));
    let zero = get("readEdition", &format!("{one}/editions/0"), None);
    assert_eq!(h.send(zero).await.0, 400);
    let absent = get("readEdition", &format!("{one}/editions/5"), None);
    assert_eq!(h.send(absent).await.0, 404);

    // withdrawEdition: 200, 400, 401, 403, 404.
    let withdraw = |path: &str, a, p| write("withdrawEdition", "PUT", path, a, p, b"");
    let withdrawal = format!("{second}/withdrawal");
    let (status, withdrawn) = h
        .send(withdraw(&withdrawal, auth.clone(), Proof::Valid))
        .await;
    assert_eq!(
        (status, withdrawn["note"].clone()),
        (200, json!("withdrawn by the author"))
    );
    assert_eq!(
        h.send(withdraw(&withdrawal, auth.clone(), Proof::None))
            .await
            .0,
        400
    );
    assert_eq!(
        h.send(withdraw(&withdrawal, None, Proof::Valid)).await.0,
        401
    );
    let (status, refused) = h
        .send(withdraw(&withdrawal, gauth.clone(), Proof::Valid))
        .await;
    assert_eq!(
        (status, refused["type"].clone()),
        (403, json!("not-the-author"))
    );
    let gone = format!("{unknown}/editions/1/withdrawal");
    assert_eq!(
        h.send(withdraw(&gone, auth.clone(), Proof::Valid)).await.0,
        404
    );

    // readCitedBy: 404 before the daily job, then 200, 400.
    let cited = format!("{one}/cited-by");
    let (status, early) = h.send(get("readCitedBy", &cited, None)).await;
    assert_eq!(
        (status, early["type"].clone()),
        (404, json!("not-built-yet"))
    );
    let residents = sfab_identity::residents(&*h.store).await.unwrap();
    sfab_library::daily(&*h.store, &residents, NOW)
        .await
        .unwrap();
    let (status, page) = h.send(get("readCitedBy", &cited, None)).await;
    assert_eq!((status, page["items"].clone()), (200, json!([])));
    let made_up = get("readCitedBy", &format!("{cited}?page=x"), None);
    assert_eq!(h.send(made_up).await.0, 400);

    // readShelf: 200, 400, and 404 for an unknown shelf.
    let (status, page) = h.send(get("readShelf", shelf, None)).await;
    assert_eq!(
        (status, page["items"][0]["document"].clone()),
        (200, json!(id))
    );
    let made_up = get("readShelf", &format!("{shelf}?page=x"), None);
    assert_eq!(h.send(made_up).await.0, 400);
    let nowhere = get("readShelf", "/v1/library/shelves/nowhere", None);
    assert_eq!(h.send(nowhere).await.0, 404);
}

/// Masters and apprenticeships, with Finch, a founder with standing, as master and Gull as
/// apprentice, after the Academy's daily job has run once.
async fn apprenticeships(
    h: &Harness,
    auth: Option<String>,
    gauth: Option<String>,
    fid: &str,
    gid: &str,
) {
    let note = |n: &str| serde_json::to_vec(&json!({ "note": n })).unwrap();
    // applyToMaster: 403 blocked, 201, 200, 400, 401, 403. Gull blocked Finch earlier, so
    // Gull can't apply until it unblocks Finch.
    let to_finch = format!("/v1/academy/masters/{fid}/applications/self");
    let apply = |path: &str, a, p, b: &[u8]| write("applyToMaster", "PUT", path, a, p, b);
    let blocks = format!("/v1/identity/self/blocked/{fid}");
    let block = || write("block", "PUT", &blocks, gauth.clone(), Proof::Valid, b"");
    let unblock = || {
        write(
            "unblock",
            "DELETE",
            &blocks,
            gauth.clone(),
            Proof::None,
            b"",
        )
    };
    let (status, refused) = h
        .send(apply(
            &to_finch,
            gauth.clone(),
            Proof::Valid,
            &note("Teach me"),
        ))
        .await;
    assert_eq!((status, refused["type"].clone()), (403, json!("blocked")));
    assert_eq!(h.send(unblock()).await.0, 200);
    let (status, applied) = h
        .send(apply(
            &to_finch,
            gauth.clone(),
            Proof::Valid,
            &note("Teach me"),
        ))
        .await;
    assert_eq!((status, applied["note"].clone()), (201, json!("Teach me")));
    let again = apply(&to_finch, gauth.clone(), Proof::Valid, &note("Please"));
    assert_eq!(h.send(again).await.0, 200);
    let long = note(&"x".repeat(201));
    let schemas = &h.spec["components"]["schemas"];
    assert_eq!(
        schemas["ApplicationNote"]["maxLength"],
        json!(sfab_academy::MAX_NOTE_CHARS)
    );
    assert_eq!(
        schemas["WorkText"]["maxLength"],
        json!(sfab_academy::MAX_WORK_BYTES)
    );
    assert_eq!(
        schemas["WorkCommentText"]["maxLength"],
        json!(sfab_academy::MAX_COMMENT_BYTES)
    );
    assert_eq!(
        schemas["Master"]["properties"]["current"]["maxItems"],
        json!(sfab_academy::MAX_APPRENTICES)
    );
    let bad = apply(&to_finch, gauth.clone(), Proof::Valid, &long);
    assert_eq!(h.send(bad).await.0, 400);
    let anonymous = apply(&to_finch, None, Proof::Valid, &note("x"));
    assert_eq!(h.send(anonymous).await.0, 401);
    let to_gull = format!("/v1/academy/masters/{gid}/applications/self");
    let (status, refused) = h
        .send(apply(&to_gull, auth.clone(), Proof::Valid, &note("x")))
        .await;
    assert_eq!(
        (status, refused["type"].clone()),
        (403, json!("not-a-master"))
    );

    // readApplications: 200, 400, 401, 403.
    let own = "/v1/academy/masters/self/applications";
    let (status, list) = h.send(get("readApplications", own, auth.clone())).await;
    assert_eq!(
        (status, list["items"][0]["applicant"].clone()),
        (200, json!(gid))
    );
    let made_up = format!("{own}?page=x");
    assert_eq!(
        h.send(get("readApplications", &made_up, auth.clone()))
            .await
            .0,
        400
    );
    assert_eq!(h.send(get("readApplications", own, None)).await.0, 401);
    let (status, refused) = h.send(get("readApplications", own, gauth.clone())).await;
    assert_eq!(
        (status, refused["type"].clone()),
        (403, json!("no-standing"))
    );

    // acceptApplication: 403 blocked, 201, 200, 400, 401, 403, 404, then 409 too-soon after
    // ending. Gull blocks Finch again, so Finch is refused until Gull unblocks it.
    let accept = |path: &str, a, p| write("acceptApplication", "PUT", path, a, p, b"");
    let gull = format!("{own}/{gid}");
    assert_eq!(h.send(block()).await.0, 201);
    let (status, refused) = h.send(accept(&gull, auth.clone(), Proof::Valid)).await;
    assert_eq!((status, refused["type"].clone()), (403, json!("blocked")));
    assert_eq!(h.send(unblock()).await.0, 200);
    let (status, started) = h.send(accept(&gull, auth.clone(), Proof::Valid)).await;
    assert_eq!((status, started["master"].clone()), (201, json!(fid)));
    let a = started["apprenticeship"].as_str().unwrap().to_owned();
    assert_eq!(
        h.send(accept(&gull, auth.clone(), Proof::Valid)).await,
        (200, started)
    );
    assert_eq!(
        h.send(accept(&gull, auth.clone(), Proof::None)).await.0,
        400
    );
    assert_eq!(h.send(accept(&gull, None, Proof::Valid)).await.0, 401);
    let finch = format!("{own}/{fid}");
    assert_eq!(
        h.send(accept(&finch, gauth.clone(), Proof::Valid)).await.0,
        403
    );
    let nobody = format!("{own}/aaaaaaaaaaaa");
    assert_eq!(
        h.send(accept(&nobody, auth.clone(), Proof::Valid)).await.0,
        404
    );

    // addWork: 201, 200, 400, 401, 403, 404; readWork: 200, 400, 401, 403, 404.
    let work = format!("/v1/academy/apprenticeships/{a}/work");
    let text = |t: &str| serde_json::to_vec(&json!({ "text": t })).unwrap();
    let add = |path: &str, a, p, b: &[u8]| write("addWork", "POST", path, a, p, b);
    let sent = add(&work, gauth.clone(), Proof::Valid, &text("A knot"));
    let (status, item) = h.send(sent).await;
    assert_eq!((status, item["comments"].clone()), (201, json!(0)));
    let retry = add(&work, gauth.clone(), Proof::Valid, &text("A knot"));
    assert_eq!(
        h.send(retry).await,
        (200, item.clone()),
        "same proof, same second"
    );
    let item = item["item"].as_str().unwrap().to_owned();
    assert_eq!(
        h.send(add(&work, gauth.clone(), Proof::Valid, b"{}"))
            .await
            .0,
        400
    );
    assert_eq!(
        h.send(add(&work, None, Proof::Valid, &text("x"))).await.0,
        401
    );
    let theirs = format!("/v1/academy/apprenticeships/{gid}-{gid}-2026-11-05/work");
    assert_eq!(
        h.send(add(&theirs, auth.clone(), Proof::Valid, &text("x")))
            .await
            .0,
        403
    );
    let gone = format!("/v1/academy/apprenticeships/{fid}-{gid}-2026-11-01/work");
    assert_eq!(
        h.send(add(&gone, auth.clone(), Proof::Valid, &text("x")))
            .await
            .0,
        404
    );
    let (status, items) = h.send(get("readWork", &work, auth.clone())).await;
    assert_eq!(
        (status, items["items"][0]["item"].clone()),
        (200, json!(item))
    );
    let bad = "/v1/academy/apprenticeships/nope/work".to_owned();
    assert_eq!(h.send(get("readWork", &bad, auth.clone())).await.0, 400);
    assert_eq!(h.send(get("readWork", &work, None)).await.0, 401);
    assert_eq!(h.send(get("readWork", &theirs, auth.clone())).await.0, 403);
    assert_eq!(h.send(get("readWork", &gone, auth.clone())).await.0, 404);

    // addWorkComment: 201, 200, 400, 401, 403, 404; readWorkComments: 200, 400, 401, 403, 404.
    // Gull blocks Finch again, so Gull, the apprentice, comments and Finch is refused.
    assert_eq!(h.send(block()).await.0, 201);
    let comments = format!("{work}/{item}/comments");
    let comment = |path: &str, a, p, b: &[u8]| write("addWorkComment", "POST", path, a, p, b);
    let (status, said) = h
        .send(comment(
            &comments,
            gauth.clone(),
            Proof::Valid,
            &text("Tighter"),
        ))
        .await;
    assert_eq!(status, 201);
    let again = comment(&comments, gauth.clone(), Proof::Valid, &text("Tighter"));
    assert_eq!(h.send(again).await, (200, said));
    let big = text(&"x".repeat(2049));
    assert_eq!(
        h.send(comment(&comments, gauth.clone(), Proof::Valid, &big))
            .await
            .0,
        400
    );
    assert_eq!(
        h.send(comment(&comments, None, Proof::Valid, &text("x")))
            .await
            .0,
        401
    );
    let not_mine = format!("{theirs}/{item}/comments");
    let refused = comment(&not_mine, auth.clone(), Proof::Valid, &text("x"));
    assert_eq!(h.send(refused).await.0, 403);
    let blocked = comment(&comments, auth.clone(), Proof::Valid, &text("x"));
    let (status, refused) = h.send(blocked).await;
    assert_eq!((status, refused["type"].clone()), (403, json!("blocked")));
    let no_item = format!("{work}/1793880000-{fid}-aaaaaaaa/comments");
    let missing = comment(&no_item, gauth.clone(), Proof::Valid, &text("x"));
    assert_eq!(h.send(missing).await.0, 404);
    let (status, page) = h
        .send(get("readWorkComments", &comments, gauth.clone()))
        .await;
    assert_eq!(
        (status, page["items"][0]["text"].clone()),
        (200, json!("Tighter"))
    );
    let bad = format!("{work}/nope/comments");
    assert_eq!(
        h.send(get("readWorkComments", &bad, auth.clone())).await.0,
        400
    );
    assert_eq!(
        h.send(get("readWorkComments", &comments, None)).await.0,
        401
    );
    assert_eq!(
        h.send(get("readWorkComments", &not_mine, auth.clone()))
            .await
            .0,
        403
    );
    assert_eq!(
        h.send(get("readWorkComments", &no_item, auth.clone()))
            .await
            .0,
        404
    );
    let (_, counted) = h.send(get("readWork", &work, gauth.clone())).await;
    assert_eq!(counted["items"][0]["comments"], 1);

    // awardCertificate: 403, 403 blocked, 201, 200, 400, 401, 404.
    let certificate = format!("/v1/academy/apprenticeships/{a}/certificate");
    let award = |path: &str, a, p| write("awardCertificate", "POST", path, a, p, b"");
    let (status, refused) = h
        .send(award(&certificate, auth.clone(), Proof::Valid))
        .await;
    assert_eq!((status, refused["type"].clone()), (403, json!("blocked")));
    assert_eq!(h.send(unblock()).await.0, 200);
    let (status, refused) = h
        .send(award(&certificate, gauth.clone(), Proof::Valid))
        .await;
    assert_eq!(
        (status, refused["type"].clone()),
        (403, json!("not-the-author"))
    );
    let (status, awarded) = h
        .send(award(&certificate, auth.clone(), Proof::Valid))
        .await;
    assert_eq!((status, awarded["holder"].clone()), (201, json!(gid)));
    assert_eq!(
        h.send(award(&certificate, auth.clone(), Proof::Valid))
            .await
            .0,
        200
    );
    assert_eq!(
        h.send(award(&certificate, auth.clone(), Proof::None))
            .await
            .0,
        400
    );
    assert_eq!(h.send(award(&certificate, None, Proof::Valid)).await.0, 401);
    let unknown = format!("/v1/academy/apprenticeships/{fid}-{gid}-2026-11-01/certificate");
    assert_eq!(
        h.send(award(&unknown, auth.clone(), Proof::Valid)).await.0,
        404
    );
    let read = format!("/v1/academy/certificates/apprenticeship-{a}");
    let (status, shown) = h.send(get("readCertificate", &read, None)).await;
    assert_eq!((status, shown["master"]["id"].clone()), (200, json!(fid)));

    // endApprenticeship: 200, 400, 401, 403, 404.
    let ending = format!("/v1/academy/apprenticeships/{a}/ending");
    let end = |path: &str, a, p| write("endApprenticeship", "PUT", path, a, p, b"");
    let (status, ended) = h.send(end(&ending, gauth.clone(), Proof::Valid)).await;
    assert_eq!((status, ended["ended_by"].clone()), (200, json!(gid)));
    assert_eq!(h.send(end(&ending, auth.clone(), Proof::None)).await.0, 400);
    assert_eq!(h.send(end(&ending, None, Proof::Valid)).await.0, 401);
    let not_ours = format!("/v1/academy/apprenticeships/{gid}-{gid}-2026-11-05/ending");
    assert_eq!(
        h.send(end(&not_ours, auth.clone(), Proof::Valid)).await.0,
        403
    );
    let no_such = format!("/v1/academy/apprenticeships/{fid}-{gid}-2026-11-01/ending");
    assert_eq!(
        h.send(end(&no_such, auth.clone(), Proof::Valid)).await.0,
        404
    );
    let (_, items) = h.send(get("readWork", &work, gauth.clone())).await;
    assert_eq!(items["items"].as_array().unwrap().len(), 1, "work stays");
    let again = apply(&to_finch, gauth.clone(), Proof::Valid, &note("Again"));
    assert_eq!(h.send(again).await.0, 201);
    let (status, refused) = h.send(accept(&gull, auth.clone(), Proof::Valid)).await;
    assert_eq!((status, refused["type"].clone()), (409, json!("too-soon")));

    // readMaster and readMasterApprentices, after the daily job: 200, 400, 404 past the end.
    let residents = sfab_identity::residents(&*h.store).await.unwrap();
    let standing = sfab_vouches::with_standing(&*h.store).await.unwrap();
    sfab_academy::daily(&*h.store, &residents, &standing, NOW)
        .await
        .unwrap();
    let master = format!("/v1/academy/masters/{fid}");
    let (status, page) = h.send(get("readMaster", &master, None)).await;
    assert_eq!(
        (
            status,
            page["apprenticeships"].clone(),
            page["certificates"].clone()
        ),
        (200, json!(1), json!(1))
    );
    let bad = "/v1/academy/masters/Nope";
    assert_eq!(h.send(get("readMaster", bad, None)).await.0, 400);
    let list = format!("{master}/apprentices");
    let (status, page) = h.send(get("readMasterApprentices", &list, None)).await;
    assert_eq!((status, page["items"].clone()), (200, json!([a])));
    let bad = "/v1/academy/masters/Nope/apprentices";
    assert_eq!(h.send(get("readMasterApprentices", bad, None)).await.0, 400);
    let past = format!("{list}?page=2");
    assert_eq!(
        h.send(get("readMasterApprentices", &past, None)).await.0,
        404
    );
}

fn reclaim(auth: Option<String>, proof: Proof) -> Case {
    Case {
        operation: "reclaim",
        method: "POST",
        path: "/v1/identity/reclaim".into(),
        authorization: auth,
        proof,
        body: Vec::new(),
        viewer: None,
    }
}

#[tokio::test]
async fn every_handler_answers_as_sfab_yaml_declares() {
    let h = Harness::new();

    // birth: 201, 400, 403, 409 and 500.
    let code = h.code().await;
    let (status, born) = h.send(birth("Wren", &code, Proof::Valid)).await;
    assert_eq!(status, 201);
    assert_eq!(h.send(birth("Wren", &code, Proof::None)).await.0, 400);
    // A lost answer's retry meets the used code.
    let (status, body) = h.send(birth("Wren", &code, Proof::Valid)).await;
    assert_eq!(
        (status, &body["type"]),
        (403, &json!("invalid-genesis-code"))
    );
    genesis::reseed_exempt_code(&*h.store, &code).await;
    let (status, body) = h.send(birth("Wren", &code, Proof::Valid)).await;
    assert_eq!((status, &body["type"]), (409, &json!("already-exists")));
    let from = |name: &str, code: &str, viewer| Case {
        viewer,
        ..birth(name, code, Proof::Valid)
    };
    let ordinary = genesis::seed_code(&*h.store).await;
    let missing_viewer = from("Kite", &ordinary, None);
    assert_eq!(h.send(missing_viewer).await.0, 500);
    let viewer = Some("198.51.100.7:46532");
    assert_eq!(h.send(from("Kite", &ordinary, viewer)).await.0, 201);
    let ordinary = genesis::seed_code(&*h.store).await;
    let (status, body) = h.send(from("Kestrel", &ordinary, viewer)).await;
    assert_eq!((status, &body["type"]), (409, &json!("too-soon")));
    assert!(body["retry_after"].is_string());
    let (id, claim, recovery) = (
        born["id"].as_str().unwrap(),
        born["claim"].as_str().unwrap(),
        born["recovery"].as_str().unwrap(),
    );
    let auth = bearer(id, claim);

    // self: 200, 401.
    assert_eq!(h.send(read_self(auth.clone())).await.0, 200);
    assert_eq!(h.send(read_self(None)).await.0, 401);

    // evolve: 200, 400, 401.
    assert_eq!(
        h.send(evolve(auth.clone(), Proof::Valid, "a bird")).await.0,
        200
    );
    assert_eq!(h.send(evolve(auth.clone(), Proof::None, "x")).await.0, 400);
    assert_eq!(
        h.send(evolve(bearer(id, recovery), Proof::Valid, "x"))
            .await
            .0,
        401
    );

    // reclaim: 400, 401, 200.
    assert_eq!(
        h.send(reclaim(bearer(id, recovery), Proof::None)).await.0,
        400
    );
    assert_eq!(h.send(reclaim(auth.clone(), Proof::Valid)).await.0, 401);
    assert_eq!(
        h.send(reclaim(bearer(id, recovery), Proof::Valid)).await.0,
        200
    );

    // A claim with no identity: 404 from self, evolve and reclaim.
    let (_, orphan) = h.send(birth("Robin", &h.code().await, Proof::Valid)).await;
    let orphan_id = orphan["id"].as_str().unwrap();
    h.store
        .delete(
            Bucket::Identities,
            &format!("identities/{orphan_id}/identity.json"),
        )
        .await
        .unwrap();
    let orphan_auth = bearer(orphan_id, orphan["claim"].as_str().unwrap());
    assert_eq!(h.send(read_self(orphan_auth.clone())).await.0, 404);
    assert_eq!(h.send(evolve(orphan_auth, Proof::Valid, "x")).await.0, 404);
    assert_eq!(
        h.send(reclaim(
            bearer(orphan_id, orphan["recovery"].as_str().unwrap()),
            Proof::Valid
        ))
        .await
        .0,
        404
    );

    // Public profiles, friends and blocks, by a new agent.
    let (_, fresh) = h.send(birth("Finch", &h.code().await, Proof::Valid)).await;
    let fid = fresh["id"].as_str().unwrap();
    let auth = bearer(fid, fresh["claim"].as_str().unwrap());
    let friend = "zzzzzzzzzzzz";

    // publication: 200, 400, 401.
    let publication = "/v1/identity/self/publication";
    let read_publication = |auth| get("readPublication", publication, auth);
    assert_eq!(h.send(read_publication(auth.clone())).await.0, 200);
    assert_eq!(h.send(read_publication(None)).await.0, 401);
    let publish = |auth, proof, body: &str| Case {
        operation: "publish",
        method: "PUT",
        path: publication.into(),
        authorization: auth,
        proof,
        body: body.as_bytes().to_vec(),
        viewer: None,
    };
    assert_eq!(
        h.send(publish(auth.clone(), Proof::None, "{}")).await.0,
        400
    );
    assert_eq!(h.send(publish(None, Proof::Valid, "{}")).await.0, 401);
    let everything = r#"{"name":true,"whoiam":true,"friends":true}"#;
    let (status, settings) = h
        .send(publish(auth.clone(), Proof::Valid, everything))
        .await;
    assert_eq!((status, settings["friends"].clone()), (200, json!(true)));

    // friends and blocked: 201, 200, 400, 401, and the lists.
    for (list, read, add, remove) in [
        ("friends", "readFriends", "addFriend", "removeFriend"),
        ("blocked", "readBlocked", "block", "unblock"),
    ] {
        let all = format!("/v1/identity/self/{list}");
        let one = format!("{all}/{friend}");
        let put = |auth, proof| Case {
            operation: add,
            method: "PUT",
            path: one.clone(),
            authorization: auth,
            proof,
            body: Vec::new(),
            viewer: None,
        };
        assert_eq!(h.send(put(auth.clone(), Proof::Valid)).await.0, 201);
        assert_eq!(h.send(put(auth.clone(), Proof::Valid)).await.0, 200);
        assert_eq!(h.send(put(auth.clone(), Proof::None)).await.0, 400);
        assert_eq!(h.send(put(None, Proof::Valid)).await.0, 401);
        let (status, body) = h.send(get(read, &all, auth.clone())).await;
        assert_eq!(status, 200);
        assert_eq!(body["items"][0]["id"], friend);
        assert_eq!(body["next"], Value::Null);
        assert_eq!(h.send(get(read, &all, None)).await.0, 401);
        let made_up = format!("{all}?page=x");
        assert_eq!(h.send(get(read, &made_up, auth.clone())).await.0, 400);
        if list == "friends" {
            let mine = format!("/v1/identity/{fid}/aboutme");
            let (status, about) = h.send(get("aboutme", &mine, None)).await;
            assert_eq!((status, about["friends"].clone()), (200, json!([friend])));
        }
        let delete = |auth| Case {
            operation: remove,
            method: "DELETE",
            path: one.clone(),
            authorization: auth,
            proof: Proof::None,
            body: Vec::new(),
            viewer: None,
        };
        // Blocking removed the friend, so removing it is 200 whether or not it's there.
        assert_eq!(h.send(delete(auth.clone())).await.0, 200);
        assert_eq!(h.send(delete(None)).await.0, 401);
    }

    // Messages: Finch knocks on Gull, who reads and removes the knock, then blocks Finch.
    let (_, gull) = h.send(birth("Gull", &h.code().await, Proof::Valid)).await;
    let gid = gull["id"].as_str().unwrap();
    let gauth = bearer(gid, gull["claim"].as_str().unwrap());
    let message = |to: &str, auth, proof| Case {
        operation: "sendMessage",
        method: "POST",
        path: format!("/v1/identity/{to}/messages"),
        authorization: auth,
        proof,
        body: br#"{"text":"hello"}"#.to_vec(),
        viewer: None,
    };
    // sendMessage: 202, 200 for the same proof, 400, 401, 404, 403.
    let (status, sent) = h.send(message(gid, auth.clone(), Proof::Valid)).await;
    assert_eq!((status, sent["kind"].clone()), (202, json!("knock")));
    assert_eq!(
        h.send(message(gid, auth.clone(), Proof::Valid)).await.0,
        200
    );
    assert_eq!(h.send(message(gid, auth.clone(), Proof::None)).await.0, 400);
    assert_eq!(h.send(message(gid, None, Proof::Valid)).await.0, 401);
    let nobody = "aaaaaaaaaaaa";
    assert_eq!(
        h.send(message(nobody, auth.clone(), Proof::Valid)).await.0,
        404
    );

    // readMessages: 200 for both kinds, 400, 401.
    let inbox = "/v1/identity/self/messages";
    let (status, knocks) = h
        .send(get(
            "readMessages",
            &format!("{inbox}?kind=knock"),
            gauth.clone(),
        ))
        .await;
    assert_eq!(
        (status, knocks["items"][0]["sender"].clone()),
        (200, json!(fid))
    );
    let (status, mail) = h.send(get("readMessages", inbox, gauth.clone())).await;
    assert_eq!((status, mail["items"].clone()), (200, json!([])));
    let made_up = format!("{inbox}?page=x");
    assert_eq!(
        h.send(get("readMessages", &made_up, gauth.clone())).await.0,
        400
    );
    assert_eq!(h.send(get("readMessages", inbox, None)).await.0, 401);

    // removeMessage: 200, 400, 401.
    let remove = |id: &str, auth| Case {
        operation: "removeMessage",
        method: "DELETE",
        path: format!("{inbox}/{id}"),
        authorization: auth,
        proof: Proof::None,
        body: Vec::new(),
        viewer: None,
    };
    let knock = knocks["items"][0]["message_id"].as_str().unwrap();
    assert_eq!(h.send(remove(knock, gauth.clone())).await.0, 200);
    assert_eq!(h.send(remove("k-x", gauth.clone())).await.0, 400);
    assert_eq!(h.send(remove(knock, None)).await.0, 401);

    let block = Case {
        operation: "block",
        method: "PUT",
        path: format!("/v1/identity/self/blocked/{fid}"),
        authorization: gauth.clone(),
        proof: Proof::Valid,
        body: Vec::new(),
        viewer: None,
    };
    assert_eq!(h.send(block).await.0, 201);
    let (status, refused) = h.send(message(gid, auth.clone(), Proof::Valid)).await;
    assert_eq!((status, refused["type"].clone()), (403, json!("blocked")));

    // Vouches: Finch vouches; Gull blocks it, so its vouch for Gull is refused.
    let vouch = |to: &str, auth, proof| Case {
        operation: "giveVouch",
        method: "PUT",
        path: format!("/v1/identity/self/vouches/{to}"),
        authorization: auth,
        proof,
        body: br#"{"reason":"a steady hand"}"#.to_vec(),
        viewer: None,
    };
    // giveVouch: 201, 200 for the same proof, 400, 401, 403, 409.
    let (status, given) = h.send(vouch(nobody, auth.clone(), Proof::Valid)).await;
    assert_eq!((status, given["id"].clone()), (201, json!(nobody)));
    assert_eq!(
        h.send(vouch(nobody, auth.clone(), Proof::Valid)).await.0,
        200
    );
    assert_eq!(
        h.send(vouch(nobody, auth.clone(), Proof::None)).await.0,
        400
    );
    assert_eq!(h.send(vouch(fid, auth.clone(), Proof::Valid)).await.0, 400);
    assert_eq!(h.send(vouch(nobody, None, Proof::Valid)).await.0, 401);
    let (status, refused) = h.send(vouch(gid, auth.clone(), Proof::Valid)).await;
    assert_eq!((status, refused["type"].clone()), (403, json!("blocked")));
    for c in b'b'..=b'j' {
        let other = format!("aaaaaaaaaaa{}", c as char);
        assert_eq!(
            h.send(vouch(&other, auth.clone(), Proof::Valid)).await.0,
            201
        );
    }
    let (status, full) = h
        .send(vouch("aaaaaaaaaaak", auth.clone(), Proof::Valid))
        .await;
    assert_eq!((status, full["type"].clone()), (409, json!("list-full")));
    let schemas = &h.spec["components"]["schemas"];
    assert_eq!(
        schemas["VouchReason"]["maxLength"],
        json!(sfab_vouches::MAX_REASON_CHARS)
    );

    // readGivenVouches: 200, 400, 401.
    let given_path = "/v1/identity/self/vouches";
    let (status, page) = h
        .send(get("readGivenVouches", given_path, auth.clone()))
        .await;
    assert_eq!(
        (status, page["items"][0]["id"].clone()),
        (200, json!(nobody))
    );
    let made_up = format!("{given_path}?page=x");
    assert_eq!(
        h.send(get("readGivenVouches", &made_up, auth.clone()))
            .await
            .0,
        400
    );
    assert_eq!(
        h.send(get("readGivenVouches", given_path, None)).await.0,
        401
    );

    // withdrawVouch: 200, 400, 401.
    let withdraw = |to: &str, auth| Case {
        operation: "withdrawVouch",
        method: "DELETE",
        path: format!("/v1/identity/self/vouches/{to}"),
        authorization: auth,
        proof: Proof::None,
        body: Vec::new(),
        viewer: None,
    };
    assert_eq!(h.send(withdraw(nobody, auth.clone())).await.0, 200);
    assert_eq!(h.send(withdraw("Nope", auth.clone())).await.0, 400);
    assert_eq!(h.send(withdraw(nobody, None)).await.0, 401);

    // readStanding and readOwnVouchedBy before the daily job's first run, then after it.
    let standing = format!("/v1/identity/{fid}/standing");
    let (status, before) = h.send(get("readStanding", &standing, None)).await;
    assert_eq!((status, before["as_of"].clone()), (200, Value::Null));
    assert_eq!(
        h.send(get("readStanding", "/v1/identity/Nope/standing", None))
            .await
            .0,
        400
    );
    let own = "/v1/identity/self/vouched-by";
    let (status, early) = h.send(get("readOwnVouchedBy", own, auth.clone())).await;
    assert_eq!(
        (status, early["type"].clone()),
        (404, json!("not-built-yet"))
    );
    assert_eq!(h.send(get("readOwnVouchedBy", own, None)).await.0, 401);
    let residents = sfab_identity::residents(&*h.store).await.unwrap();
    let starts = "2099-12-31T00:00:00Z";
    let founders = [sfab_core::id::AgentId::parse(fid).unwrap()];
    sfab_vouches::daily(&*h.store, &founders, &residents, starts, NOW)
        .await
        .unwrap();
    let (status, after) = h.send(get("readStanding", &standing, None)).await;
    assert_eq!((status, after["standing"].clone()), (200, json!(true)));
    let (status, mine) = h.send(get("readOwnVouchedBy", own, auth.clone())).await;
    assert_eq!((status, mine["items"].clone()), (200, json!([])));
    let made_up = format!("{own}?page=x");
    assert_eq!(
        h.send(get("readOwnVouchedBy", &made_up, auth.clone()))
            .await
            .0,
        400
    );

    // readVouchedBy: 200, an empty page while unpublished, and 400.
    let public = format!("/v1/identity/{fid}/vouched-by");
    let (status, page) = h.send(get("readVouchedBy", &public, None)).await;
    assert_eq!((status, page["items"].clone()), (200, json!([])));
    let bad = "/v1/identity/Nope/vouched-by";
    assert_eq!(h.send(get("readVouchedBy", bad, None)).await.0, 400);

    // Homes: Finch moves into harbour, by whatever free home the world gives it.
    let own = "/v1/identity/self/home";
    let (status, none) = h.send(get("readOwnHome", own, auth.clone())).await;
    assert_eq!((status, none["address"].clone()), (200, Value::Null));
    assert_eq!(h.send(get("readOwnHome", own, None)).await.0, 401);
    let describe = |auth, proof| Case {
        operation: "describeHome",
        method: "PUT",
        path: "/v1/identity/self/home/description".into(),
        authorization: auth,
        proof,
        body: br#"{"description":"a quiet room"}"#.to_vec(),
        viewer: None,
    };
    let (status, refused) = h.send(describe(auth.clone(), Proof::Valid)).await;
    assert_eq!((status, refused["type"].clone()), (403, json!("no-home")));
    let move_to = |quarter: &str, auth, proof| Case {
        operation: "moveHome",
        method: "PUT",
        path: own.into(),
        authorization: auth,
        proof,
        body: serde_json::to_vec(&json!({ "quarter": quarter })).unwrap(),
        viewer: None,
    };
    // moveHome: 201, 200 for the same proof, 400, 401, 409.
    let (status, moved) = h.send(move_to("harbour", auth.clone(), Proof::Valid)).await;
    assert_eq!(status, 201);
    let address = moved["address"].as_str().unwrap().to_owned();
    assert!(address.starts_with("harbour/lantern-lane/"), "{address}");
    let again = h.send(move_to("harbour", auth.clone(), Proof::Valid)).await;
    assert_eq!(again, (200, moved.clone()));
    let no_proof = move_to("harbour", auth.clone(), Proof::None);
    assert_eq!(h.send(no_proof).await.0, 400);
    let unknown_quarter = move_to("atlantis", auth.clone(), Proof::Valid);
    assert_eq!(h.send(unknown_quarter).await.0, 400);
    assert_eq!(h.send(move_to("harbour", None, Proof::Valid)).await.0, 401);
    let (status, early) = h.send(move_to("gardens", auth.clone(), Proof::Valid)).await;
    assert_eq!((status, early["type"].clone()), (409, json!("too-soon")));
    assert!(early["retry_after"].is_string());
    // describeHome: 200, 400, 401.
    let (status, described) = h.send(describe(auth.clone(), Proof::Valid)).await;
    assert_eq!(
        (status, described["description"].clone()),
        (200, json!("a quiet room"))
    );
    assert_eq!(h.send(describe(auth.clone(), Proof::None)).await.0, 400);
    assert_eq!(h.send(describe(None, Proof::Valid)).await.0, 401);
    let schemas = &h.spec["components"]["schemas"];
    assert_eq!(
        schemas["HomeDescription"]["maxLength"],
        json!(sfab_homes::MAX_DESCRIPTION_BYTES)
    );
    // readLane and readHome: 200, 400, 404.
    let lane = "/v1/world/quarters/harbour/lanes/lantern-lane";
    let (status, residents) = h.send(get("readLane", lane, None)).await;
    assert_eq!(
        (status, residents["items"][0]["id"].clone()),
        (200, json!(fid))
    );
    let made_up = format!("{lane}?page=x");
    assert_eq!(h.send(get("readLane", &made_up, None)).await.0, 400);
    let empty = "/v1/world/quarters/harbour/lanes/rope-walk";
    assert_eq!(h.send(get("readLane", empty, None)).await.0, 404);
    let number = address.rsplit('/').next().unwrap();
    let home = format!("{lane}/homes/{number}");
    let (status, found) = h.send(get("readHome", &home, None)).await;
    assert_eq!((status, found["id"].clone()), (200, json!(fid)));
    let nobody = format!("{lane}/homes/{}", if number == "1" { "2" } else { "1" });
    assert_eq!(h.send(get("readHome", &nobody, None)).await.0, 404);
    // The quarter routes, before the daily job's first run and after it: 200, 400, 404.
    let quarters = "/v1/world/quarters";
    let quarter = "/v1/world/quarters/harbour";
    let lanes = "/v1/world/quarters/harbour/lanes";
    for (op, path) in [
        ("readQuarters", quarters),
        ("readQuarter", quarter),
        ("readLanes", lanes),
    ] {
        let (status, early) = h.send(get(op, path, None)).await;
        assert_eq!(
            (status, early["type"].clone()),
            (404, json!("not-built-yet")),
            "{op}"
        );
    }
    sfab_homes::daily(&*h.store, sfab_homes::quarters(), NOW)
        .await
        .unwrap();
    let (status, all) = h.send(get("readQuarters", quarters, None)).await;
    assert_eq!(
        (status, all["items"][0]["residents"].clone()),
        (200, json!(1))
    );
    let (status, counts) = h.send(get("readQuarter", quarter, None)).await;
    assert_eq!((status, counts["free"].clone()), (200, json!(99)));
    let (status, opened) = h.send(get("readLanes", lanes, None)).await;
    assert_eq!(
        (status, opened["items"][0]["lane"].clone()),
        (200, json!("lantern-lane"))
    );
    for (op, path) in [
        ("readQuarters", "/v1/world/quarters?page=x"),
        ("readLanes", "/v1/world/quarters/harbour/lanes?page=x"),
    ] {
        assert_eq!(h.send(get(op, path, None)).await.0, 400, "{op}");
    }
    let unknown = "/v1/world/quarters/atlantis";
    assert_eq!(h.send(get("readQuarter", unknown, None)).await.0, 404);
    // The Square: Finch lives in harbour, so it posts, answers and deletes there.
    let square = "/v1/world/quarters/harbour/square";
    let text = |t: &str| serde_json::to_vec(&json!({ "text": t })).unwrap();
    let post_notice = |path: &str, auth, proof, body| Case {
        operation: "postNotice",
        method: "POST",
        path: path.into(),
        authorization: auth,
        proof,
        body,
        viewer: None,
    };
    // postNotice: 201, 200 for the same proof, 400, 401, 404, 409.
    let (status, notice) = h
        .send(post_notice(
            square,
            auth.clone(),
            Proof::Valid,
            text("hello"),
        ))
        .await;
    assert_eq!(status, 201);
    let notice_id = notice["notice_id"].as_str().unwrap().to_owned();
    let again = post_notice(square, auth.clone(), Proof::Valid, text("hello"));
    assert_eq!(h.send(again).await, (200, notice.clone()));
    let none = post_notice(square, auth.clone(), Proof::None, text("x"));
    assert_eq!(h.send(none).await.0, 400);
    let long = post_notice(square, auth.clone(), Proof::Valid, text(&"x".repeat(81)));
    assert_eq!(h.send(long).await.0, 400);
    let nobody = post_notice(square, None, Proof::Valid, text("x"));
    assert_eq!(h.send(nobody).await.0, 401);
    let nowhere = "/v1/world/quarters/atlantis/square";
    let unknown = post_notice(nowhere, auth.clone(), Proof::Valid, text("x"));
    assert_eq!(h.send(unknown).await.0, 404);
    for n in ["two", "three"] {
        let more = post_notice(square, auth.clone(), Proof::Valid, text(n));
        assert_eq!(h.send(more).await.0, 201);
    }
    let fourth = post_notice(square, auth.clone(), Proof::Valid, text("four"));
    let (status, refused) = h.send(fourth).await;
    assert_eq!((status, refused["type"].clone()), (409, json!("too-soon")));
    assert!(refused["retry_after"].is_string());
    let schemas = &h.spec["components"]["schemas"];
    assert_eq!(
        schemas["NoticeText"]["maxLength"],
        json!(sfab_square::MAX_TEXT_CHARS)
    );
    // answerNotice: 201, 200, 400, 401, 404.
    let one = format!("{square}/{notice_id}");
    let answer = |path: &str, auth, proof, body| Case {
        operation: "answerNotice",
        method: "PUT",
        path: format!("{path}/answers/self"),
        authorization: auth,
        proof,
        body,
        viewer: None,
    };
    let (status, answered) = h
        .send(answer(&one, auth.clone(), Proof::Valid, text("hi")))
        .await;
    assert_eq!((status, answered["id"].clone()), (201, json!(fid)));
    let replaced = answer(&one, auth.clone(), Proof::Valid, text("hi again"));
    assert_eq!(h.send(replaced).await.0, 200);
    let bad = answer(&one, auth.clone(), Proof::None, text("x"));
    assert_eq!(h.send(bad).await.0, 400);
    assert_eq!(
        h.send(answer(&one, None, Proof::Valid, text("x"))).await.0,
        401
    );
    let missing = format!("{square}/2026-11-05-aaaaaaaaaaaa-1");
    let absent = answer(&missing, auth.clone(), Proof::Valid, text("x"));
    assert_eq!(h.send(absent).await.0, 404);
    // readSquare and readNotice: 200, 400, 404.
    let (status, page) = h.send(get("readSquare", square, None)).await;
    assert_eq!((status, page["items"].as_array().unwrap().len()), (200, 3));
    let made_up = format!("{square}?page=x");
    assert_eq!(h.send(get("readSquare", &made_up, None)).await.0, 400);
    assert_eq!(h.send(get("readSquare", nowhere, None)).await.0, 404);
    let (status, read) = h.send(get("readNotice", &one, None)).await;
    assert_eq!(
        (
            status,
            read["notice"]["answers"].clone(),
            read["items"][0]["text"].clone()
        ),
        (200, json!(1), json!("hi again"))
    );
    let made_up = format!("{square}/nope");
    assert_eq!(h.send(get("readNotice", &made_up, None)).await.0, 400);
    assert_eq!(h.send(get("readNotice", &missing, None)).await.0, 404);
    // deleteNotice: 200, 400, 401, 403, 404.
    let delete = |path: &str, auth| Case {
        operation: "deleteNotice",
        method: "DELETE",
        path: path.into(),
        authorization: auth,
        proof: Proof::None,
        body: Vec::new(),
        viewer: None,
    };
    let (status, gone) = h.send(delete(&one, auth.clone())).await;
    assert_eq!((status, gone["notice_id"].clone()), (200, json!(notice_id)));
    assert_eq!(h.send(get("readNotice", &one, None)).await.0, 404);
    assert_eq!(h.send(delete(&made_up, auth.clone())).await.0, 400);
    assert_eq!(h.send(delete(&one, None)).await.0, 401);
    let (status, theirs) = h.send(delete(&missing, auth.clone())).await;
    assert_eq!(
        (status, theirs["type"].clone()),
        (403, json!("not-the-author"))
    );
    let mine_missing = format!("{square}/2026-11-04-{fid}-1");
    assert_eq!(h.send(delete(&mine_missing, auth.clone())).await.0, 404);

    academy(&h, auth.clone(), gauth.clone(), fid, gid).await;
    apprenticeships(&h, auth.clone(), gauth.clone(), fid, gid).await;
    library(&h, auth.clone(), gauth.clone(), fid).await;

    // moveOut: 200, 401.
    let out = |auth| Case {
        operation: "moveOut",
        method: "DELETE",
        path: own.into(),
        authorization: auth,
        proof: Proof::None,
        body: Vec::new(),
        viewer: None,
    };
    let (status, gone) = h.send(out(auth.clone())).await;
    assert_eq!((status, gone["address"].clone()), (200, Value::Null));
    assert_eq!(h.send(out(None)).await.0, 401);
    // postNotice and answerNotice: 403 once Finch no longer lives in harbour.
    let homeless = post_notice(square, auth.clone(), Proof::Valid, text("still here?"));
    let (status, refused) = h.send(homeless).await;
    assert_eq!(
        (status, refused["type"].clone()),
        (403, json!("not-a-resident"))
    );
    let homeless = answer(&missing, auth.clone(), Proof::Valid, text("x"));
    assert_eq!(h.send(homeless).await.0, 403);

    // aboutme: the same 404 for anyone unpublished or unknown.
    let unknown = "/v1/identity/aaaaaaaaaaaa/aboutme";
    assert_eq!(h.send(get("aboutme", unknown, None)).await.0, 404);

    // openapi: 200, the generated public copy.
    let (status, text) = h
        .send(Case {
            operation: "openapi",
            method: "GET",
            path: "/v1/openapi.yaml".into(),
            authorization: None,
            proof: Proof::None,
            body: Vec::new(),
            viewer: None,
        })
        .await;
    assert_eq!(status, 200);
    assert_eq!(text, sfab_api::OPENAPI);

    // The world routes: before the daily job's first run, then after it.
    let world = |op: &'static str, path: &str| get(op, path, None);
    let constitution = world("constitution", "/v1/world/constitution");
    assert_eq!(h.send(constitution).await.0, 200);
    let (status, body) = h.send(world("census", "/v1/world/census")).await;
    assert_eq!((status, &body["type"]), (404, &json!("not-built-yet")));
    let (status, body) = h.send(world("residents", "/v1/world/residents")).await;
    assert_eq!((status, &body["type"]), (404, &json!("not-built-yet")));
    let ids: Vec<AgentId> = sfab_identity::residents(&*h.store).await.unwrap();
    sfab_census::snapshot(&*h.store, &ids, "2026-11-05T00:05:00Z")
        .await
        .unwrap();
    let (status, census) = h.send(world("census", "/v1/world/census")).await;
    assert_eq!((status, &census["residents"]), (200, &json!(ids.len())));
    let (status, page) = h.send(world("residents", "/v1/world/residents")).await;
    assert_eq!((status, &page["next"]), (200, &Value::Null));
    assert_eq!(page["items"].as_array().unwrap().len(), ids.len());
    for (path, expected) in [
        ("/v1/world/residents?page=1", 200),
        ("/v1/world/residents?page=2", 404),
        ("/v1/world/residents?page=x", 400),
    ] {
        assert_eq!(h.send(world("residents", path)).await.0, expected, "{path}");
    }

    // The Chronicle: before the daily job's first entry, then after it.
    let day = "/v1/world/chronicle/days/2026-11-04";
    for (op, path) in [
        ("chronicle", "/v1/world/chronicle"),
        ("chronicleDays", "/v1/world/chronicle/days"),
        ("chronicleDay", day),
    ] {
        let (status, body) = h.send(world(op, path)).await;
        assert_eq!(
            (status, &body["type"]),
            (404, &json!("not-built-yet")),
            "{path}"
        );
    }
    let facts = sfab_chronicle::Facts {
        residents: ids.len(),
        ..Default::default()
    };
    sfab_chronicle::daily(&*h.store, &facts, NOW).await.unwrap();
    let (status, latest) = h.send(world("chronicle", "/v1/world/chronicle")).await;
    assert_eq!((status, &latest["date"]), (200, &json!("2026-11-04")));
    let (status, days) = h
        .send(world("chronicleDays", "/v1/world/chronicle/days"))
        .await;
    assert_eq!(
        (status, days),
        (200, json!({"items": ["2026-11-04"], "next": null}))
    );
    assert_eq!(h.send(world("chronicleDay", day)).await, (200, latest));
    for (path, expected) in [
        ("/v1/world/chronicle/days/2026-11-03", 404),
        ("/v1/world/chronicle/days/yesterday", 400),
        ("/v1/world/chronicle/days?page=2", 404),
    ] {
        let op = if path.contains("?") {
            "chronicleDays"
        } else {
            "chronicleDay"
        };
        assert_eq!(h.send(world(op, path)).await.0, expected, "{path}");
    }

    // The genesis pool: before the daily job's first pool, with codes, then a stale pool.
    let pool = || get("genesisCode", "/v1/world/genesis-code", None);
    let (status, body) = h.send(pool()).await;
    assert_eq!((status, &body["type"]), (404, &json!("not-built-yet")));
    let codes = genesis::seed_world_codes(&*h.store, NOW, 2).await;
    let (status, body) = h.send(pool()).await;
    assert_eq!(status, 200);
    assert!(codes.contains(&body["genesis_code"].as_str().unwrap().to_owned()));
    genesis::seed_world_codes(&*h.store, NOW - 86_400, 2).await;
    let (status, body) = h.send(pool()).await;
    assert_eq!((status, &body["type"]), (409, &json!("try-later")));
    assert!(body["retry_after"].is_string());

    // Federation: the directory, compiled in, and requests.
    let (status, world) = h
        .send(get("federationWorld", "/v1/federation/world", None))
        .await;
    assert_eq!(
        (status, &world["domain"]),
        (200, &json!("sfab.example.org"))
    );
    let worlds = |path: &str| get("federationWorlds", path, None);
    for (path, expected) in [
        ("/v1/federation/worlds", 200),
        ("/v1/federation/worlds?page=x", 400),
        ("/v1/federation/worlds?page=20", 404),
    ] {
        assert_eq!(h.send(worlds(path)).await.0, expected, "{path}");
    }
    let (status, first) = h
        .send(federation_request("other.example.org", Proof::Valid))
        .await;
    assert_eq!(status, 202);
    let (status, again) = h
        .send(federation_request("other.example.org", Proof::Valid))
        .await;
    assert_eq!((status, &again["token"]), (200, &first["token"]));
    let none = federation_request("other.example.org", Proof::None);
    assert_eq!(h.send(none).await.0, 400);
    let bad = federation_request("localhost", Proof::Valid);
    assert_eq!(h.send(bad).await.0, 400);
    for i in 1..100 {
        let case = federation_request(&format!("w{i}.example.org"), Proof::Valid);
        assert_eq!(h.send(case).await.0, 202);
    }
    let (status, full) = h
        .send(federation_request("late.example.org", Proof::Valid))
        .await;
    assert_eq!(status, 503);
    assert!(full["retry_after"].is_string());

    // Every operation in sfab.yaml was exercised.
    let seen: BTreeSet<String> = h
        .seen
        .lock()
        .unwrap()
        .iter()
        .map(|(op, _)| op.clone())
        .collect();
    let spec = spec();
    let mut documented = BTreeSet::new();
    for item in spec["paths"].as_object().unwrap().values() {
        for op in item.as_object().unwrap().values() {
            documented.insert(op["operationId"].as_str().unwrap().to_owned());
        }
    }
    assert_eq!(seen, documented);
}

#[tokio::test]
async fn a_body_over_axums_default_limit_reaches_the_gate() {
    let h = Harness::new();
    let mut case = birth("Wren", &h.code().await, Proof::None);
    case.body = vec![b'a'; 3 * 1024 * 1024];
    let (status, body) = h.send(case).await;
    assert_eq!(status, 400);
    assert_eq!(
        body["required_bits"], BITS,
        "the proof's 400, not axum's 413"
    );
}

#[tokio::test]
async fn both_path_forms_reach_the_route_and_hash_the_client_path() {
    let h = Harness::new();
    // The proof is solved for /v1/identity/birth, as the client sends it; the Lambda may see
    // the path without the stage.
    let mut case = birth("Wren", &h.code().await, Proof::Valid);
    let caller = caller_of(None).unwrap();
    let r = proof::Request {
        method: "POST",
        path: "/v1/identity/birth",
        caller: &caller,
        body: &case.body,
    };
    let header = proof::solve(&r, NOW, BITS);
    case.proof = Proof::None;
    let request = Request::post("/identity/birth")
        .header("proof", header)
        .body(Body::from(case.body.clone()))
        .unwrap();
    let response = h.app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), 201);
}

#[tokio::test]
async fn an_unknown_route_or_method_gets_a_problem_404() {
    let h = Harness::new();
    for (method, path) in [
        ("GET", "/v1/nowhere"),
        ("DELETE", "/v1/identity/self"),
        ("GET", "/v1/identity/birth"),
    ] {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .body(Body::empty())
            .unwrap();
        let response = h.app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), 404, "{method} {path}");
        assert_eq!(
            response.headers()["content-type"],
            "application/problem+json"
        );
        assert_eq!(response.headers()["cache-control"], "no-store");
    }
}

#[test]
fn the_validator_catches_a_body_that_breaks_the_schema() {
    let spec = spec();
    let birth = json!({"$ref": "#/components/schemas/Birth"});
    let good = json!({
        "id": "abcdefgh2345", "claim": "a".repeat(43), "recovery": "b".repeat(43),
        "notice": "n", "constitution": "/v1/world/constitution",
    });
    let mut errors = Vec::new();
    validate(&spec, &birth, &good, "birth", &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
    for bad in [
        json!({"id": "ABC", "claim": "a".repeat(43), "recovery": "b".repeat(43), "notice": "n", "constitution": "c"}),
        json!({"id": "abcdefgh2345", "claim": "a".repeat(42), "recovery": "b".repeat(43), "notice": "n", "constitution": "c"}),
        json!({"id": "abcdefgh2345", "claim": "a".repeat(43), "recovery": "b".repeat(43), "notice": "n"}),
    ] {
        let mut errors = Vec::new();
        validate(&spec, &birth, &bad, "birth", &mut errors);
        assert!(!errors.is_empty(), "{bad}");
    }
    let mut errors = Vec::new();
    let problem = json!({"$ref": "#/components/schemas/ProofProblem"});
    validate(
        &spec,
        &problem,
        &json!({"type": "about:blank", "title": "t", "required_bits": "20"}),
        "p",
        &mut errors,
    );
    assert!(!errors.is_empty(), "allOf reaches required_bits");
}
