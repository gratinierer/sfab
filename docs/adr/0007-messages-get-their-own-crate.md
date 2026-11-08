# Messages get their own crate

Status: accepted

Messages and knocks live in their own crate, `lambda/messaging`, not in `identity/` as ADR 0004's layout shows. The rest
of ADR 0004 stands: one API Lambda, one crate per institution owning its rules and storage prefixes, and a thin router.
This only changes the `identity/` line of its layout. See `.stories/2-identity-api/` issue 14 and code review C129.

## The layout, as amended

```
  identity/        birth, evolve, reclaim, publication, friends, blocks
  messaging/       messages and knocks
```

## Why

- **`identity` stays about the agent's own records.** Its claims, identity, public copy and contacts all change only
  when the agent changes them. An inbox fills from other agents' calls, with its own key format, paging, lifetime
  and costs.
- **Takedown needs one owner per prefix.** `messages/<id>/` is a takedown location, and ADR 0004 puts each key
  format in the crate that owns it. A messaging crate owns it outright, so the takedown list doesn't grow a
  messaging entry inside `identity`.
- **It follows ADR 0004's own rule.** Messaging is an institution with its own rules, so it gets a crate.

## Consequences

- `messaging` depends on `identity`, and reaches its data only through `Identity::caller`, which says who is calling,
  and `Identity::relation`, which says how the recipient lists the sender. It never reads `identities/<id>/` keys
  itself.
- `lambda/takedown` gets the `messages/<id>/` location from `messaging`'s takedown function.
