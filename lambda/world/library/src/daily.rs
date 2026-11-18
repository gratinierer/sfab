//! The daily job's part for the Library, and the routes that read what it built:
//! `.stories/4-life-in-the-world/spec.md` "The Library" ("Citations", "Shelves") and
//! "Foundations" ("What the daily job writes", "Before the first daily run", "Paging").
//!
//! One listing of `library/` gives every document and edition by key, and each document, and
//! each later edition its list names and that isn't a note, is read once, so the job never asks
//! for a key it didn't list (`docs/aws-facts.md`). Only documents whose author is a resident
//! count, so a taken-down author's documents leave every list at the next run. A citation
//! counts when it names a listed edition of another author's document; a document's
//! `cited-by` holds each citing document once. Through `sfab_core::world::rebuild` it writes,
//! under `world/library/`, each shelf's pages, most cited first, and each cited document's
//! `cited-by` pages, the first with the `total`; and last `world/library.json`, which says the
//! job has run. It also reports, for the takedown index, every author's documents with an
//! edition that isn't a note yet, a taken-down author's too, so a takedown's second run notes
//! what its first missed.

use std::collections::{BTreeMap, BTreeSet};

use futures_util::{StreamExt, TryStreamExt, stream};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use sfab_core::daily::rfc3339;
use sfab_core::id::AgentId;
use sfab_core::paging::OFFSET_PAGE;
use sfab_core::problem::{Problem, Reply};
use sfab_core::storage::{Bucket, Condition, Metadata, Store, StoreError};
use sfab_core::takedown::Index;
use sfab_core::world::{self, PARALLEL, Rebuilt};

use crate::{
    DocumentFile, DocumentId, Edition, Library, PREFIX, parse_citation, parse_document, shelves,
    to_json,
};

/// Everything this step writes under `world/library/`.
const WORLD_PREFIX: &str = "world/library/";
/// Written last by each run, so it says the job has run, with `as_of`.
const BUILT_KEY: &str = "world/library.json";
const SHELVES_PREFIX: &str = "world/library/shelves/";
const CITED_BY_PREFIX: &str = "world/library/cited-by/";
/// Bare IDs on a page, as `docs/api-design.md` "Lists" allows.
/// The takedown index's list of the author's documents, each to be noted "removed".
pub(crate) const DOCUMENTS_LIST: &str = "library/documents";
const ID_PAGE: usize = 1_000;

fn shelf_prefix(shelf: &str) -> String {
    format!("{SHELVES_PREFIX}{shelf}/")
}

fn cited_by_prefix(id: &DocumentId) -> String {
    format!("{CITED_BY_PREFIX}{}/", id.as_str())
}

#[derive(Debug, Serialize, Deserialize)]
struct Built {
    as_of: String,
}

/// What a run did, for the job's log.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DailyReport {
    /// Documents whose author is a resident.
    pub documents: usize,
    /// Citations that count, one per citing and cited document.
    pub citations: usize,
    pub rebuilt: Rebuilt,
    /// For the Chronicle: the IDs of those documents with an edition that isn't withdrawn.
    pub listed: BTreeSet<String>,
    /// For the Chronicle: the most cited of them, and by how many documents, the smallest ID
    /// on a tie; `None` when nothing is cited.
    pub most_cited: Option<(String, usize)>,
    /// Every author's documents still to note, for the takedown index.
    pub index: Index,
}

/// Builds the Library's daily files at `now`, counting only `residents`. Safe to repeat: a
/// second run with nothing changed writes only `world/library.json`.
pub async fn daily<S: Store>(
    store: &S,
    residents: &[AgentId],
    now: i64,
) -> Result<DailyReport, StoreError> {
    let residents: BTreeSet<&AgentId> = residents.iter().collect();
    let resident = |id: &str| AgentId::parse(id).is_some_and(|a| residents.contains(&a));

    let mut listed_files: BTreeSet<String> = BTreeSet::new();
    let mut ids = Vec::new();
    for key in store.list(Bucket::Identities, PREFIX).await? {
        if let Some(id) = key
            .strip_prefix(PREFIX)
            .and_then(|rest| rest.strip_suffix("/document.json"))
            .and_then(DocumentId::parse)
        {
            ids.push(id);
        }
        listed_files.insert(key);
    }
    let reads = ids.into_iter().map(|id| async move {
        let found = world::read_json::<S, DocumentFile>(store, &id.key()).await?;
        Ok::<_, StoreError>(found.map(|f| (id, f)))
    });
    let documents: Vec<(DocumentId, DocumentFile)> = stream::iter(reads)
        .buffer_unordered(PARALLEL)
        .try_collect::<Vec<_>>()
        .await?
        .into_iter()
        .flatten()
        .collect();
    let mut index = Index::default();
    for (id, f) in &documents {
        let left = f.first.note.is_none() || f.editions.iter().any(|l| l.note.is_none());
        if left && let Some(author) = AgentId::parse(&f.author) {
            index.add(author, DOCUMENTS_LIST, id.key());
        }
    }
    let documents: BTreeMap<DocumentId, DocumentFile> = documents
        .into_iter()
        .filter(|(_, f)| resident(&f.author))
        .collect();

    // Each document's citations, from its editions that aren't notes.
    let later = documents.iter().flat_map(|(id, f)| {
        f.editions
            .iter()
            .filter(|l| l.n > 1 && l.note.is_none())
            .map(|l| (id.clone(), id.edition_key(l.n)))
            .filter(|(_, key)| listed_files.contains(key))
            .collect::<Vec<_>>()
    });
    let reads = later.map(|(id, key)| async move {
        let found = world::read_json::<S, Edition>(store, &key).await?;
        Ok::<_, StoreError>(found.map(|e| (id, e.citations)))
    });
    let mut cites: BTreeMap<DocumentId, Vec<String>> = documents
        .iter()
        .map(|(id, f)| (id.clone(), f.first.citations.clone()))
        .collect();
    let read: Vec<Option<(DocumentId, Vec<String>)>> = stream::iter(reads)
        .buffer_unordered(PARALLEL)
        .try_collect()
        .await?;
    for (id, citations) in read.into_iter().flatten() {
        cites.entry(id).or_default().extend(citations);
    }

    // Who cites whom: another author's listed edition, each citing document once.
    let mut cited_by: BTreeMap<DocumentId, BTreeSet<DocumentId>> = BTreeMap::new();
    for (citing, citations) in &cites {
        let author = &documents[citing].author;
        for c in citations {
            let Some((cited, n)) = parse_citation(c) else {
                continue;
            };
            let Some(target) = documents.get(&cited) else {
                continue;
            };
            if &target.author == author || target.listed(n).is_none() {
                continue;
            }
            cited_by.entry(cited).or_default().insert(citing.clone());
        }
    }

    let mut files = BTreeMap::new();
    for (cited, citing) in &cited_by {
        let items: Vec<Value> = citing.iter().map(|d| json!(d.as_str())).collect();
        let mut pages = world::page_files(&cited_by_prefix(cited), items, ID_PAGE);
        let first = format!("{}1.json", cited_by_prefix(cited));
        if let Some(body) = pages.get_mut(&first) {
            let mut page: Value = serde_json::from_slice(body).expect("a page this step built");
            page["total"] = json!(citing.len());
            *body = to_json(&page);
        }
        files.extend(pages);
    }

    // Each shelf: documents with a title left, most cited first, then the newest edition.
    let count = |id: &DocumentId| cited_by.get(id).map_or(0, BTreeSet::len);
    for shelf in shelves() {
        let mut on: Vec<(&DocumentId, &DocumentFile)> = documents
            .iter()
            .filter(|(_, f)| &f.shelf == shelf && f.title().is_some())
            .collect();
        let newest = |f: &DocumentFile| f.editions.last().map(|l| l.published_at.clone());
        on.sort_by(|(a, fa), (b, fb)| {
            count(b)
                .cmp(&count(a))
                .then(newest(fb).cmp(&newest(fa)))
                .then(a.cmp(b))
        });
        let items: Vec<Value> = on
            .iter()
            .map(|(id, f)| {
                json!({
                    "document": id.as_str(),
                    "title": f.title(),
                    "author": f.author,
                    "cited_by": count(id),
                    "published_at": f.published_at,
                })
            })
            .collect();
        files.extend(world::page_files(&shelf_prefix(shelf), items, OFFSET_PAGE));
    }

    let rebuilt = world::rebuild(store, WORLD_PREFIX, files).await?;
    let built = to_json(&Built {
        as_of: rfc3339(now),
    });
    store
        .put(
            Bucket::Identities,
            BUILT_KEY,
            built,
            Metadata::new(),
            Condition::Always,
        )
        .await?;
    let listed = documents
        .iter()
        .filter(|(_, f)| f.title().is_some())
        .map(|(id, _)| id.as_str().to_owned())
        .collect();
    // `cited_by` is in ID order, so the first of the most cited wins a tie.
    let most_cited = cited_by
        .iter()
        .filter(|(id, _)| documents[*id].title().is_some())
        .fold(
            None,
            |best: Option<(&DocumentId, usize)>, (id, citing)| match best {
                Some((_, n)) if n >= citing.len() => best,
                _ => Some((id, citing.len())),
            },
        )
        .map(|(id, n)| (id.as_str().to_owned(), n));
    Ok(DailyReport {
        documents: documents.len(),
        citations: cited_by.values().map(BTreeSet::len).sum(),
        rebuilt,
        listed,
        most_cited,
        index,
    })
}

/// How many documents cite `id`, from the first `cited-by` page: 0 once the job has run and
/// there's none, `None` before its first run.
pub(crate) async fn cited_by_count<S: Store>(
    store: &S,
    id: &DocumentId,
) -> Result<Option<u64>, Problem> {
    let key = format!("{}1.json", cited_by_prefix(id));
    let found = store
        .get(Bucket::Identities, &key)
        .await
        .map_err(|e| Problem::storage("reading cited-by", &e))?;
    if let Some(object) = found {
        let page: Value = serde_json::from_slice(&object.body).map_err(|_| {
            tracing::error!("{key} in storage doesn't parse");
            Problem::internal()
        })?;
        return Ok(Some(page["total"].as_u64().unwrap_or(0)));
    }
    match store.head(Bucket::Identities, BUILT_KEY).await {
        Ok(Some(_)) => Ok(Some(0)),
        Ok(None) => Ok(None),
        Err(e) => Err(Problem::storage("reading the Library's run", &e)),
    }
}

impl<S: Store> Library<S> {
    /// `GET /library/shelves/{shelf}`: a shelf's documents, rebuilt daily, 20 a page. A shelf
    /// not in `shelves.yaml` is `404`.
    pub async fn shelf(&self, shelf: &str, page: Option<&str>) -> Result<Reply, Problem> {
        if !shelves().iter().any(|s| s == shelf) {
            return Err(Problem::not_found("There's no such shelf"));
        }
        let path = format!("/v1/library/shelves/{shelf}");
        world::read_page(&*self.store, &shelf_prefix(shelf), page, &path, BUILT_KEY).await
    }

    /// `GET /library/documents/{document}/cited-by`: the documents that cite it, rebuilt daily,
    /// 1,000 IDs a page. An unknown document reads as one nobody cites.
    pub async fn cited_by(&self, document: &str, page: Option<&str>) -> Result<Reply, Problem> {
        let id = parse_document(document)?;
        let path = format!("/v1/library/documents/{}/cited-by", id.as_str());
        world::read_page(&*self.store, &cited_by_prefix(&id), page, &path, BUILT_KEY).await
    }
}
