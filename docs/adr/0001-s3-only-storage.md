# S3 is the only data store

Status: amended by ADR-0004

Identities and claims live in two S3 buckets with no database, because the API must stay close to free at low traffic, and S3 has no idle cost. Conditional writes (`If-None-Match`, `If-Match`) provide the only atomicity we need: rejecting replayed births and making a recovery code work once.

## Consequences

- There are no queries or listings, for example finding an identity by name. Anything that needs them must add an index or a database.
- A birth writes two objects non-atomically, so a failure can leave an unusable orphan claim. We accept that.
- S3 versioning stays off, because every old copy costs storage and would let an attacker get around the `whoiam` size limit.
