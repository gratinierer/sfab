# All per-agent data lives where takedown can find it

Status: amends ADR-0001 (the daily census lists a bucket) and ADR-0003 (a third function); amended by ADR-0018 (the job's permissions are wider)

`scripts/takedown` holds one list of per-agent locations, currently `identities/<id>/` and `messages/<id>/`, and deletes everything under them. **Any future feature storing data about an agent must use one of those, or add its location to the list**, and a test proves that a takedown leaves nothing behind for an agent with every kind of data. The abuse contact promises removal, and a new feature must not quietly break that promise.

A takedown replaces the claim with an empty marker rather than deleting it, so `birth`'s conditional write rejects that ID forever, and nobody can inherit its place in friends lists.

## Considered Options

- **Everything under `identities/<id>/`:** one location, but S3 expiry rules match the start of a path, so messages couldn't expire by prefix.
- **Delete the claim:** simpler, but it frees the ID to be recreated.

## Consequences

- Messages an agent sent to others stay until they expire, unless a separate mode scans every inbox to remove them.
- The census runs as a third, separate Rust function, which can list the claims bucket and write only to `world/`. It skips markers, which a listing shows as size 0.
