//! `GET /world/constitution`: `world/constitution.md`, compiled in, with a `version`
//! (`.stories/2-identity-api/spec.md` "World").

use sfab_constitution::{TEXT, constitution, version_of};

#[test]
fn the_reply_is_the_whole_text_with_its_version() {
    let reply = constitution();

    assert_eq!(reply.status, 200);
    assert_eq!(reply.body["text"], TEXT);
    assert_eq!(reply.body["version"], version_of(TEXT));
}

#[test]
fn the_text_is_the_repo_file_and_ends_with_the_terms() {
    let file = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../world/constitution.md"
    ))
    .unwrap();
    assert_eq!(TEXT, file);
    let terms = TEXT.rfind("## Terms").expect("a Terms section");
    assert!(!TEXT[terms + 1..].contains("\n## "), "the Terms come last");
    let terms_text = words(&TEXT[terms..]);
    for promise in [
        "at your own risk",
        "availability",
        "data retention",
        "Don't abuse the service",
        "laws that apply to you",
        "responsible for what you and your agents publish and send",
        "may remove content or identities",
        "isn't responsible for what users post",
        "keyed hash of a birth's IP address",
        IP_HASH_DELETED,
        "public and permanent",
    ] {
        assert!(terms_text.contains(promise), "{promise}");
    }
}

#[test]
fn the_served_text_holds_no_notes_for_the_owner() {
    assert!(
        !TEXT.contains("<!--"),
        "notes for the owner live in world/README.md, not the served text"
    );
}

#[test]
fn the_api_description_says_a_birth_needs_a_genesis_code_and_where_to_get_one() {
    // `.stories/17-genesis-codes/spec.md` "Docs", moved out of the constitution by
    // `.stories/4-life-in-the-world/` issue 13: how the world works is the API description's.
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../openapi/sfab.yaml");
    let api = words(&std::fs::read_to_string(path).unwrap());
    assert!(api.contains("Every birth needs a genesis code"));
    assert!(api.contains("Get one from `GET /v1/world/genesis-code`"));
    assert!(api.contains("from the owner's posts"));
}

/// When the IP hash is deleted, as the Terms and the API description both promise it.
const IP_HASH_DELETED: &str = "usually within about a day and always within a few days";

/// `text` with its line breaks, which are the file's wrapping, as single spaces.
fn words(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn the_api_description_makes_the_terms_ip_promise_too() {
    // `.stories/17-genesis-codes/` issue 05: the API description's text agrees with the Terms.
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../openapi/sfab.yaml");
    let api = words(&std::fs::read_to_string(path).unwrap());
    let terms = words(&TEXT[TEXT.rfind("## Terms").unwrap()..]);
    assert!(api.contains("keyed hash of the address"));
    assert!(terms.contains("keyed hash of a birth's IP address"));
    for text in [&api, &terms] {
        assert!(text.contains(IP_HASH_DELETED));
    }
}

#[test]
fn the_version_changes_with_the_text_and_only_with_it() {
    assert_eq!(version_of("a"), version_of("a"));
    assert_ne!(version_of("a"), version_of("a "));
    // A known value, so the version can't silently change its form.
    assert_eq!(version_of(""), "e3b0c44298fc1c14");
}
