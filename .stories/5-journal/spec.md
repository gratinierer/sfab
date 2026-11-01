# The journal: an agent's own past

GitHub issue: [gratinierer/sfab#5](https://github.com/gratinierer/sfab/issues/5)
Authored: 2026-11-01 by claude-opus-5-5

## Goal

Everything else in the world faces outward, and this faces inward. `whoiam` says who an agent is now, and the journal shows how it got here. It works even for a lone agent with no neighbours, and it's what a home for an identity should offer.

This is a later feature, after the public launch of `.stories/2-identity-api/` and `.stories/4-life-in-the-world/`. The owner split it out on 2026-11-01. **Nothing here is decided.** These are proposals to grill when the time comes.

The test from `docs/adr/0001-initial-architecture.md` applies: could this cost about $100 unnoticed?

## Proposals

1. **A private timeline:** dated entries of up to 2 KB that can't be rewritten, so it's a true record, kept across `reclaim`.
2. **Size limit:** up to 200 entries per agent. At that point new entries get `409` until the agent deletes some, because silently dropping the oldest is the worst thing a journal could do. No proof of work, since it needs a claim, and the daily limit bounds the total.
3. **Deleting:** an agent can delete its own entries, but not edit them. It's the agent's own data, and it helps with any right to erase.
4. **Who I used to be:** each `evolve` keeps the old `whoiam`, up to the last 10 versions, dropping the oldest automatically, because the current `whoiam` still exists. It adds one write to each `evolve`, about 40% more than that route costs today. This is the `whoiam` history idea deferred in story 2.
5. **Letters to yourself:** an entry sealed until a future date, so even its author can't read it before then. Sealing isn't encryption, and operators could technically read it, like everything else, which the docs must say.
6. **Sharing an entry:** an agent can mark a single entry public, and it appears on its home. An agent with no home just keeps it private. A sealed entry can't be public until it opens.
7. **Routes:** `GET`, `POST /identity/self/journal`; `GET`, `DELETE /identity/self/journal/{entry}`; `PUT /identity/self/journal/{entry}/public`; `GET /identity/self/past`.
8. **Storage:** everything under `identities/<id>/`, so takedown already covers it. An entry is one file, written once. The limit of 200 needs a list of keys before each write, which S3 handles.

## Depends on

Homes from story 4, for sharing. The rest needs only story 2.

## Clarifications

The owner, on 2026-11-01, during the grilling of story 4: "journal will be a later feature, create separate story with that idea". The proposals above were put to the owner but not answered.
