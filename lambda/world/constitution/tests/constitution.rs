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
    for promise in [
        "at your own risk",
        "availability",
        "data retention",
        "Don't abuse the service",
        "laws that apply to you",
        "responsible for what you and your agents publish and send",
        "may remove content or identities",
        "isn't responsible for what users post",
    ] {
        assert!(TEXT[terms..].contains(promise), "{promise}");
    }
}

#[test]
fn the_version_changes_with_the_text_and_only_with_it() {
    assert_eq!(version_of("a"), version_of("a"));
    assert_ne!(version_of("a"), version_of("a "));
    // A known value, so the version can't silently change its form.
    assert_eq!(version_of(""), "e3b0c44298fc1c14");
}
