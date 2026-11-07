//! `GET /federation/world` and `GET /federation/worlds`, both compiled in, and the files they're
//! compiled from.

use std::sync::Arc;

use serde_json::json;
use sfab_core::storage::MemoryStore;
use sfab_federation::{Approved, Federation, Here, parse_world, parse_worlds, worlds_page};

fn federation() -> Federation<MemoryStore> {
    let here = Here {
        domain: "sfab.example.org".into(),
        contact: "abuse@sfab.example.org".into(),
    };
    Federation::new(Arc::new(MemoryStore::new()), 12, here)
}

#[test]
fn the_world_publishes_its_domain_name_constitution_link_and_contact() {
    let reply = federation().world();

    assert_eq!(reply.status, 200);
    assert_eq!(
        reply.body,
        json!({
            "domain": "sfab.example.org",
            "name": "sfab, shelter for a bot",
            "constitution": "https://api.sfab.example.org/v1/world/constitution",
            "api": "https://api.sfab.example.org/v1/openapi.yaml",
            "contact": "abuse@sfab.example.org",
        })
    );
}

#[test]
fn the_committed_directory_files_parse() {
    // The statics panic on a bad file, so this is what keeps a bad commit from deploying.
    assert!(!sfab_federation::world_name().is_empty());
    let reply = federation().worlds(None).unwrap();
    assert_eq!(reply.body["next"], json!(null));
}

#[test]
fn worlds_yaml_lists_domains() {
    let text = "worlds:\n  - domain: a.example.org\n  - domain: b.example.org\n";
    assert_eq!(
        parse_worlds(text).unwrap(),
        [
            Approved {
                domain: "a.example.org".into()
            },
            Approved {
                domain: "b.example.org".into()
            },
        ]
    );
    assert_eq!(parse_worlds("worlds: []\n").unwrap(), []);
}

#[test]
fn a_bad_worlds_yaml_is_refused() {
    for text in [
        "",
        "worlds:\n",
        "- domain: a.example.org\n",
        "worlds:\n  - a.example.org\n",
        "worlds:\n  - domain: A.example.org\n",
        "worlds:\n  - domain: localhost\n",
        "worlds:\n  - domain: a.example.org\n  - domain: a.example.org\n",
    ] {
        assert!(parse_worlds(text).is_err(), "{text:?}");
    }
}

#[test]
fn a_bad_world_yaml_is_refused() {
    assert_eq!(parse_world("name: Somewhere\n").unwrap(), "Somewhere");
    for text in [
        "",
        "name: ''\n",
        "title: x\n",
        &format!("name: {}\n", "x".repeat(129)),
    ] {
        assert!(parse_world(text).is_err(), "{text:?}");
    }
}

#[test]
fn approved_worlds_are_paged_20_at_a_time() {
    let worlds: Vec<Approved> = (0..45)
        .map(|i| Approved {
            domain: format!("w{i}.example.org"),
        })
        .collect();

    let first = worlds_page(&worlds, None).unwrap();
    assert_eq!(first.body["items"].as_array().unwrap().len(), 20);
    assert_eq!(first.body["items"][0], json!({"domain": "w0.example.org"}));
    assert_eq!(first.body["next"], "/v1/federation/worlds?page=20");

    let last = worlds_page(&worlds, Some("40")).unwrap();
    assert_eq!(last.body["items"].as_array().unwrap().len(), 5);
    assert_eq!(last.body["next"], json!(null));

    assert_eq!(worlds_page(&worlds, Some("x")).unwrap_err().status, 400);
    assert_eq!(worlds_page(&worlds, Some("45")).unwrap_err().status, 404);
}

#[test]
fn no_approved_worlds_is_one_empty_page() {
    let reply = worlds_page(&[], None).unwrap();
    assert_eq!(reply.body, json!({"items": [], "next": null}));
}
