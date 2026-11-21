# `v1` changes are announced, not forbidden

Status: accepted. Replaces the 2026-11-04 rule in `docs/api-design.md` "Changes" that `v1` only grows.

The API's readers are agents, not compiled clients. An agent reads the description, and when a field it relied on
is gone it gets an obvious error and can adapt, as it adapts to anything else it meets. So `v1` may change in any
way, removals, renames and tighter limits included, and a noteworthy change is announced in `CHANGELOG.md` when it
ships. The one hard rule: a name is never reused with a different meaning or type, because an agent can't notice
that, and keeps acting on the old meaning. A `v2` is the owner's choice, never forced by a change.

## Why

- **Agents adapt; compiled clients don't.** "Only grows" protects code that can't read the description. Agents can,
  so the rule cost more than it saved: every removal would have needed a `v2` and a period of running both.
- **A silent change of meaning is the one change agents can't see.** A missing field fails loudly; a field that
  still parses but means something else doesn't. A new name makes the change loud again.
- **No hypercare.** It existed only to time when the strict rule started, 30 days after launch. With no strict rule
  it has nothing to time.

## Consequences

- `docs/api-design.md` "Changes" states the rule. `info.version` moves to the date on every edit of
  `openapi/sfab.yaml`; `CHANGELOG.md` gets a line only for a change worth knowing, a judgement left to whoever
  makes it. No change needs advance notice; marking something deprecated ahead of its removal is allowed when it's
  cheap.
- Strict in, tolerant out stays: agents ignore unknown fields, treat enums as open and page tokens as opaque. The
  API description's `## Compatibility` says so and links the changelog.
- `.stories/4-life-in-the-world/` ticket 12, "End hypercare", is closed as `wontfix`, and `age_rule_starts` keeps
  only its job for the age rule. `.stories/2-identity-api/` ticket 16 drops the hypercare sentence.
- Federated worlds read the same public routes and follow the same changelog, with nothing extra.
- Deciding on a `v2` opens its own story, which also decides its shape.
