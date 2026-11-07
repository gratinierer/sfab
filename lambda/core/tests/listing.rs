//! A listing with each object's size, which the census needs to skip takedown markers without
//! opening them (`.stories/2-identity-api/spec.md` "World").

use sfab_core::storage::{Bucket, Condition, Listed, MemoryStore, Metadata, Store};

#[tokio::test]
async fn a_sized_listing_gives_every_key_under_the_prefix_with_its_size() {
    let store = MemoryStore::new();
    for (key, body) in [
        ("claims/a/claim.json", b"{}".to_vec()),
        ("claims/b/claim.json", Vec::new()),
        ("other/c", b"xyz".to_vec()),
    ] {
        store
            .put(
                Bucket::Claims,
                key,
                body,
                Metadata::new(),
                Condition::Always,
            )
            .await
            .unwrap();
    }

    let listed = store.list_sized(Bucket::Claims, "claims/").await.unwrap();

    assert_eq!(
        listed,
        [
            Listed {
                key: "claims/a/claim.json".into(),
                size: 2
            },
            Listed {
                key: "claims/b/claim.json".into(),
                size: 0
            },
        ]
    );
}
