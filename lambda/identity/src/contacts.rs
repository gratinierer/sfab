//! Friends and blocks, in `identities/<id>/contacts.json`. `.stories/2-identity-api/spec.md`
//! "Friends and blocks": up to 100 of each, one-directional, with no check that an ID exists.
//! Blocking an agent also removes it as a friend. Writes use `If-Match`, retried on a conflict.
//!
//! When a friend leaves the list, a published copy drops it before `contacts.json` is saved, and
//! after any change the copy is rebuilt from a fresh read, as "Public profile" orders.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use sfab_core::daily::rfc3339;
use sfab_core::gate::{Call, admit_write};
use sfab_core::id::AgentId;
use sfab_core::paging::{OFFSET_PAGE, offset_page};
use sfab_core::problem::{Problem, Reply};
use sfab_core::storage::{Bucket, Condition, Metadata, Store, StoreError};

use crate::profile::{Field, MAX_TRIES};
use crate::{Identity, Saved, Secret, contacts_key, identity_key, storage_failure, to_json};

/// The most entries each list holds; a 101st gets `409` `list-full`.
pub const MAX_ENTRIES: usize = 100;

/// Which of the two lists a route works on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum List {
    Friends,
    Blocked,
}

impl List {
    fn path(self) -> &'static str {
        match self {
            List::Friends => "/v1/identity/self/friends",
            List::Blocked => "/v1/identity/self/blocked",
        }
    }
}

/// One entry, which can gain fields later: unknown ones are kept as they are.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Entry {
    pub id: String,
    pub added_at: String,
    #[serde(flatten)]
    pub more: Map<String, Value>,
}

/// `contacts.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct Contacts {
    #[serde(default)]
    pub friends: Vec<Entry>,
    #[serde(default)]
    pub blocked: Vec<Entry>,
}

impl Contacts {
    fn list(&self, list: List) -> &Vec<Entry> {
        match list {
            List::Friends => &self.friends,
            List::Blocked => &self.blocked,
        }
    }

    fn list_mut(&mut self, list: List) -> &mut Vec<Entry> {
        match list {
            List::Friends => &mut self.friends,
            List::Blocked => &mut self.blocked,
        }
    }

    fn find(&self, list: List, id: &AgentId) -> Option<&Entry> {
        self.list(list).iter().find(|e| e.id == id.as_str())
    }

    fn remove(&mut self, list: List, id: &AgentId) {
        self.list_mut(list).retain(|e| e.id != id.as_str());
    }

    /// These contacts with only the friends still in `after`, for the copy written before a
    /// save: it drops leavers but never shows a newcomer early.
    fn keeping_only_lasting_friends(&self, after: &Contacts) -> Contacts {
        let friends = self
            .friends
            .iter()
            .filter(|e| after.friends.iter().any(|a| a.id == e.id))
            .cloned()
            .collect();
        Contacts {
            friends,
            blocked: self.blocked.clone(),
        }
    }
}

/// How a recipient lists a sender, for messages: `.stories/2-identity-api/spec.md` "Messages".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    /// The recipient lists the sender as a friend.
    Friend,
    /// The recipient blocks the sender.
    Blocked,
    /// Anyone else.
    Stranger,
}

fn bad_id() -> Problem {
    Problem::bad_request("The ID must be 12 lowercase base32 characters")
}

impl<S: Store> Identity<S> {
    /// `PUT /identity/self/friends/{id}` and `PUT /identity/self/blocked/{id}`: `201` with the
    /// new entry, or `200` with the existing one, so a retry is safe.
    pub async fn add_contact(
        &self,
        list: List,
        call: &Call<'_>,
        other: &str,
        now: i64,
    ) -> Result<Reply, Problem> {
        admit_write(call, now, self.required_bits)?;
        let agent = self
            .authenticate(call.authorization, Secret::Claim, now)
            .await?;
        let other = AgentId::parse(other).ok_or_else(bad_id)?;
        let id = &agent.id;
        for _ in 0..MAX_TRIES {
            let (before, etag) = self.read_contacts(id).await?;
            let existing = before.find(list, &other).cloned();
            if existing.is_none() && before.list(list).len() >= MAX_ENTRIES {
                return Err(Problem::conflict(
                    "list-full",
                    format!("This list already holds {MAX_ENTRIES} agents"),
                ));
            }
            let mut after = before.clone();
            let entry = existing.clone().unwrap_or_else(|| Entry {
                id: other.as_str().to_owned(),
                added_at: rfc3339(now),
                more: Map::new(),
            });
            if existing.is_none() {
                after.list_mut(list).push(entry.clone());
            }
            if list == List::Blocked {
                after.remove(List::Friends, &other);
            }
            // A retry, with nothing left to change: redo only the copy's rebuild.
            let unchanged = existing.is_some() && after.friends.len() == before.friends.len();
            if !unchanged {
                match self.save_contacts(id, &before, &after, etag).await? {
                    Saved::Yes => {}
                    Saved::Raced => continue,
                }
            }
            self.refresh_copy(id).await?;
            return Ok(match existing {
                Some(_) => Reply::ok(json!(entry)),
                None => Reply::created(json!(entry)),
            });
        }
        Err(crate::lost_races("contacts"))
    }

    /// `DELETE /identity/self/friends/{id}` and `DELETE /identity/self/blocked/{id}`: `200`
    /// whether or not the agent was listed, so a retry is safe.
    pub async fn remove_contact(
        &self,
        list: List,
        authorization: Option<&str>,
        other: &str,
        now: i64,
    ) -> Result<Reply, Problem> {
        let agent = self.authenticate(authorization, Secret::Claim, now).await?;
        let other = AgentId::parse(other).ok_or_else(bad_id)?;
        let id = &agent.id;
        let removed = json!({"id": other.as_str()});
        for _ in 0..MAX_TRIES {
            let (before, etag) = self.read_contacts(id).await?;
            if before.find(list, &other).is_none() {
                // Already gone, perhaps by an earlier try: redo only the copy's rebuild, as
                // `add_contact` does, in case that try failed after saving.
                self.refresh_copy(id).await?;
                return Ok(Reply::ok(removed));
            }
            let mut after = before.clone();
            after.remove(list, &other);
            match self.save_contacts(id, &before, &after, etag).await? {
                Saved::Yes => {}
                Saved::Raced => continue,
            }
            self.refresh_copy(id).await?;
            return Ok(Reply::ok(removed));
        }
        Err(crate::lost_races("contacts"))
    }

    /// `GET /identity/self/friends` and `GET /identity/self/blocked`, 20 entries a page, with an
    /// offset as the `?page=` token.
    pub async fn list_contacts(
        &self,
        list: List,
        authorization: Option<&str>,
        page: Option<&str>,
        now: i64,
    ) -> Result<Reply, Problem> {
        let agent = self.authenticate(authorization, Secret::Claim, now).await?;
        let (contacts, _) = self.read_contacts(&agent.id).await?;
        let entries = contacts.list(list);
        let page = offset_page(entries.len(), page, OFFSET_PAGE, list.path())?;
        let items: Vec<Value> = entries[page.range].iter().map(|e| json!(e)).collect();
        Ok(Reply::ok(json!({"items": items, "next": page.next})))
    }

    /// How `recipient` lists `sender`, from one read of its `contacts.json`. With no
    /// `contacts.json`, `identity.json` says whether the recipient exists: `None` when it doesn't.
    pub async fn relation(
        &self,
        recipient: &AgentId,
        sender: &AgentId,
    ) -> Result<Option<Relation>, Problem> {
        let (contacts, etag) = self.read_contacts(recipient).await?;
        if etag.is_none() {
            let identity = self
                .store
                .get(Bucket::Identities, &identity_key(recipient))
                .await
                .map_err(|e| storage_failure("reading an identity", &e))?;
            return Ok(identity.map(|_| Relation::Stranger));
        }
        let relation = if contacts.find(List::Blocked, sender).is_some() {
            Relation::Blocked
        } else if contacts.find(List::Friends, sender).is_some() {
            Relation::Friend
        } else {
            Relation::Stranger
        };
        Ok(Some(relation))
    }

    /// `contacts.json` and its ETag, or an empty one and `None` when there's none yet.
    pub(crate) async fn read_contacts(
        &self,
        id: &AgentId,
    ) -> Result<(Contacts, Option<String>), Problem> {
        let object = self
            .store
            .get(Bucket::Identities, &contacts_key(id))
            .await
            .map_err(|e| storage_failure("reading contacts", &e))?;
        let Some(object) = object else {
            return Ok((Contacts::default(), None));
        };
        let contacts = serde_json::from_slice(&object.body).map_err(|_| {
            tracing::error!("contacts in storage don't parse");
            Problem::internal()
        })?;
        Ok((contacts, Some(object.etag)))
    }

    /// Saves `after` only if `contacts.json` is unchanged since `before` was read. When friends
    /// leave and are published, the copy drops them first.
    async fn save_contacts(
        &self,
        id: &AgentId,
        before: &Contacts,
        after: &Contacts,
        etag: Option<String>,
    ) -> Result<Saved, Problem> {
        let kept = before.keeping_only_lasting_friends(after);
        if kept.friends.len() < before.friends.len() {
            let (identity, _) = self.read_identity_tagged(id).await?;
            if identity.public.contains(&Field::Friends) {
                self.write_copy_with(id, &identity, &kept).await?;
            }
        }
        let condition = match etag {
            Some(etag) => Condition::IfMatch(etag),
            None => Condition::IfAbsent,
        };
        let write = self
            .store
            .put(
                Bucket::Identities,
                &contacts_key(id),
                to_json(after),
                Metadata::new(),
                condition,
            )
            .await;
        match write {
            Ok(()) => Ok(Saved::Yes),
            Err(StoreError::ConditionFailed | StoreError::Missing) => Ok(Saved::Raced),
            Err(e) => Err(storage_failure("writing contacts", &e)),
        }
    }

    /// Rebuilds the copy from fresh reads, if friends are published.
    async fn refresh_copy(&self, id: &AgentId) -> Result<(), Problem> {
        let (identity, _) = self.read_identity_tagged(id).await?;
        if identity.public.contains(&Field::Friends) {
            self.write_copy(id, &identity).await?;
        }
        Ok(())
    }
}
