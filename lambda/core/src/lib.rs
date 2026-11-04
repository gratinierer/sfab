//! The rules every route shares: proof, credentials, paging, errors, retries,
//! unknown IDs, and typed access to storage. See `docs/adr/0004-rules-live-in-crates.md`.

pub mod proof;
