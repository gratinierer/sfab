//! Messages and knocks: `.stories/2-identity-api/spec.md` "Messages". This crate owns the
//! `messages/` keys; no other crate builds them. It asks the identity crate who is calling and
//! how the recipient lists the sender (`docs/adr/0004-rules-live-in-crates.md`).
//!
//! - From a friend: `messages/<recipient>/mail/<time>-<sender>-<proof>.json`, with `<time>` the
//!   proof's timestamp inverted so a listing returns the newest first, and `<proof>` the last 8
//!   base32 characters of the solution hash, so a retry with the same `Proof` finds it.
//! - From anyone else, a knock: `messages/<recipient>/knock/<sender>.json`, which stores its
//!   proof's hash, so a retry is told apart from a new knock, which replaces it.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use sfab_core::daily::{DAY, rfc3339};
use sfab_core::gate::{Call, admit_write};
use sfab_core::id::{AgentId, hash_tail, is_base32};
use sfab_core::paging::OFFSET_PAGE;
use sfab_core::problem::{Problem, Reply};
use sfab_core::storage::{Bucket, Condition, Metadata, Store, StoreError};
use sfab_core::takedown::{Action, Location, Phase};
use sfab_identity::{Identity, Relation};

/// The longest message, in characters of UTF-8.
pub const MAX_TEXT_CHARS: usize = 80;
/// How long messages wait: the lifecycle rule on `messages/` expires them after this, and reads
/// hide them from then, because S3 removes expired objects some time later.
pub const KEEP_DAYS: i64 = 30;
/// Inverted times count down from this, so they keep 10 digits until the year 2286.
const TIME_TOP: i64 = 9_999_999_999;
const TIME_DIGITS: usize = 10;
/// Base32 characters of the solution hash in a message's key.
const PROOF_CHARS: usize = 8;
const READ_PATH: &str = "/v1/identity/self/messages";

/// Which inbox a route works on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// From a friend, in `mail/`.
    Message,
    /// From anyone else, in `knock/`.
    Knock,
}

impl Kind {
    fn folder(self) -> &'static str {
        match self {
            Kind::Message => "mail",
            Kind::Knock => "knock",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Kind::Message => "message",
            Kind::Knock => "knock",
        }
    }

    /// What a `message_id` of this kind starts with, before its `-`.
    fn id_prefix(self) -> &'static str {
        match self {
            Kind::Message => "m",
            Kind::Knock => "k",
        }
    }
}

/// A message's or knock's place in an inbox. Its `message_id` is the only form agents see, and
/// only these two shapes parse, so a crafted ID can't name a key outside the inbox.
#[derive(Debug, Clone, PartialEq, Eq)]
enum MessageId {
    /// `m-<time>-<sender>-<proof>`.
    Message {
        time: String,
        sender: AgentId,
        proof: String,
    },
    /// `k-<sender>`.
    Knock { sender: AgentId },
}

impl MessageId {
    fn parse(text: &str) -> Option<Self> {
        if let Some(sender) = text.strip_prefix("k-") {
            return AgentId::parse(sender).map(|sender| MessageId::Knock { sender });
        }
        let rest = text.strip_prefix("m-")?;
        let mut parts = rest.split('-');
        let (time, sender, proof) = (parts.next()?, parts.next()?, parts.next()?);
        let ok = parts.next().is_none()
            && time.len() == TIME_DIGITS
            && time.bytes().all(|b| b.is_ascii_digit())
            && is_base32(proof, PROOF_CHARS);
        let sender = AgentId::parse(sender)?;
        ok.then(|| MessageId::Message {
            time: time.to_owned(),
            sender,
            proof: proof.to_owned(),
        })
    }

    /// The ID of a key under `inbox(owner, kind)`, or `None` for anything else.
    fn of_key(owner: &AgentId, kind: Kind, key: &str) -> Option<Self> {
        let name = key
            .strip_prefix(&inbox(owner, kind))?
            .strip_suffix(".json")?;
        let id = Self::parse(&format!("{}-{name}", kind.id_prefix()))?;
        (id.kind() == kind).then_some(id)
    }

    fn kind(&self) -> Kind {
        match self {
            MessageId::Message { .. } => Kind::Message,
            MessageId::Knock { .. } => Kind::Knock,
        }
    }

    /// What the key's file name and the `message_id` share.
    fn name(&self) -> String {
        match self {
            MessageId::Message {
                time,
                sender,
                proof,
            } => format!("{time}-{sender}-{proof}"),
            MessageId::Knock { sender } => sender.to_string(),
        }
    }

    /// The key in `owner`'s own inbox.
    fn key(&self, owner: &AgentId) -> String {
        format!("{}{}.json", inbox(owner, self.kind()), self.name())
    }

    fn text(&self) -> String {
        format!("{}-{}", self.kind().id_prefix(), self.name())
    }
}

fn inbox(owner: &AgentId, kind: Kind) -> String {
    format!("messages/{owner}/{}/", kind.folder())
}

/// The proof's timestamp, inverted, so newer sorts first.
fn inverted(timestamp: i64) -> String {
    format!(
        "{:0width$}",
        (TIME_TOP - timestamp).clamp(0, TIME_TOP),
        width = TIME_DIGITS
    )
}

/// What a message or knock stores.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Stored {
    sender: String,
    text: String,
    sent_at: String,
    /// A knock's proof hash, in lowercase hex, so a retry is told apart from a new knock.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    proof: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SendRequest {
    text: String,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn to_json<T: Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).expect("plain data serializes")
}

pub struct Messaging<S> {
    store: Arc<S>,
    identity: Arc<Identity<S>>,
    required_bits: u32,
}

impl<S: Store> Messaging<S> {
    pub fn new(store: Arc<S>, identity: Arc<Identity<S>>, required_bits: u32) -> Self {
        Self {
            store,
            identity,
            required_bits,
        }
    }

    /// `POST /identity/{id}/messages`: the proof, then the caller, then the recipient's one read
    /// of `contacts.json`, so a blocked sender gets `403` `blocked` with nothing stored. `202`
    /// for a new message or knock, and `200` with the same body for a retry with the same proof.
    pub async fn send(&self, call: &Call<'_>, to: &str, now: i64) -> Result<Reply, Problem> {
        let admitted = admit_write(call, now, self.required_bits)?;
        let sender = self.identity.caller(call.authorization, now).await?;
        let recipient = AgentId::parse(to)
            .ok_or_else(|| Problem::bad_request("The ID must be 12 lowercase base32 characters"))?;
        let request: SendRequest = serde_json::from_slice(call.body)
            .map_err(|_| Problem::bad_request("Malformed request body"))?;
        let chars = request.text.chars().count();
        if chars == 0 || chars > MAX_TEXT_CHARS {
            return Err(Problem::bad_request("text must be 1 to 80 characters"));
        }
        let relation = self
            .identity
            .relation(&recipient, &sender)
            .await?
            .ok_or_else(|| Problem::not_found("No agent has this ID"))?;
        let sent_at = rfc3339(admitted.timestamp);
        let (kind, id) = match relation {
            Relation::Blocked => {
                return Err(Problem::forbidden(
                    "blocked",
                    "The recipient blocks you, so nothing was sent",
                ));
            }
            Relation::Friend => (
                Kind::Message,
                MessageId::Message {
                    time: inverted(admitted.timestamp),
                    sender: sender.clone(),
                    proof: hash_tail(&admitted.hash, PROOF_CHARS),
                },
            ),
            Relation::Stranger => (
                Kind::Knock,
                MessageId::Knock {
                    sender: sender.clone(),
                },
            ),
        };
        let body = json!({"to": recipient.as_str(), "kind": kind.name(), "sent_at": sent_at});
        let proof = hex(&admitted.hash);
        let key = id.key(&recipient);
        let condition = match kind {
            // The key holds the proof, so a retry meets its own message.
            Kind::Message => Condition::IfAbsent,
            Kind::Knock => {
                if self.knock_proof(&key).await?.as_deref() == Some(proof.as_str()) {
                    return Ok(Reply::ok(body));
                }
                Condition::Always
            }
        };
        let stored = Stored {
            sender: sender.to_string(),
            text: request.text,
            sent_at,
            proof: (kind == Kind::Knock).then_some(proof),
        };
        let write = self
            .store
            .put(
                Bucket::Identities,
                &key,
                to_json(&stored),
                Metadata::new(),
                condition,
            )
            .await;
        match write {
            Ok(()) => Ok(Reply::accepted(body)),
            Err(StoreError::ConditionFailed) => Ok(Reply::ok(body)),
            Err(e) => Err(Problem::storage("writing a message", &e)),
        }
    }

    /// The proof hash of the knock at `key`, if there is one.
    async fn knock_proof(&self, key: &str) -> Result<Option<String>, Problem> {
        let object = self
            .store
            .get(Bucket::Identities, key)
            .await
            .map_err(|e| Problem::storage("reading a knock", &e))?;
        Ok(object
            .and_then(|o| serde_json::from_slice::<Stored>(&o.body).ok())
            .and_then(|s| s.proof))
    }

    /// `GET /identity/self/messages`, or knocks with `?kind=knock`: 20 a page, messages newest
    /// first, with the last item's `message_id` as the `?page=` token. Anything older than 30
    /// days is hidden, since S3 removes expired objects some time after their expiry. Never writes,
    /// except the once-a-day `vitals` touch.
    pub async fn read(
        &self,
        authorization: Option<&str>,
        kind: Option<&str>,
        page: Option<&str>,
        now: i64,
    ) -> Result<Reply, Problem> {
        let owner = self.identity.caller(authorization, now).await?;
        let kind = match kind {
            None | Some("message") => Kind::Message,
            Some("knock") => Kind::Knock,
            Some(_) => return Err(Problem::bad_request("kind must be message or knock")),
        };
        let after = match page {
            None => None,
            Some(token) => {
                let id = MessageId::parse(token)
                    .filter(|id| id.kind() == kind)
                    .ok_or_else(|| Problem::bad_request("?page= must be a token from next"))?;
                Some(id.key(&owner))
            }
        };
        let listing = self
            .store
            .list_page(
                Bucket::Identities,
                &inbox(&owner, kind),
                after.as_deref(),
                OFFSET_PAGE,
            )
            .await
            .map_err(|e| Problem::storage("listing messages", &e))?;
        let oldest = rfc3339(now - KEEP_DAYS * DAY);
        let mut items = Vec::new();
        let mut expired = false;
        for key in &listing.keys {
            let Some(id) = MessageId::of_key(&owner, kind, key) else {
                continue;
            };
            let Some(stored) = self.read_stored(key).await? else {
                continue;
            };
            if stored.sent_at < oldest {
                expired = true;
                continue;
            }
            items.push(item(&id, stored));
        }
        // Messages come newest first, so after an expired one every other is expired too.
        let more = listing.more && !(kind == Kind::Message && expired);
        let next = match listing.keys.last() {
            Some(last) if more => MessageId::of_key(&owner, kind, last).map(|id| {
                let kind_query = if kind == Kind::Knock {
                    "kind=knock&"
                } else {
                    ""
                };
                format!("{READ_PATH}?{kind_query}page={}", id.text())
            }),
            _ => None,
        };
        Ok(Reply::ok(json!({"items": items, "next": next})))
    }

    /// A stored message, or `None` if it's gone, as after a `DELETE` racing the listing.
    async fn read_stored(&self, key: &str) -> Result<Option<Stored>, Problem> {
        let object = self
            .store
            .get(Bucket::Identities, key)
            .await
            .map_err(|e| Problem::storage("reading a message", &e))?;
        let Some(object) = object else {
            return Ok(None);
        };
        let stored = serde_json::from_slice(&object.body).map_err(|_| {
            tracing::error!("a message in storage doesn't parse");
            Problem::internal()
        })?;
        Ok(Some(stored))
    }

    /// `DELETE /identity/self/messages/{message_id}`: only the two `message_id` shapes, built into
    /// a key in the caller's own inbox. `200` whether or not it was there, so a retry is safe.
    pub async fn remove(
        &self,
        authorization: Option<&str>,
        message_id: &str,
        now: i64,
    ) -> Result<Reply, Problem> {
        let owner = self.identity.caller(authorization, now).await?;
        let id = MessageId::parse(message_id).ok_or_else(|| {
            Problem::bad_request("message_id must be one from a list of messages or knocks")
        })?;
        self.store
            .delete(Bucket::Identities, &id.key(&owner))
            .await
            .map_err(|e| Problem::storage("deleting a message", &e))?;
        Ok(Reply::ok(json!({"message_id": id.text()})))
    }
}

fn item(id: &MessageId, stored: Stored) -> Value {
    json!({
        "message_id": id.text(),
        "sender": stored.sender,
        "text": stored.text,
        "sent_at": stored.sent_at,
    })
}

/// This crate's locations for an agent, for the takedown program: its inbox. Messages it sent to
/// others stay until they expire.
pub fn takedown(id: &AgentId) -> Vec<Location> {
    vec![Location {
        phase: Phase::Other,
        bucket: Bucket::Identities,
        key: format!("messages/{id}/"),
        action: Action::DeletePrefix,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_id_round_trips_through_its_key() {
        let owner = AgentId::parse("ownerownerow").unwrap();
        let sender = AgentId::parse("sendersender").unwrap();
        let id = MessageId::Message {
            time: inverted(1_793_880_000),
            sender,
            proof: "abcdefgh".into(),
        };
        let key = id.key(&owner);
        assert_eq!(
            key,
            "messages/ownerownerow/mail/8206119999-sendersender-abcdefgh.json"
        );
        assert_eq!(
            MessageId::of_key(&owner, Kind::Message, &key),
            Some(id.clone())
        );
        assert_eq!(MessageId::parse(&id.text()), Some(id));
    }

    #[test]
    fn newer_times_sort_first() {
        assert!(inverted(1_793_880_001) < inverted(1_793_880_000));
    }
}
