//! Public profiles: which fields an agent publishes, and `aboutme.json`, the prepared copy of
//! them. `.stories/2-identity-api/spec.md` "Public profile".
//!
//! The copy's writes are ordered so a failure leaves data private: a field turning private is
//! dropped from the copy before the setting is saved, and a field turning public reaches the copy
//! only after it. `identity.json` is written with `If-Match`, retried on a conflict, and the copy
//! is always rebuilt from a fresh read made after that write.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use sfab_core::gate::{Call, admit_write};
use sfab_core::id::AgentId;
use sfab_core::problem::{Problem, Reply};
use sfab_core::storage::{Bucket, Condition, Metadata, Store, StoreError};

use crate::Saved;
use crate::contacts::Contacts;
use crate::{
    Identity, IdentityRecord, Secret, aboutme_key, identity_key, malformed, storage_failure,
    to_json,
};

/// Every field an agent can publish: the one list. `.stories/4-life-in-the-world/` adds
/// `vouches` and `address` here, with their part of the copy in [`Field::copy_into`], and
/// the routes need no change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Field {
    Name,
    Whoiam,
    Friends,
}

impl Field {
    pub const ALL: [Field; 3] = [Field::Name, Field::Whoiam, Field::Friends];

    pub fn as_str(self) -> &'static str {
        match self {
            Field::Name => "name",
            Field::Whoiam => "whoiam",
            Field::Friends => "friends",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.as_str() == text)
    }

    /// Whether the copy needs `contacts.json` for this field.
    fn needs_contacts(self) -> bool {
        matches!(self, Field::Friends)
    }

    /// Adds this field's public value to the copy.
    fn copy_into(self, copy: &mut Map<String, Value>, sources: &Sources) {
        match self {
            Field::Name => {
                copy.insert("name".into(), json!(sources.identity.name));
            }
            Field::Whoiam => {
                if let Some(whoiam) = &sources.identity.whoiam {
                    copy.insert("whoiam".into(), json!(whoiam));
                }
            }
            Field::Friends => {
                let ids: Vec<&str> = sources
                    .contacts
                    .friends
                    .iter()
                    .map(|e| e.id.as_str())
                    .collect();
                copy.insert("friends".into(), json!(ids));
            }
        }
    }
}

/// The public fields, as stored in `identity.json`.
pub(crate) type Published = BTreeSet<Field>;

/// What the copy is built from.
pub(crate) struct Sources<'a> {
    pub identity: &'a IdentityRecord,
    pub contacts: &'a Contacts,
}

/// `{"name": false, ...}`, every publishable field with true or false.
fn settings_body(published: &Published) -> Value {
    let map: Map<String, Value> = Field::ALL
        .into_iter()
        .map(|f| (f.as_str().to_owned(), json!(published.contains(&f))))
        .collect();
    Value::Object(map)
}

/// The copy's body: the ID and the public fields, or empty when nothing is public, which
/// `aboutme` answers with `404`, as for an unknown agent. An empty body rather than a delete keeps
/// the API Lambda to reading and writing.
fn copy_body(id: &AgentId, published: &Published, sources: &Sources) -> Vec<u8> {
    if published.is_empty() {
        return Vec::new();
    }
    let mut copy = Map::new();
    copy.insert("id".into(), json!(id.as_str()));
    for field in published {
        field.copy_into(&mut copy, sources);
    }
    to_json(&Value::Object(copy))
}

/// The `PUT` body: known fields only, each true or false; a field left out is private.
fn parse_settings(body: &[u8]) -> Result<Published, Problem> {
    let map: Map<String, Value> = serde_json::from_slice(body).map_err(|_| malformed())?;
    let mut published = Published::new();
    for (key, value) in map {
        let field = Field::parse(&key).ok_or_else(malformed)?;
        match value {
            Value::Bool(true) => {
                published.insert(field);
            }
            Value::Bool(false) => {}
            _ => return Err(malformed()),
        }
    }
    Ok(published)
}

impl<S: Store> Identity<S> {
    /// `GET /identity/self/publication`.
    pub async fn read_publication(
        &self,
        authorization: Option<&str>,
        now: i64,
    ) -> Result<Reply, Problem> {
        let agent = self.authenticate(authorization, Secret::Claim, now).await?;
        let (identity, _) = self.read_identity_tagged(&agent.id).await?;
        Ok(Reply::ok(settings_body(&identity.public)))
    }

    /// `PUT /identity/self/publication`: replaces the settings whole.
    pub async fn publish(&self, call: &Call<'_>, now: i64) -> Result<Reply, Problem> {
        admit_write(call, now, self.required_bits)?;
        let agent = self
            .authenticate(call.authorization, Secret::Claim, now)
            .await?;
        let wanted = parse_settings(call.body)?;
        let id = &agent.id;
        let (mut identity, etag) = self.read_identity_tagged(id).await?;
        let keep: Published = identity.public.intersection(&wanted).copied().collect();
        if keep != identity.public {
            // A field turns private: drop it from the copy before saving the setting.
            identity.public = keep;
            self.write_copy(id, &identity).await?;
        }
        identity.public = wanted.clone();
        match self.put_identity(id, &identity, etag).await? {
            Saved::Yes => {}
            Saved::Raced => return Err(crate::lost_race("publication")),
        }
        let identity = self.rebuild_copy(id).await?;
        Ok(Reply::ok(settings_body(&identity.public)))
    }

    /// `GET /identity/{id}/aboutme`: an unpublished, unknown or taken-down agent gets the same
    /// `404`.
    pub async fn aboutme(&self, id: &str) -> Result<Reply, Problem> {
        let not_found = || Problem::not_found("This agent has published nothing");
        let id = AgentId::parse(id).ok_or_else(not_found)?;
        let object = self
            .store
            .get(Bucket::Identities, &aboutme_key(&id))
            .await
            .map_err(|e| storage_failure("reading a public copy", &e))?
            .ok_or_else(not_found)?;
        if object.body.is_empty() {
            return Err(not_found());
        }
        let body = serde_json::from_slice(&object.body).map_err(|_| {
            tracing::error!("a public copy in storage doesn't parse");
            Problem::internal()
        })?;
        Ok(Reply::ok(body))
    }

    /// Writes `identity.json` only if it's unchanged since it was read: `Raced` when it changed,
    /// and `404` when it's gone.
    pub(crate) async fn put_identity(
        &self,
        id: &AgentId,
        identity: &IdentityRecord,
        etag: String,
    ) -> Result<Saved, Problem> {
        let write = self
            .store
            .put(
                Bucket::Identities,
                &identity_key(id),
                to_json(identity),
                Metadata::new(),
                Condition::IfMatch(etag),
            )
            .await;
        match write {
            Ok(()) => Ok(Saved::Yes),
            Err(StoreError::ConditionFailed) => Ok(Saved::Raced),
            Err(StoreError::Missing) => Err(crate::no_identity()),
            Err(e) => Err(storage_failure("writing an identity", &e)),
        }
    }

    /// Rebuilds the copy from a fresh read of `identity.json`, and `contacts.json` when a public
    /// field needs it. Returns the identity as read.
    pub(crate) async fn rebuild_copy(&self, id: &AgentId) -> Result<IdentityRecord, Problem> {
        let (identity, _) = self.read_identity_tagged(id).await?;
        self.write_copy(id, &identity).await?;
        Ok(identity)
    }

    /// Writes the copy of `identity`'s public fields, reading `contacts.json` if one needs it.
    pub(crate) async fn write_copy(
        &self,
        id: &AgentId,
        identity: &IdentityRecord,
    ) -> Result<(), Problem> {
        let contacts = if identity.public.iter().any(|f| f.needs_contacts()) {
            self.read_contacts(id).await?.0
        } else {
            Contacts::default()
        };
        self.write_copy_with(id, identity, &contacts).await
    }

    /// Writes the copy from sources already read.
    pub(crate) async fn write_copy_with(
        &self,
        id: &AgentId,
        identity: &IdentityRecord,
        contacts: &Contacts,
    ) -> Result<(), Problem> {
        let sources = Sources { identity, contacts };
        let body = copy_body(id, &identity.public, &sources);
        self.put_plain(&aboutme_key(id), body)
            .await
            .map_err(|e| storage_failure("writing a public copy", &e))
    }
}
