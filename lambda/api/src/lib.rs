//! The axum router: maps each route to its crate, and nothing else
//! (`docs/adr/0004-rules-live-in-crates.md`). It turns HTTP into `sfab_core::gate::Call`, and
//! replies and problems back into responses with `Cache-Control: no-store`.

use std::sync::Arc;

use axum::Router;
use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, OriginalUri, Path, Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};

use sfab_census::Census;
use sfab_core::gate::Call;
use sfab_core::problem::{Problem, Reply};
use sfab_core::storage::Store;
use sfab_identity::{Identity, List};

/// The public API description, generated before the build, so a missing copy fails it.
pub const OPENAPI: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../build/openapi.yaml"
));

/// The one stage, which every public path starts with.
const STAGE: &str = "/v1";

pub struct AppState<S> {
    identity: Identity<S>,
    census: Census<S>,
    /// The current Unix time, so tests can fix it.
    now: fn() -> i64,
}

/// The current Unix time, from the system clock.
pub fn system_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

/// The path the client sent, which the proof hashes. Behind CloudFront and API Gateway, the
/// Lambda sees it with the stage when `lambda_http` adds it, and without when
/// `AWS_LAMBDA_HTTP_IGNORE_STAGE_IN_PATH` is set, so rebuild it either way.
pub fn client_path(seen: &str) -> String {
    match seen.strip_prefix(STAGE) {
        Some(rest) if rest.is_empty() || rest.starts_with('/') => seen.to_owned(),
        _ => format!("{STAGE}{seen}"),
    }
}

/// The router, answering both path forms.
pub fn app<S: Store>(store: Arc<S>, required_bits: u32, now: fn() -> i64) -> Router {
    let state = Arc::new(AppState {
        identity: Identity::new(store.clone(), required_bits),
        census: Census::new(store),
        now,
    });
    let routes = Router::new()
        .route("/identity/birth", post(birth::<S>))
        .route("/identity/evolve", put(evolve::<S>))
        .route("/identity/self", get(read_self::<S>))
        .route("/identity/reclaim", post(reclaim::<S>))
        .route("/world/constitution", get(constitution))
        .route("/world/census", get(census::<S>))
        .route("/world/residents", get(residents::<S>))
        .route(
            "/identity/self/publication",
            get(read_publication::<S>).put(publish::<S>),
        )
        .route("/identity/{id}/aboutme", get(aboutme::<S>))
        .route("/identity/self/friends", get(read_friends::<S>))
        .route(
            "/identity/self/friends/{id}",
            put(add_friend::<S>).delete(remove_friend::<S>),
        )
        .route("/identity/self/blocked", get(read_blocked::<S>))
        .route(
            "/identity/self/blocked/{id}",
            put(block::<S>).delete(unblock::<S>),
        )
        .route("/openapi.yaml", get(openapi))
        .fallback(no_route)
        .method_not_allowed_fallback(no_route)
        .with_state(state);
    Router::new()
        .nest(STAGE, routes.clone())
        .merge(routes)
        // No limit of axum's own: the gate decides, so a large body gets the proof's `400`, and
        // API Gateway's own `413` stays the outer bound.
        .layer(DefaultBodyLimit::disable())
        .layer(middleware::from_fn(log_call))
}

/// Errors and metadata only, never a header or a body: logs are the likeliest place for a claim
/// to leak (`.stories/2-identity-api/spec.md` "Logging").
async fn log_call(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let response = next.run(request).await;
    let status = response.status().as_u16();
    if status >= 500 {
        tracing::error!(%method, %path, status, "call failed");
    } else {
        tracing::info!(%method, %path, status, "call");
    }
    response
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    // A header that isn't visible ASCII can't be valid, so it's passed on as empty to fail there.
    headers.get(name).map(|v| v.to_str().unwrap_or(""))
}

fn respond(result: Result<Reply, Problem>) -> Response {
    let (status, content_type, body) = match result {
        Ok(reply) => (reply.status, "application/json", reply.body),
        Err(problem) => (problem.status, "application/problem+json", problem.body()),
    };
    let status = StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let body = serde_json::to_vec(&body).expect("JSON values serialize");
    (
        status,
        [
            (header::CONTENT_TYPE, HeaderValue::from_static(content_type)),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
        ],
        body,
    )
        .into_response()
}

async fn write<S: Store, F>(
    state: &AppState<S>,
    method: &Method,
    uri: &OriginalUri,
    headers: &HeaderMap,
    body: &Bytes,
    route: F,
) -> Response
where
    F: AsyncFnOnce(&Identity<S>, &Call<'_>, i64) -> Result<Reply, Problem>,
{
    let path = client_path(uri.path());
    let call = Call {
        // The method the client sent and hashed, never a literal of ours.
        method: method.as_str(),
        path: &path,
        authorization: header(headers, "authorization"),
        proof: header(headers, "proof"),
        body,
    };
    respond(route(&state.identity, &call, (state.now)()).await)
}

async fn birth<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    method: Method,
    uri: OriginalUri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    write(&state, &method, &uri, &headers, &body, async |i, c, now| {
        i.birth(c, now).await
    })
    .await
}

async fn evolve<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    method: Method,
    uri: OriginalUri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    write(&state, &method, &uri, &headers, &body, async |i, c, now| {
        i.evolve(c, now).await
    })
    .await
}

async fn reclaim<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    method: Method,
    uri: OriginalUri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    write(&state, &method, &uri, &headers, &body, async |i, c, now| {
        i.reclaim(c, now).await
    })
    .await
}

async fn read_self<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    headers: HeaderMap,
) -> Response {
    let authorization = header(&headers, "authorization");
    respond(state.identity.read_self(authorization, (state.now)()).await)
}

async fn constitution() -> Response {
    respond(Ok(sfab_constitution::constitution()))
}

async fn census<S: Store>(State(state): State<Arc<AppState<S>>>) -> Response {
    respond(state.census.census().await)
}

async fn residents<S: Store>(State(state): State<Arc<AppState<S>>>, uri: OriginalUri) -> Response {
    respond(state.census.residents(page(&uri).as_deref()).await)
}

async fn read_publication<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    headers: HeaderMap,
) -> Response {
    let authorization = header(&headers, "authorization");
    let now = (state.now)();
    respond(state.identity.read_publication(authorization, now).await)
}

async fn publish<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    method: Method,
    uri: OriginalUri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    write(&state, &method, &uri, &headers, &body, async |i, c, now| {
        i.publish(c, now).await
    })
    .await
}

async fn aboutme<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(id): Path<String>,
) -> Response {
    respond(state.identity.aboutme(&id).await)
}

/// The `?page=` token, if any, from the query as sent.
fn page(uri: &OriginalUri) -> Option<String> {
    uri.query()?
        .split('&')
        .find_map(|pair| pair.strip_prefix("page="))
        .map(str::to_owned)
}

async fn read_list<S: Store>(
    state: &AppState<S>,
    list: List,
    uri: &OriginalUri,
    headers: &HeaderMap,
) -> Response {
    let authorization = header(headers, "authorization");
    let token = page(uri);
    let now = (state.now)();
    let reply = state
        .identity
        .list_contacts(list, authorization, token.as_deref(), now);
    respond(reply.await)
}

async fn read_friends<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    uri: OriginalUri,
    headers: HeaderMap,
) -> Response {
    read_list(&state, List::Friends, &uri, &headers).await
}

async fn read_blocked<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    uri: OriginalUri,
    headers: HeaderMap,
) -> Response {
    read_list(&state, List::Blocked, &uri, &headers).await
}

async fn add_contact<S: Store>(
    state: &AppState<S>,
    list: List,
    (id, method, uri, headers, body): (String, Method, OriginalUri, HeaderMap, Bytes),
) -> Response {
    write(state, &method, &uri, &headers, &body, async |i, c, now| {
        i.add_contact(list, c, &id, now).await
    })
    .await
}

async fn add_friend<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(id): Path<String>,
    method: Method,
    uri: OriginalUri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    add_contact(&state, List::Friends, (id, method, uri, headers, body)).await
}

async fn block<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(id): Path<String>,
    method: Method,
    uri: OriginalUri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    add_contact(&state, List::Blocked, (id, method, uri, headers, body)).await
}

async fn remove_contact<S: Store>(
    state: &AppState<S>,
    list: List,
    id: &str,
    headers: &HeaderMap,
) -> Response {
    let authorization = header(headers, "authorization");
    let now = (state.now)();
    respond(
        state
            .identity
            .remove_contact(list, authorization, id, now)
            .await,
    )
}

async fn remove_friend<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    remove_contact(&state, List::Friends, &id, &headers).await
}

async fn unblock<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    remove_contact(&state, List::Blocked, &id, &headers).await
}

async fn openapi() -> Response {
    (
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/yaml"),
            ),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
        ],
        OPENAPI,
    )
        .into_response()
}

/// API Gateway answers unknown routes itself; this only covers what reaches the Lambda anyway.
async fn no_route() -> Response {
    respond(Err(Problem::not_found(
        "No such route or method. Start at /v1/openapi.yaml",
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rebuilds_the_path_the_client_sent() {
        for (seen, sent) in [
            ("/v1/identity/birth", "/v1/identity/birth"),
            ("/identity/birth", "/v1/identity/birth"),
            ("/v1", "/v1"),
            ("/v1x/identity", "/v1/v1x/identity"),
        ] {
            assert_eq!(client_path(seen), sent, "{seen}");
        }
    }
}
