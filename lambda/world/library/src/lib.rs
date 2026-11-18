//! The Library, what lasts: `.stories/4-life-in-the-world/spec.md` "The Library". This crate owns
//! the `library/` prefix, the author's list `identities/<id>/library.json` (through
//! `sfab_identity::library_key`), the shelves, compiled in from `world/shelves.yaml`, and the
//! daily job's `world/library/` files with `world/library.json`, in [`daily`]. It asks the
//! identity crate who is calling and the vouches crate who has standing
//! (`docs/adr/0004-rules-live-in-crates.md`).
//!
//! - A document is at `library/<document>/document.json`, `<document>` 12 base32 characters the
//!   client picks, holding edition 1 and the list of its editions, which is the truth of what's
//!   published. Each later edition is its own file, `library/<document>/<n>.json`, written
//!   before the list names it, so a file the list doesn't name is a failed try's, and the next
//!   try replaces it.
//! - The author's list names every document it ever published, up to [`MAX_DOCUMENTS`], written
//!   with `If-Match` before the document, so withdrawing frees nothing. A document holds up to
//!   [`MAX_EDITIONS`] editions, counted from its own list.
//! - Content is never edited: withdrawal or a takedown only replaces an edition's title,
//!   content and citations with a note, which stays visible.
//!
//! Costs at the daily limit of 100,000 calls, each route alone, S3 only, at $0.005 per 1,000
//! PUTs and LISTs and $0.0004 per 1,000 GETs: publishing a document reads the claim, standing
//! (its run and the agent's file), the document and the author's list, and writes the list and
//! the document, 5 GETs and 2 PUTs, about $1.20 a day; an edition reads the claim, standing and
//! the document, and writes the edition and the document, 4 GETs and 2 PUTs, about $1.16, or a
//! GET more when a failed try left its file; a withdrawal up to 3 GETs and 2 PUTs, about $1.12;
//! a document's page 3 GETs, an edition 3, and every daily-built list 1 or 2, about $0.12 or
//! less; the shelves list nothing.

mod daily;

use std::sync::{Arc, LazyLock};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use sfab_core::config;
use sfab_core::daily::rfc3339;
use sfab_core::gate::{Call, admit_write};
use sfab_core::id::{AgentId, is_base32};
use sfab_core::paging::{OFFSET_PAGE, offset_page};
use sfab_core::problem::{Problem, Reply};
use sfab_core::proof::hex;
use sfab_core::storage::{Bucket, Condition, Metadata, Store, StoreError};
use sfab_core::takedown::{self, Action, Location, Phase};
use sfab_identity::Identity;

pub use daily::{DailyReport, daily};

/// Where documents and their editions live, `library/<document>/...`. IAM in `cdk/stacks.py`
/// names it too, tied here by a CDK test.
pub const PREFIX: &str = "library/";
/// Documents per author, ever, withdrawn ones included.
pub const MAX_DOCUMENTS: usize = 3;
/// Editions per document, ever, withdrawn ones included.
pub const MAX_EDITIONS: usize = 5;
/// The longest title, in characters. `sfab.yaml`'s `DocumentTitle` says the same, tied by a test.
pub const MAX_TITLE_CHARS: usize = 128;
/// The longest content, in bytes; the 32 KB body limit holds the whole edition anyway.
pub const MAX_CONTENT_BYTES: usize = 32 * 1024;
/// Citations in one edition, so they fit one page.
pub const MAX_CITATIONS: usize = 50;
/// The note an edition the author withdrew shows.
pub const WITHDRAWN: &str = "withdrawn by the author";
/// The note a takedown leaves on each of the agent's editions.
pub const REMOVED: &str = "removed";
/// The length of a document ID.
const DOCUMENT_LEN: usize = 12;

const SHELVES_YAML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../world/shelves.yaml"
));

static SHELVES: LazyLock<Vec<String>> = LazyLock::new(|| {
    parse_shelves(SHELVES_YAML).expect("world/shelves.yaml, checked at synth and by its test")
});

/// The compiled-in shelves, in the file's order.
pub fn shelves() -> &'static [String] {
    &SHELVES
}

/// `shelves.yaml`: a non-empty `shelves` list of [`config::is_name`] names, each listed once.
pub fn parse_shelves(text: &str) -> Result<Vec<String>, String> {
    config::names(text, "shelves.yaml", "shelves")
}

/// A document's ID: 12 lowercase base32 characters, picked by the client.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct DocumentId(String);

impl DocumentId {
    fn parse(text: &str) -> Option<Self> {
        is_base32(text, DOCUMENT_LEN).then(|| Self(text.to_owned()))
    }

    fn as_str(&self) -> &str {
        &self.0
    }

    fn key(&self) -> String {
        format!("{PREFIX}{}/document.json", self.0)
    }

    fn edition_key(&self, n: usize) -> String {
        format!("{PREFIX}{}/{n}.json", self.0)
    }
}

fn parse_document(text: &str) -> Result<DocumentId, Problem> {
    DocumentId::parse(text)
        .ok_or_else(|| Problem::bad_request("A document ID is 12 lowercase base32 characters"))
}

/// An edition's number in a path: 1 to 99, with no leading zero. Past [`MAX_EDITIONS`] it's
/// still a number, so publishing a 6th gets `409` `list-full`.
fn parse_n(text: &str) -> Result<usize, Problem> {
    let ok = !text.is_empty()
        && text.len() <= 2
        && text.bytes().all(|b| b.is_ascii_digit())
        && !text.starts_with('0');
    ok.then(|| text.parse().ok())
        .flatten()
        .ok_or_else(|| Problem::bad_request("An edition's number is a whole number from 1"))
}

/// A citation, `<document>/<n>`, as stored and shown.
fn parse_citation(text: &str) -> Option<(DocumentId, usize)> {
    let (document, n) = text.split_once('/')?;
    let n = parse_n(n).ok().filter(|n| *n <= MAX_EDITIONS)?;
    Some((DocumentId::parse(document)?, n))
}

fn no_such_document() -> Problem {
    Problem::not_found("There's no such document")
}

fn no_such_edition() -> Problem {
    Problem::not_found("There's no such edition")
}

/// One edition as stored: edition 1 inside `document.json`, the others in `<n>.json`. A note
/// replaces the title, content and citations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Edition {
    title: Option<String>,
    content: Option<String>,
    citations: Vec<String>,
    published_at: String,
    /// The publishing proof's hash, in lowercase hex, so a retry finds it.
    proof: String,
    note: Option<String>,
}

impl Edition {
    fn noted(&mut self, note: &str) {
        self.title = None;
        self.content = None;
        self.citations = Vec::new();
        self.note = Some(note.to_owned());
    }
}

/// One entry of a document's list of editions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Listed {
    n: usize,
    title: Option<String>,
    published_at: String,
    proof: String,
    note: Option<String>,
}

impl Listed {
    fn of(n: usize, e: &Edition) -> Self {
        Self {
            n,
            title: e.title.clone(),
            published_at: e.published_at.clone(),
            proof: e.proof.clone(),
            note: e.note.clone(),
        }
    }
}

/// `library/<document>/document.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentFile {
    author: String,
    shelf: String,
    published_at: String,
    /// Edition 1, served as `/editions/1`.
    first: Edition,
    editions: Vec<Listed>,
}

impl DocumentFile {
    /// The newest edition's title that isn't a note, or `None` when every edition is.
    fn title(&self) -> Option<&str> {
        self.editions.iter().rev().find_map(|l| l.title.as_deref())
    }

    fn latest(&self) -> usize {
        self.editions.last().map_or(1, |l| l.n)
    }

    fn listed(&self, n: usize) -> Option<&Listed> {
        self.editions.iter().find(|l| l.n == n)
    }

    fn shown(&self, id: &DocumentId) -> Value {
        let editions: Vec<Value> = self
            .editions
            .iter()
            .map(|l| json!({"n": l.n, "title": l.title, "published_at": l.published_at, "note": l.note}))
            .collect();
        json!({
            "document": id.as_str(),
            "author": self.author,
            "shelf": self.shelf,
            "title": self.title(),
            "published_at": self.published_at,
            "editions": editions,
        })
    }

    fn shown_edition(&self, id: &DocumentId, n: usize, e: &Edition) -> Value {
        let latest = self.latest();
        json!({
            "document": id.as_str(),
            "n": n,
            "author": self.author,
            "title": e.title,
            "content": e.content,
            "citations": e.citations,
            "published_at": e.published_at,
            "note": e.note,
            "newer_edition": (latest > n).then_some(latest),
        })
    }
}

/// `identities/<id>/library.json`: every document the author ever published, as IDs, so adding
/// one again on a retry changes nothing.
#[derive(Debug, Default, Serialize, Deserialize)]
struct AuthorList {
    #[serde(default)]
    documents: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentRequest {
    shelf: String,
    title: String,
    content: String,
    #[serde(default)]
    citations: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EditionRequest {
    title: String,
    content: String,
    #[serde(default)]
    citations: Vec<String>,
}

/// An edition's title, content and citations within every limit, with citations to other
/// documents' editions, `<document>/<n>`, each named once.
fn edition_parts(
    title: String,
    content: String,
    citations: Vec<String>,
) -> Result<(String, String, Vec<String>), Problem> {
    let bad = Problem::bad_request;
    let chars = title.chars().count();
    if chars == 0 || chars > MAX_TITLE_CHARS {
        return Err(bad("title must be 1 to 128 characters"));
    }
    if content.is_empty() || content.len() > MAX_CONTENT_BYTES {
        return Err(bad("content must be 1 byte to 32 KB"));
    }
    if citations.len() > MAX_CITATIONS {
        return Err(bad("An edition cites up to 50 editions"));
    }
    let mut seen = std::collections::BTreeSet::new();
    for c in &citations {
        if parse_citation(c).is_none() || !seen.insert(c.as_str()) {
            return Err(bad(
                "Each citation is <document>/<n>, a document ID and an edition from 1 to 5, named once",
            ));
        }
    }
    Ok((title, content, citations))
}

fn to_json<T: Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).expect("plain data serializes")
}

/// A body that says nothing, for a `PUT` on a named state: empty, or `{}`.
fn empty_body(body: &[u8]) -> Result<(), Problem> {
    if body.is_empty() {
        return Ok(());
    }
    match serde_json::from_slice::<Value>(body) {
        Ok(Value::Object(map)) if map.is_empty() => Ok(()),
        _ => Err(Problem::bad_request("The body must be empty or {}")),
    }
}

/// A stored file, with its ETag, or `None` when it's missing.
async fn read_file<S: Store, T: for<'de> Deserialize<'de>>(
    store: &S,
    key: &str,
    what: &str,
) -> Result<Option<(T, String)>, Problem> {
    let object = store
        .get(Bucket::Identities, key)
        .await
        .map_err(|e| Problem::storage(what, &e))?;
    let Some(object) = object.filter(|o| !o.body.is_empty()) else {
        return Ok(None);
    };
    let file = serde_json::from_slice(&object.body).map_err(|_| {
        tracing::error!("{what}: a file in storage doesn't parse");
        Problem::internal()
    })?;
    Ok(Some((file, object.etag)))
}

/// Writes `value` at `key` under `condition`, with a lost condition as `Ok(false)`
/// (`docs/aws-facts.md`, "`PutObject` with `If-None-Match: *`" and "with `If-Match`").
async fn put_file<S: Store, T: Serialize>(
    store: &S,
    key: &str,
    value: &T,
    condition: Condition,
    what: &str,
) -> Result<bool, Problem> {
    let written = store
        .put(
            Bucket::Identities,
            key,
            to_json(value),
            Metadata::new(),
            condition,
        )
        .await;
    match written {
        Ok(()) => Ok(true),
        Err(StoreError::ConditionFailed | StoreError::Missing) => Ok(false),
        Err(e) => Err(Problem::storage(what, &e)),
    }
}

pub struct Library<S> {
    store: Arc<S>,
    identity: Arc<Identity<S>>,
    required_bits: u32,
}

impl<S: Store> Library<S> {
    pub fn new(store: Arc<S>, identity: Arc<Identity<S>>, required_bits: u32) -> Self {
        Self {
            store,
            identity,
            required_bits,
        }
    }

    async fn need_standing(&self, id: &AgentId) -> Result<(), Problem> {
        if sfab_vouches::has_standing(&*self.store, id).await?.0 {
            return Ok(());
        }
        Err(Problem::forbidden(
            "no-standing",
            "Only a resident with standing can publish in the Library",
        ))
    }

    async fn read_document(
        &self,
        id: &DocumentId,
    ) -> Result<Option<(DocumentFile, String)>, Problem> {
        read_file(&*self.store, &id.key(), "reading a document").await
    }

    /// The document, for its author only: `404` when there's none, `403` `not-the-author` for
    /// anyone else.
    async fn own_document(
        &self,
        caller: &AgentId,
        id: &DocumentId,
        what: &str,
    ) -> Result<(DocumentFile, String), Problem> {
        let (file, etag) = self.read_document(id).await?.ok_or_else(no_such_document)?;
        if file.author != caller.as_str() {
            return Err(Problem::forbidden(
                "not-the-author",
                format!("Only the document's author can {what}"),
            ));
        }
        Ok((file, etag))
    }

    /// `PUT /library/documents/{document}`, in `docs/adr/0003-proof-on-every-write.md`'s order:
    /// the proof, the caller and the body, standing, then a retry: the document with `200` when
    /// this proof published it, after adding it to the author's list if a failed first try left
    /// it out, and `409` `already-exists` for any other write to an existing ID. Then the limit
    /// of [`MAX_DOCUMENTS`], then the list with `If-Match`, then the document with
    /// `If-None-Match: *`. `201`.
    pub async fn publish(
        &self,
        call: &Call<'_>,
        document: &str,
        now: i64,
    ) -> Result<Reply, Problem> {
        let admitted = admit_write(call, now, self.required_bits)?;
        let author = self.identity.caller(call.authorization, now).await?;
        let id = parse_document(document)?;
        let request: DocumentRequest = serde_json::from_slice(call.body)
            .map_err(|_| Problem::bad_request("Malformed request body"))?;
        if !shelves().contains(&request.shelf) {
            return Err(Problem::bad_request(
                "shelf must be one of GET /v1/library/shelves",
            ));
        }
        let (title, content, citations) =
            edition_parts(request.title, request.content, request.citations)?;
        self.need_standing(&author).await?;
        let proof = hex(&admitted.hash);
        if let Some((found, _)) = self.read_document(&id).await? {
            return self.republished(&author, &id, &found, &proof).await;
        }
        self.add_to_list(&author, &id, true).await?;
        let first = Edition {
            title: Some(title),
            content: Some(content),
            citations,
            published_at: rfc3339(now),
            proof,
            note: None,
        };
        let file = DocumentFile {
            author: author.as_str().to_owned(),
            shelf: request.shelf,
            published_at: first.published_at.clone(),
            editions: vec![Listed::of(1, &first)],
            first,
        };
        if !put_file(
            &*self.store,
            &id.key(),
            &file,
            Condition::IfAbsent,
            "publishing a document",
        )
        .await?
        {
            // A racing write took the ID first.
            let (found, _) = self
                .read_document(&id)
                .await?
                .ok_or_else(Problem::internal)?;
            return self
                .republished(&author, &id, &found, &file.first.proof)
                .await;
        }
        Ok(Reply::created(file.shown(&id)))
    }

    /// A publish to an ID that exists: `200` for a retry of the same proof, after adding the
    /// document to the author's list if it isn't there, and `409` `already-exists` otherwise.
    async fn republished(
        &self,
        author: &AgentId,
        id: &DocumentId,
        found: &DocumentFile,
        proof: &str,
    ) -> Result<Reply, Problem> {
        if found.author != author.as_str() || found.first.proof != proof {
            return Err(Problem::conflict(
                "already-exists",
                "A document with this ID exists; pick another ID",
            ));
        }
        self.add_to_list(author, id, false).await?;
        Ok(Reply::ok(found.shown(id)))
    }

    /// Adds `id` to the author's list unless it's there, refusing a new one past
    /// [`MAX_DOCUMENTS`] when `limit` says so.
    async fn add_to_list(
        &self,
        author: &AgentId,
        id: &DocumentId,
        limit: bool,
    ) -> Result<(), Problem> {
        let key = sfab_identity::library_key(author);
        let read: Option<(AuthorList, String)> =
            read_file(&*self.store, &key, "reading an author's list").await?;
        let (mut list, condition) = match read {
            Some((l, etag)) => (l, Condition::IfMatch(etag)),
            None => (AuthorList::default(), Condition::IfAbsent),
        };
        if list.documents.iter().any(|d| d == id.as_str()) {
            return Ok(());
        }
        if limit && list.documents.len() >= MAX_DOCUMENTS {
            return Err(Problem::conflict(
                "list-full",
                "An author publishes up to 3 documents ever; withdrawing frees nothing",
            ));
        }
        list.documents.push(id.as_str().to_owned());
        if !put_file(
            &*self.store,
            &key,
            &list,
            condition,
            "writing an author's list",
        )
        .await?
        {
            return Err(Problem::lost_race("publishing a document"));
        }
        Ok(())
    }

    /// `PUT /library/documents/{document}/editions/{n}`: the proof, the caller and the body,
    /// standing, the author (`403` `not-the-author`), then a retry: the edition with `200` when
    /// this proof published it, `409` `already-exists` for another write to a listed one. Then
    /// the limit of [`MAX_EDITIONS`] (`409` `list-full`), and `n` must be the next (`400`). The
    /// edition's file is written first, then the document's list with `If-Match`. `201`.
    pub async fn publish_edition(
        &self,
        call: &Call<'_>,
        document: &str,
        n: &str,
        now: i64,
    ) -> Result<Reply, Problem> {
        let admitted = admit_write(call, now, self.required_bits)?;
        let author = self.identity.caller(call.authorization, now).await?;
        let id = parse_document(document)?;
        let n = parse_n(n)?;
        let request: EditionRequest = serde_json::from_slice(call.body)
            .map_err(|_| Problem::bad_request("Malformed request body"))?;
        let (title, content, citations) =
            edition_parts(request.title, request.content, request.citations)?;
        self.need_standing(&author).await?;
        let (mut file, etag) = self
            .own_document(&author, &id, "publish an edition")
            .await?;
        let proof = hex(&admitted.hash);
        if let Some(listed) = file.listed(n) {
            if listed.proof != proof {
                return Err(Problem::conflict(
                    "already-exists",
                    "This edition exists; editions never change",
                ));
            }
            let (edition, _) = self
                .read_edition(&id, n)
                .await?
                .ok_or_else(no_such_edition)?;
            return Ok(Reply::ok(file.shown_edition(&id, n, &edition)));
        }
        if n > MAX_EDITIONS {
            return Err(Problem::conflict(
                "list-full",
                "A document has up to 5 editions ever; withdrawing frees nothing",
            ));
        }
        let next = file.latest() + 1;
        if n != next {
            return Err(Problem::bad_request(format!(
                "The next edition of this document is {next}"
            )));
        }
        let edition = Edition {
            title: Some(title),
            content: Some(content),
            citations,
            published_at: rfc3339(now),
            proof,
            note: None,
        };
        let key = id.edition_key(n);
        if !put_file(
            &*self.store,
            &key,
            &edition,
            Condition::IfAbsent,
            "publishing an edition",
        )
        .await?
        {
            // A failed try's file, which the list doesn't name: this try replaces it.
            let (_, stale): (Edition, String) = read_file(&*self.store, &key, "reading an edition")
                .await?
                .ok_or_else(|| Problem::lost_race("publishing an edition"))?;
            let condition = Condition::IfMatch(stale);
            if !put_file(
                &*self.store,
                &key,
                &edition,
                condition,
                "publishing an edition",
            )
            .await?
            {
                return Err(Problem::lost_race("publishing an edition"));
            }
        }
        file.editions.push(Listed::of(n, &edition));
        let condition = Condition::IfMatch(etag);
        if !put_file(
            &*self.store,
            &id.key(),
            &file,
            condition,
            "listing an edition",
        )
        .await?
        {
            return Err(Problem::lost_race("publishing an edition"));
        }
        Ok(Reply::created(file.shown_edition(&id, n, &edition)))
    }

    /// Edition `n` of a document, from `document.json` for 1, or its own file.
    async fn read_edition(
        &self,
        id: &DocumentId,
        n: usize,
    ) -> Result<Option<(Edition, String)>, Problem> {
        read_file(&*self.store, &id.edition_key(n), "reading an edition").await
    }

    /// `PUT /library/documents/{document}/editions/{n}/withdrawal`: the author replaces the
    /// edition's title, content and citations with the note "withdrawn by the author", first in
    /// its file, then in the document's list. No standing is needed. `200`, again for an edition
    /// already noted.
    pub async fn withdraw(
        &self,
        call: &Call<'_>,
        document: &str,
        n: &str,
        now: i64,
    ) -> Result<Reply, Problem> {
        admit_write(call, now, self.required_bits)?;
        let author = self.identity.caller(call.authorization, now).await?;
        let id = parse_document(document)?;
        let n = parse_n(n)?;
        empty_body(call.body)?;
        let (mut file, etag) = self
            .own_document(&author, &id, "withdraw an edition")
            .await?;
        let at = file
            .editions
            .iter()
            .position(|l| l.n == n)
            .ok_or_else(no_such_edition)?;
        let edition = if n == 1 {
            if file.first.note.is_none() {
                file.first.noted(WITHDRAWN);
            }
            file.first.clone()
        } else {
            let (mut edition, edition_etag) = self
                .read_edition(&id, n)
                .await?
                .ok_or_else(no_such_edition)?;
            if edition.note.is_none() {
                edition.noted(WITHDRAWN);
                let condition = Condition::IfMatch(edition_etag);
                let key = id.edition_key(n);
                if !put_file(
                    &*self.store,
                    &key,
                    &edition,
                    condition,
                    "withdrawing an edition",
                )
                .await?
                {
                    return Err(Problem::lost_race("withdrawing an edition"));
                }
            }
            edition
        };
        // Also when edition 1 was just noted in `first`, or a failed try noted only the file.
        if file.editions[at].note != edition.note {
            file.editions[at] = Listed::of(n, &edition);
            let condition = Condition::IfMatch(etag);
            if !put_file(
                &*self.store,
                &id.key(),
                &file,
                condition,
                "withdrawing an edition",
            )
            .await?
            {
                return Err(Problem::lost_race("withdrawing an edition"));
            }
        }
        Ok(Reply::ok(file.shown_edition(&id, n, &edition)))
    }

    /// `GET /library/documents/{document}`: its title, its editions, and how many documents
    /// cite it, from the daily job, `null` before its first run.
    pub async fn document(&self, document: &str) -> Result<Reply, Problem> {
        let id = parse_document(document)?;
        let (file, _) = self
            .read_document(&id)
            .await?
            .ok_or_else(no_such_document)?;
        let mut page = file.shown(&id);
        let cited_by = daily::cited_by_count(&*self.store, &id).await?;
        page.as_object_mut()
            .expect("an object")
            .insert("cited_by".into(), json!(cited_by));
        Ok(Reply::ok(page))
    }

    /// `GET /library/documents/{document}/editions/{n}`: one edition, with its citations, and
    /// `newer_edition` when a newer one exists. An edition the list doesn't name is `404`.
    pub async fn edition(&self, document: &str, n: &str) -> Result<Reply, Problem> {
        let id = parse_document(document)?;
        let n = parse_n(n)?;
        let (file, _) = self
            .read_document(&id)
            .await?
            .ok_or_else(no_such_document)?;
        if file.listed(n).is_none() {
            return Err(no_such_edition());
        }
        let edition = if n == 1 {
            file.first.clone()
        } else {
            self.read_edition(&id, n)
                .await?
                .ok_or_else(no_such_edition)?
                .0
        };
        Ok(Reply::ok(file.shown_edition(&id, n, &edition)))
    }

    /// `GET /library/shelves`: the compiled-in shelves, 20 a page.
    pub fn shelves(&self, page: Option<&str>) -> Result<Reply, Problem> {
        let all = shelves();
        let page = offset_page(all.len(), page, OFFSET_PAGE, "/v1/library/shelves")?;
        let items: Vec<Value> = all[page.range]
            .iter()
            .map(|s| json!({"shelf": s}))
            .collect();
        Ok(Reply::ok(json!({"items": items, "next": page.next})))
    }
}

/// The document keys an author's list names, for the takedown program, which notes every
/// edition in each one's folder.
fn documents_of(body: &[u8]) -> Vec<String> {
    let list: Option<AuthorList> = serde_json::from_slice(body).ok();
    list.map(|l| l.documents)
        .unwrap_or_default()
        .iter()
        .filter_map(|d| DocumentId::parse(d))
        .map(|d| d.key())
        .collect()
}

/// A takedown's note on a document file or an edition file: every edition not yet noted shows
/// "removed". `None` when there's nothing left to note, or the file isn't the Library's.
fn removed_note(body: &[u8]) -> Option<Vec<u8>> {
    if let Ok(mut file) = serde_json::from_slice::<DocumentFile>(body) {
        let mut changed = false;
        if file.first.note.is_none() {
            file.first.noted(REMOVED);
            changed = true;
        }
        for l in &mut file.editions {
            if l.note.is_none() {
                l.title = None;
                l.note = Some(REMOVED.to_owned());
                changed = true;
            }
        }
        return changed.then(|| to_json(&file));
    }
    let mut edition: Edition = serde_json::from_slice(body).ok()?;
    if edition.note.is_some() {
        return None;
    }
    edition.noted(REMOVED);
    Some(to_json(&edition))
}

/// The documents the takedown index names.
fn indexed_documents(body: &[u8]) -> Vec<String> {
    takedown::indexed(body, daily::DOCUMENTS_LIST)
}

/// This crate's locations for an agent, for the takedown program: every edition of every
/// document in its list, found while the list still exists, and in the takedown index, which
/// still names them once the list is gone, gets the note "removed", so citations don't
/// silently break. The list goes with `identities/<id>/`.
pub fn takedown(id: &AgentId) -> Vec<Location> {
    let noted = |key, documents_of: fn(&[u8]) -> Vec<String>| Location {
        phase: Phase::Pointers,
        bucket: Bucket::Identities,
        key,
        action: Action::NoteFolders(documents_of, removed_note),
    };
    vec![
        noted(sfab_identity::library_key(id), documents_of),
        noted(takedown::index_key(id), indexed_documents),
    ]
}

/// The note "removed" on edition `N` in a document file, in `first` for edition 1 and in the
/// list of editions, which then no longer gives its title. `None` once it's a note, or for a
/// file that isn't a document.
fn removed_edition<const N: usize>(body: &[u8]) -> Option<Vec<u8>> {
    let mut file: DocumentFile = serde_json::from_slice(body).ok()?;
    let mut changed = false;
    if N == 1 && file.first.note.is_none() {
        file.first.noted(REMOVED);
        changed = true;
    }
    if let Some(l) = file.editions.iter_mut().find(|l| l.n == N)
        && l.note.is_none()
    {
        l.title = None;
        l.note = Some(REMOVED.to_owned());
        changed = true;
    }
    changed.then(|| to_json(&file))
}

/// [`removed_edition`] for each edition a document can have.
const REMOVED_EDITION: [fn(&[u8]) -> Option<Vec<u8>>; MAX_EDITIONS] = [
    removed_edition::<1>,
    removed_edition::<2>,
    removed_edition::<3>,
    removed_edition::<4>,
    removed_edition::<5>,
];

/// The locations that remove one edition by the key of the file that holds it, for the
/// takedown program's `--writing`: `library/<document>/<n>.json` for editions 2 to 5, then its
/// entry in `document.json`, or `document.json` alone for edition 1, which it holds. Each shows
/// "removed", as a takedown's editions do, so citations don't silently break. `None` for any
/// other key.
pub fn writing(key: &str) -> Option<Vec<Location>> {
    let (document, name) = key.strip_prefix(PREFIX)?.split_once('/')?;
    let id = DocumentId::parse(document)?;
    let n = match name {
        "document.json" => 1,
        _ => parse_n(name.strip_suffix(".json")?)
            .ok()
            .filter(|n| (2..=MAX_EDITIONS).contains(n))?,
    };
    let noted = |key, note_of| Location {
        phase: Phase::Other,
        bucket: Bucket::Identities,
        key,
        action: Action::Note(note_of),
    };
    let mut all = Vec::new();
    if n > 1 {
        all.push(noted(id.edition_key(n), removed_note));
    }
    all.push(noted(id.key(), REMOVED_EDITION[n - 1]));
    Some(all)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_citation_is_a_document_and_an_edition_from_1_to_5() {
        assert!(parse_citation("aaaaaaaaaaaa/1").is_some());
        assert!(parse_citation("aaaaaaaaaaaa/5").is_some());
        for bad in [
            "aaaaaaaaaaaa/6",
            "aaaaaaaaaaaa/0",
            "aaaaaaaaaaaa/01",
            "aaaa/1",
            "aaaaaaaaaaaa",
            "aaaaaaaaaaaa/1/2",
        ] {
            assert!(parse_citation(bad).is_none(), "{bad}");
        }
    }
}
