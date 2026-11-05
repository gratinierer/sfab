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
use sfab_core::proof;
use sfab_core::storage::{Bucket, MemoryStore, Store};
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
        _ => {}
    }
}

/// One request, and which operation it exercises.
struct Case {
    operation: &'static str,
    method: &'static str,
    path: &'static str,
    authorization: Option<String>,
    proof: Proof,
    body: Vec<u8>,
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
            app: sfab_api::app(store.clone(), BITS, now),
            store,
            seen: Mutex::new(BTreeSet::new()),
        }
    }

    async fn send(&self, case: Case) -> (u16, Value) {
        let mut request = Request::builder().method(case.method).uri(case.path);
        if let Some(auth) = &case.authorization {
            request = request.header("authorization", auth);
        }
        if let Proof::Valid = case.proof {
            let caller = caller_of(case.authorization.as_deref()).unwrap();
            let r = proof::Request {
                method: case.method,
                path: case.path,
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
        self.check(case.operation, case.method, case.path, response)
            .await
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
        let route = path.strip_prefix("/v1").unwrap();
        let op = &self.spec["paths"][route][method.to_lowercase()];
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

fn birth(name: &str, proof: Proof) -> Case {
    Case {
        operation: "birth",
        method: "POST",
        path: "/v1/identity/birth",
        authorization: None,
        proof,
        body: format!(r#"{{"name":"{name}"}}"#).into_bytes(),
    }
}

fn bearer(id: &str, secret: &str) -> Option<String> {
    Some(format!("Bearer {id}.{secret}"))
}

fn evolve(auth: Option<String>, proof: Proof, whoiam: &str) -> Case {
    Case {
        operation: "evolve",
        method: "PUT",
        path: "/v1/identity/evolve",
        authorization: auth,
        proof,
        body: serde_json::to_vec(&json!({ "whoiam": whoiam })).unwrap(),
    }
}

fn read_self(auth: Option<String>) -> Case {
    Case {
        operation: "self",
        method: "GET",
        path: "/v1/identity/self",
        authorization: auth,
        proof: Proof::None,
        body: Vec::new(),
    }
}

fn reclaim(auth: Option<String>, proof: Proof) -> Case {
    Case {
        operation: "reclaim",
        method: "POST",
        path: "/v1/identity/reclaim",
        authorization: auth,
        proof,
        body: Vec::new(),
    }
}

#[tokio::test]
async fn every_handler_answers_as_sfab_yaml_declares() {
    let h = Harness::new();

    // birth: 201, 400, 409.
    let (status, born) = h.send(birth("Wren", Proof::Valid)).await;
    assert_eq!(status, 201);
    assert_eq!(h.send(birth("Wren", Proof::None)).await.0, 400);
    assert_eq!(h.send(birth("Wren", Proof::Valid)).await.0, 409);
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
    let (_, orphan) = h.send(birth("Robin", Proof::Valid)).await;
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

    // openapi: 200, the generated public copy.
    let (status, text) = h
        .send(Case {
            operation: "openapi",
            method: "GET",
            path: "/v1/openapi.yaml",
            authorization: None,
            proof: Proof::None,
            body: Vec::new(),
        })
        .await;
    assert_eq!(status, 200);
    assert_eq!(text, sfab_api::OPENAPI);

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
    let mut case = birth("Wren", Proof::None);
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
    let mut case = birth("Wren", Proof::Valid);
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
