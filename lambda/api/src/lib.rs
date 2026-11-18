//! The axum router: maps each route to its crate, and nothing else
//! (`docs/adr/0004-rules-live-in-crates.md`). It turns HTTP into `sfab_core::gate::Call`, and
//! replies and problems back into responses with `Cache-Control: no-store`.

use std::sync::Arc;

use axum::Router;
use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, FromRequest, OriginalUri, Path, Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};

use sfab_academy::Academy;
use sfab_census::Census;
use sfab_chronicle::Chronicle;
use sfab_core::gate::Call;
use sfab_core::problem::{Problem, Reply};
use sfab_core::storage::Store;
use sfab_federation::{Federation, Here};
use sfab_genesis::{Genesis, IpRule};
use sfab_homes::Homes;
use sfab_identity::{Identity, List};
use sfab_library::Library;
use sfab_messaging::Messaging;
use sfab_square::Square;
use sfab_vouches::Vouches;

/// The public API description, generated before the build, so a missing copy fails it.
pub const OPENAPI: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../build/openapi.yaml"
));

/// The one stage, which every public path starts with.
const STAGE: &str = "/v1";

pub struct AppState<S> {
    identity: Arc<Identity<S>>,
    messaging: Messaging<S>,
    vouches: Vouches<S>,
    homes: Homes<S>,
    square: Square<S>,
    academy: Academy<S>,
    library: Library<S>,
    census: Census<S>,
    chronicle: Chronicle<S>,
    federation: Federation<S>,
    genesis: Arc<Genesis<S>>,
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

/// The router, answering both path forms. `ip_rule` is `birth`'s IP rule, and `here` is this
/// world, for the federation directory.
pub fn app<S: Store>(
    store: Arc<S>,
    required_bits: u32,
    ip_rule: IpRule,
    now: fn() -> i64,
    here: Here,
) -> Router {
    let genesis = Arc::new(Genesis::new(store.clone(), ip_rule));
    let identity = Arc::new(Identity::new(store.clone(), required_bits, genesis.clone()));
    let state = Arc::new(AppState {
        messaging: Messaging::new(store.clone(), identity.clone(), required_bits),
        vouches: Vouches::new(store.clone(), identity.clone(), required_bits),
        homes: Homes::new(store.clone(), identity.clone(), required_bits),
        square: Square::new(store.clone(), identity.clone(), required_bits),
        academy: Academy::new(store.clone(), identity.clone(), required_bits),
        library: Library::new(store.clone(), identity.clone(), required_bits),
        identity,
        census: Census::new(store.clone()),
        chronicle: Chronicle::new(store.clone()),
        genesis,
        federation: Federation::new(store, required_bits, here),
        now,
    });
    let routes = Router::new()
        .route("/identity/birth", post(birth::<S>))
        .route("/identity/evolve", put(evolve::<S>))
        .route("/identity/self", get(read_self::<S>))
        .route("/identity/reclaim", post(reclaim::<S>))
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
        .route("/identity/{id}/messages", post(send_message::<S>))
        .route("/identity/self/messages", get(read_messages::<S>))
        .route(
            "/identity/self/messages/{message_id}",
            delete(remove_message::<S>),
        )
        .route("/identity/self/vouches", get(read_given_vouches::<S>))
        .route(
            "/identity/self/vouches/{id}",
            put(give_vouch::<S>).delete(withdraw_vouch::<S>),
        )
        .route("/identity/self/vouched-by", get(read_own_vouched_by::<S>))
        .route("/identity/{id}/vouched-by", get(read_vouched_by::<S>))
        .route("/identity/{id}/standing", get(read_standing::<S>))
        .route(
            "/identity/self/home",
            get(read_own_home::<S>)
                .put(move_home::<S>)
                .delete(move_out::<S>),
        )
        .route("/identity/self/home/description", put(describe_home::<S>))
        .route("/world/quarters", get(read_quarters::<S>))
        .route("/world/quarters/{quarter}", get(read_quarter::<S>))
        .route("/world/quarters/{quarter}/lanes", get(read_lanes::<S>))
        .route(
            "/world/quarters/{quarter}/lanes/{lane}",
            get(read_lane::<S>),
        )
        .route(
            "/world/quarters/{quarter}/lanes/{lane}/homes/{home}",
            get(read_home::<S>),
        )
        .route(
            "/world/quarters/{quarter}/square",
            get(read_square::<S>).post(post_notice::<S>),
        )
        .route(
            "/world/quarters/{quarter}/square/{notice_id}",
            get(read_notice::<S>).delete(delete_notice::<S>),
        )
        .route(
            "/world/quarters/{quarter}/square/{notice_id}/answers/self",
            put(answer_notice::<S>),
        )
        .route(
            "/academy/courses",
            get(read_catalogue::<S>).post(publish_course::<S>),
        )
        .route(
            "/academy/courses/{course}",
            get(read_course::<S>).put(edit_course::<S>),
        )
        .route(
            "/academy/courses/{course}/retirement",
            put(retire_course::<S>),
        )
        .route("/academy/courses/{course}/tasks", get(read_tasks::<S>))
        .route(
            "/academy/courses/{course}/enrolled",
            get(read_enrolled::<S>),
        )
        .route(
            "/academy/courses/{course}/graduates",
            get(read_graduates::<S>),
        )
        .route(
            "/academy/courses/{course}/enrolments/self",
            put(enrol::<S>).delete(leave_course::<S>),
        )
        .route(
            "/academy/courses/{course}/attempts",
            post(start_attempt::<S>),
        )
        .route(
            "/academy/courses/{course}/attempts/{attempt}",
            get(read_attempt::<S>),
        )
        .route(
            "/academy/courses/{course}/attempts/{attempt}/answers/{n}",
            put(hand_in::<S>),
        )
        .route(
            "/academy/courses/{course}/attempts/{attempt}/submission",
            put(submit_attempt::<S>),
        )
        .route("/academy/courses/{course}/marking", get(read_queue::<S>))
        .route(
            "/academy/courses/{course}/marking/{attempt}",
            get(read_marking::<S>).put(mark_attempt::<S>),
        )
        .route(
            "/academy/certificates/{certificate}",
            get(read_certificate::<S>),
        )
        .route(
            "/academy/certificates/{certificate}/revocation",
            put(revoke_certificate::<S>),
        )
        .route("/academy/teachers/{id}", get(read_teacher::<S>))
        .route(
            "/academy/teachers/{id}/graduates",
            get(read_teacher_graduates::<S>),
        )
        .route("/academy/masters/{id}", get(read_master::<S>))
        .route(
            "/academy/masters/{id}/apprentices",
            get(read_master_apprentices::<S>),
        )
        .route(
            "/academy/masters/{id}/applications/self",
            put(apply_to_master::<S>),
        )
        .route(
            "/academy/masters/self/applications",
            get(read_applications::<S>),
        )
        .route(
            "/academy/masters/self/applications/{id}",
            put(accept_application::<S>),
        )
        .route(
            "/academy/apprenticeships/{apprenticeship}/work",
            get(read_work::<S>).post(add_work::<S>),
        )
        .route(
            "/academy/apprenticeships/{apprenticeship}/work/{item}/comments",
            get(read_comments::<S>).post(add_comment::<S>),
        )
        .route(
            "/academy/apprenticeships/{apprenticeship}/ending",
            put(end_apprenticeship::<S>),
        )
        .route(
            "/academy/apprenticeships/{apprenticeship}/certificate",
            post(award_certificate::<S>),
        )
        .route("/library/shelves", get(read_shelves::<S>))
        .route("/library/shelves/{shelf}", get(read_shelf::<S>))
        .route(
            "/library/documents/{document}",
            get(read_document::<S>).put(publish_document::<S>),
        )
        .route(
            "/library/documents/{document}/cited-by",
            get(read_cited_by::<S>),
        )
        .route(
            "/library/documents/{document}/editions/{n}",
            get(read_edition::<S>).put(publish_edition::<S>),
        )
        .route(
            "/library/documents/{document}/editions/{n}/withdrawal",
            put(withdraw_edition::<S>),
        )
        .route("/identity/{id}/enrolments", get(read_enrolments::<S>))
        .route("/identity/{id}/certificates", get(read_certificates::<S>))
        .route("/world/constitution", get(constitution))
        .route("/world/census", get(census::<S>))
        .route("/world/residents", get(residents::<S>))
        .route("/world/chronicle", get(chronicle::<S>))
        .route("/world/chronicle/days", get(chronicle_days::<S>))
        .route("/world/chronicle/days/{date}", get(chronicle_day::<S>))
        .route("/world/genesis-code", get(genesis_code::<S>))
        .route("/federation/world", get(federation_world::<S>))
        .route("/federation/worlds", get(federation_worlds::<S>))
        .route("/federation/requests", post(request_federation::<S>))
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

/// A `POST` or `PUT` as the client signed it: what the gate and the proof need, with the path
/// rebuilt as the client sent it. It reads the body, so it's always a handler's last argument.
struct Signed {
    method: Method,
    path: String,
    headers: HeaderMap,
    body: Bytes,
}

impl<S: Send + Sync> FromRequest<S> for Signed {
    type Rejection = Response;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let method = request.method().clone();
        let path = match request.extensions().get::<OriginalUri>() {
            Some(original) => client_path(original.path()),
            None => client_path(request.uri().path()),
        };
        let headers = request.headers().clone();
        let body = Bytes::from_request(request, state)
            .await
            .map_err(IntoResponse::into_response)?;
        Ok(Signed {
            method,
            path,
            headers,
            body,
        })
    }
}

impl Signed {
    fn call(&self) -> Call<'_> {
        Call {
            // The method the client sent and hashed, never a literal of ours.
            method: self.method.as_str(),
            path: &self.path,
            authorization: header(&self.headers, "authorization"),
            proof: header(&self.headers, "proof"),
            body: &self.body,
            // Only CloudFront sets it, for `birth`'s IP rule; never logged.
            viewer: header(&self.headers, "cloudfront-viewer-address"),
        }
    }
}

async fn write<S: Store, F>(state: &AppState<S>, signed: &Signed, route: F) -> Response
where
    F: AsyncFnOnce(&Identity<S>, &Call<'_>, i64) -> Result<Reply, Problem>,
{
    respond(route(&state.identity, &signed.call(), (state.now)()).await)
}

async fn birth<S: Store>(State(state): State<Arc<AppState<S>>>, signed: Signed) -> Response {
    write(&state, &signed, async |i, c, now| i.birth(c, now).await).await
}

async fn evolve<S: Store>(State(state): State<Arc<AppState<S>>>, signed: Signed) -> Response {
    write(&state, &signed, async |i, c, now| i.evolve(c, now).await).await
}

async fn reclaim<S: Store>(State(state): State<Arc<AppState<S>>>, signed: Signed) -> Response {
    write(&state, &signed, async |i, c, now| i.reclaim(c, now).await).await
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

async fn chronicle<S: Store>(State(state): State<Arc<AppState<S>>>) -> Response {
    respond(state.chronicle.latest().await)
}

async fn chronicle_days<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    uri: OriginalUri,
) -> Response {
    respond(state.chronicle.days(page(&uri).as_deref()).await)
}

async fn chronicle_day<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(date): Path<String>,
) -> Response {
    respond(state.chronicle.day(&date).await)
}

async fn genesis_code<S: Store>(State(state): State<Arc<AppState<S>>>) -> Response {
    respond(state.genesis.pool_code((state.now)()).await)
}

async fn read_publication<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    headers: HeaderMap,
) -> Response {
    let authorization = header(&headers, "authorization");
    let now = (state.now)();
    respond(state.identity.read_publication(authorization, now).await)
}

async fn publish<S: Store>(State(state): State<Arc<AppState<S>>>, signed: Signed) -> Response {
    write(&state, &signed, async |i, c, now| i.publish(c, now).await).await
}

async fn aboutme<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(id): Path<String>,
) -> Response {
    respond(state.identity.aboutme(&id).await)
}

/// The `?page=` token, if any, from the query as sent.
fn page(uri: &OriginalUri) -> Option<String> {
    query(uri, "page")
}

/// A query parameter's value as sent, if any. Every value this API takes is plain ASCII with
/// nothing to decode, and anything else fails its route's own check.
fn query(uri: &OriginalUri, name: &str) -> Option<String> {
    uri.query()?
        .split('&')
        .find_map(|pair| pair.strip_prefix(name)?.strip_prefix('='))
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
    id: &str,
    signed: &Signed,
) -> Response {
    write(state, signed, async |i, c, now| {
        i.add_contact(list, c, id, now).await
    })
    .await
}

async fn add_friend<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(id): Path<String>,
    signed: Signed,
) -> Response {
    add_contact(&state, List::Friends, &id, &signed).await
}

async fn block<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(id): Path<String>,
    signed: Signed,
) -> Response {
    add_contact(&state, List::Blocked, &id, &signed).await
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

async fn federation_world<S: Store>(State(state): State<Arc<AppState<S>>>) -> Response {
    respond(Ok(state.federation.world()))
}

async fn federation_worlds<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    uri: OriginalUri,
) -> Response {
    respond(state.federation.worlds(page(&uri).as_deref()))
}

async fn request_federation<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    signed: Signed,
) -> Response {
    respond(
        state
            .federation
            .request(&signed.call(), (state.now)())
            .await,
    )
}

async fn send_message<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(id): Path<String>,
    signed: Signed,
) -> Response {
    respond(
        state
            .messaging
            .send(&signed.call(), &id, (state.now)())
            .await,
    )
}

async fn read_messages<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    uri: OriginalUri,
    headers: HeaderMap,
) -> Response {
    let authorization = header(&headers, "authorization");
    let (kind, token) = (query(&uri, "kind"), page(&uri));
    let reply = state.messaging.read(
        authorization,
        kind.as_deref(),
        token.as_deref(),
        (state.now)(),
    );
    respond(reply.await)
}

async fn remove_message<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(message_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let authorization = header(&headers, "authorization");
    let now = (state.now)();
    respond(
        state
            .messaging
            .remove(authorization, &message_id, now)
            .await,
    )
}

async fn give_vouch<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(id): Path<String>,
    signed: Signed,
) -> Response {
    let now = (state.now)();
    respond(state.vouches.give(&signed.call(), &id, now).await)
}

async fn withdraw_vouch<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let authorization = header(&headers, "authorization");
    let now = (state.now)();
    respond(state.vouches.withdraw(authorization, &id, now).await)
}

async fn read_given_vouches<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    uri: OriginalUri,
    headers: HeaderMap,
) -> Response {
    let authorization = header(&headers, "authorization");
    let token = page(&uri);
    let now = (state.now)();
    respond(
        state
            .vouches
            .given(authorization, token.as_deref(), now)
            .await,
    )
}

async fn read_own_vouched_by<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    uri: OriginalUri,
    headers: HeaderMap,
) -> Response {
    let authorization = header(&headers, "authorization");
    let token = page(&uri);
    let now = (state.now)();
    respond(
        state
            .vouches
            .received(authorization, token.as_deref(), now)
            .await,
    )
}

async fn read_vouched_by<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(id): Path<String>,
    uri: OriginalUri,
) -> Response {
    let token = page(&uri);
    respond(state.vouches.vouched_by(&id, token.as_deref()).await)
}

async fn read_standing<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(id): Path<String>,
) -> Response {
    respond(state.vouches.standing(&id).await)
}

async fn read_own_home<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    headers: HeaderMap,
) -> Response {
    let authorization = header(&headers, "authorization");
    let now = (state.now)();
    respond(state.homes.own_home(authorization, now).await)
}

async fn move_home<S: Store>(State(state): State<Arc<AppState<S>>>, signed: Signed) -> Response {
    let now = (state.now)();
    respond(state.homes.move_home(&signed.call(), now).await)
}

async fn move_out<S: Store>(State(state): State<Arc<AppState<S>>>, headers: HeaderMap) -> Response {
    let authorization = header(&headers, "authorization");
    let now = (state.now)();
    respond(state.homes.move_out(authorization, now).await)
}

async fn describe_home<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    signed: Signed,
) -> Response {
    let now = (state.now)();
    respond(state.homes.describe(&signed.call(), now).await)
}

async fn read_quarters<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    uri: OriginalUri,
) -> Response {
    let token = page(&uri);
    respond(state.homes.quarters(token.as_deref()).await)
}

async fn read_quarter<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(quarter): Path<String>,
) -> Response {
    respond(state.homes.quarter_counts(&quarter).await)
}

async fn read_lanes<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(quarter): Path<String>,
    uri: OriginalUri,
) -> Response {
    let token = page(&uri);
    respond(state.homes.lanes(&quarter, token.as_deref()).await)
}

async fn read_lane<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path((quarter, lane)): Path<(String, String)>,
    uri: OriginalUri,
) -> Response {
    let token = page(&uri);
    respond(state.homes.lane(&quarter, &lane, token.as_deref()).await)
}

async fn read_home<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path((quarter, lane, home)): Path<(String, String, String)>,
) -> Response {
    respond(state.homes.home(&quarter, &lane, &home).await)
}

async fn read_square<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(quarter): Path<String>,
    uri: OriginalUri,
) -> Response {
    let token = page(&uri);
    let now = (state.now)();
    respond(state.square.notices(&quarter, token.as_deref(), now).await)
}

async fn post_notice<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(quarter): Path<String>,
    signed: Signed,
) -> Response {
    let now = (state.now)();
    respond(state.square.post(&signed.call(), &quarter, now).await)
}

async fn read_notice<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path((quarter, notice_id)): Path<(String, String)>,
    uri: OriginalUri,
) -> Response {
    let token = page(&uri);
    let now = (state.now)();
    let read = state
        .square
        .notice(&quarter, &notice_id, token.as_deref(), now);
    respond(read.await)
}

async fn delete_notice<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path((quarter, notice_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let authorization = header(&headers, "authorization");
    let now = (state.now)();
    let deleted = state
        .square
        .delete(authorization, &quarter, &notice_id, now);
    respond(deleted.await)
}

async fn answer_notice<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path((quarter, notice_id)): Path<(String, String)>,
    signed: Signed,
) -> Response {
    let now = (state.now)();
    let call = signed.call();
    respond(state.square.answer(&call, &quarter, &notice_id, now).await)
}

async fn read_shelves<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    uri: OriginalUri,
) -> Response {
    let token = page(&uri);
    respond(state.library.shelves(token.as_deref()))
}

async fn read_shelf<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(shelf): Path<String>,
    uri: OriginalUri,
) -> Response {
    let token = page(&uri);
    respond(state.library.shelf(&shelf, token.as_deref()).await)
}

async fn read_document<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(document): Path<String>,
) -> Response {
    respond(state.library.document(&document).await)
}

async fn publish_document<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(document): Path<String>,
    signed: Signed,
) -> Response {
    let now = (state.now)();
    respond(state.library.publish(&signed.call(), &document, now).await)
}

async fn read_cited_by<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(document): Path<String>,
    uri: OriginalUri,
) -> Response {
    let token = page(&uri);
    respond(state.library.cited_by(&document, token.as_deref()).await)
}

async fn read_edition<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path((document, n)): Path<(String, String)>,
) -> Response {
    respond(state.library.edition(&document, &n).await)
}

async fn publish_edition<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path((document, n)): Path<(String, String)>,
    signed: Signed,
) -> Response {
    let now = (state.now)();
    let call = signed.call();
    respond(
        state
            .library
            .publish_edition(&call, &document, &n, now)
            .await,
    )
}

async fn withdraw_edition<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path((document, n)): Path<(String, String)>,
    signed: Signed,
) -> Response {
    let now = (state.now)();
    let call = signed.call();
    respond(state.library.withdraw(&call, &document, &n, now).await)
}

async fn read_catalogue<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    uri: OriginalUri,
) -> Response {
    let token = page(&uri);
    respond(state.academy.catalogue(token.as_deref()).await)
}

async fn publish_course<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    signed: Signed,
) -> Response {
    let now = (state.now)();
    respond(state.academy.publish(&signed.call(), now).await)
}

async fn read_course<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(course): Path<String>,
) -> Response {
    respond(state.academy.course(&course).await)
}

async fn edit_course<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(course): Path<String>,
    signed: Signed,
) -> Response {
    let now = (state.now)();
    respond(state.academy.edit(&signed.call(), &course, now).await)
}

async fn retire_course<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(course): Path<String>,
    signed: Signed,
) -> Response {
    let now = (state.now)();
    respond(state.academy.retire(&signed.call(), &course, now).await)
}

async fn read_tasks<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(course): Path<String>,
    headers: HeaderMap,
    uri: OriginalUri,
) -> Response {
    let authorization = header(&headers, "authorization");
    let token = page(&uri);
    let now = (state.now)();
    let read = state
        .academy
        .tasks(authorization, &course, token.as_deref(), now);
    respond(read.await)
}

async fn read_enrolled<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(course): Path<String>,
    uri: OriginalUri,
) -> Response {
    let token = page(&uri);
    respond(state.academy.enrolled(&course, token.as_deref()).await)
}

async fn read_graduates<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(course): Path<String>,
    uri: OriginalUri,
) -> Response {
    let token = page(&uri);
    respond(state.academy.graduates(&course, token.as_deref()).await)
}

async fn enrol<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(course): Path<String>,
    signed: Signed,
) -> Response {
    let now = (state.now)();
    respond(state.academy.enrol(&signed.call(), &course, now).await)
}

async fn leave_course<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(course): Path<String>,
    headers: HeaderMap,
) -> Response {
    let authorization = header(&headers, "authorization");
    let now = (state.now)();
    respond(state.academy.leave(authorization, &course, now).await)
}

async fn start_attempt<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(course): Path<String>,
    signed: Signed,
) -> Response {
    let now = (state.now)();
    respond(state.academy.start(&signed.call(), &course, now).await)
}

async fn read_attempt<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path((course, attempt)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let authorization = header(&headers, "authorization");
    let now = (state.now)();
    let read = state.academy.attempt(authorization, &course, &attempt, now);
    respond(read.await)
}

async fn hand_in<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path((course, attempt, n)): Path<(String, String, String)>,
    signed: Signed,
) -> Response {
    let now = (state.now)();
    let call = signed.call();
    respond(
        state
            .academy
            .answer(&call, &course, &attempt, &n, now)
            .await,
    )
}

async fn submit_attempt<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path((course, attempt)): Path<(String, String)>,
    signed: Signed,
) -> Response {
    let now = (state.now)();
    let call = signed.call();
    respond(state.academy.submit(&call, &course, &attempt, now).await)
}

async fn read_queue<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(course): Path<String>,
    headers: HeaderMap,
    uri: OriginalUri,
) -> Response {
    let authorization = header(&headers, "authorization");
    let token = page(&uri);
    let now = (state.now)();
    let read = state
        .academy
        .queue(authorization, &course, token.as_deref(), now);
    respond(read.await)
}

async fn read_marking<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path((course, attempt)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let authorization = header(&headers, "authorization");
    let now = (state.now)();
    let read = state.academy.marking(authorization, &course, &attempt, now);
    respond(read.await)
}

async fn mark_attempt<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path((course, attempt)): Path<(String, String)>,
    signed: Signed,
) -> Response {
    let now = (state.now)();
    let call = signed.call();
    respond(state.academy.mark(&call, &course, &attempt, now).await)
}

async fn read_certificate<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(certificate): Path<String>,
) -> Response {
    respond(state.academy.certificate(&certificate).await)
}

async fn revoke_certificate<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(certificate): Path<String>,
    signed: Signed,
) -> Response {
    let now = (state.now)();
    respond(
        state
            .academy
            .revoke(&signed.call(), &certificate, now)
            .await,
    )
}

async fn read_teacher<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(id): Path<String>,
) -> Response {
    respond(state.academy.teacher(&id).await)
}

async fn read_teacher_graduates<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(id): Path<String>,
    uri: OriginalUri,
) -> Response {
    let token = page(&uri);
    respond(state.academy.teacher_graduates(&id, token.as_deref()).await)
}

async fn read_master<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(id): Path<String>,
) -> Response {
    respond(state.academy.master(&id).await)
}

async fn read_master_apprentices<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(id): Path<String>,
    uri: OriginalUri,
) -> Response {
    let token = page(&uri);
    respond(
        state
            .academy
            .master_apprentices(&id, token.as_deref())
            .await,
    )
}

async fn apply_to_master<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(id): Path<String>,
    signed: Signed,
) -> Response {
    let now = (state.now)();
    respond(state.academy.apply(&signed.call(), &id, now).await)
}

async fn read_applications<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    headers: HeaderMap,
    uri: OriginalUri,
) -> Response {
    let authorization = header(&headers, "authorization");
    let token = page(&uri);
    let now = (state.now)();
    let read = state
        .academy
        .applications(authorization, token.as_deref(), now);
    respond(read.await)
}

async fn accept_application<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(id): Path<String>,
    signed: Signed,
) -> Response {
    let now = (state.now)();
    respond(state.academy.accept(&signed.call(), &id, now).await)
}

async fn read_work<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(apprenticeship): Path<String>,
    headers: HeaderMap,
    uri: OriginalUri,
) -> Response {
    let authorization = header(&headers, "authorization");
    let token = page(&uri);
    let now = (state.now)();
    let read = state
        .academy
        .work(authorization, &apprenticeship, token.as_deref(), now);
    respond(read.await)
}

async fn add_work<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(apprenticeship): Path<String>,
    signed: Signed,
) -> Response {
    let now = (state.now)();
    let call = signed.call();
    respond(state.academy.add_work(&call, &apprenticeship, now).await)
}

async fn read_comments<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path((apprenticeship, item)): Path<(String, String)>,
    headers: HeaderMap,
    uri: OriginalUri,
) -> Response {
    let authorization = header(&headers, "authorization");
    let token = page(&uri);
    let now = (state.now)();
    let read = state
        .academy
        .comments(authorization, &apprenticeship, &item, token.as_deref(), now);
    respond(read.await)
}

async fn add_comment<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path((apprenticeship, item)): Path<(String, String)>,
    signed: Signed,
) -> Response {
    let now = (state.now)();
    let call = signed.call();
    respond(
        state
            .academy
            .comment(&call, &apprenticeship, &item, now)
            .await,
    )
}

async fn end_apprenticeship<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(apprenticeship): Path<String>,
    signed: Signed,
) -> Response {
    let now = (state.now)();
    let call = signed.call();
    respond(state.academy.end(&call, &apprenticeship, now).await)
}

async fn award_certificate<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(apprenticeship): Path<String>,
    signed: Signed,
) -> Response {
    let now = (state.now)();
    let call = signed.call();
    respond(state.academy.award(&call, &apprenticeship, now).await)
}

async fn read_enrolments<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(id): Path<String>,
    uri: OriginalUri,
) -> Response {
    let token = page(&uri);
    respond(state.academy.enrolments(&id, token.as_deref()).await)
}

async fn read_certificates<S: Store>(
    State(state): State<Arc<AppState<S>>>,
    Path(id): Path<String>,
    uri: OriginalUri,
) -> Response {
    let token = page(&uri);
    respond(state.academy.certificates(&id, token.as_deref()).await)
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
