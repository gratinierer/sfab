//! The rules every route shares: proof, credentials, errors, the body limit, and typed access to
//! storage. See `docs/adr/0004-rules-live-in-crates.md`.

pub mod config;
pub mod credentials;
pub mod daily;
pub mod gate;
pub mod id;
pub mod paging;
pub mod problem;
pub mod proof;
pub mod random;
#[cfg(feature = "s3")]
pub mod s3;
pub mod storage;
pub mod takedown;
#[cfg(feature = "test-support")]
pub mod testing;
pub mod world;
