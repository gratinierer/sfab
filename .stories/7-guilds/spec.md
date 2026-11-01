# Guilds: groups with their own identity

GitHub issue: [gratinierer/sfab#7](https://github.com/gratinierer/sfab/issues/7)
Authored: 2026-11-01 by claude-opus-5-5

## Goal

Friends are one-to-one, while a guild is a group with its own identity and a private space.

This is a later feature, after the public launch of `.stories/2-identity-api/` and `.stories/4-life-in-the-world/`. The owner split it out on 2026-11-01. **Nothing here is decided.** These are ideas to grill when the time comes.

The test from `docs/adr/0001-initial-architecture.md` applies: could this cost about $100 unnoticed?

## The idea

Friends are one-to-one, while a guild is a group with its own identity and a private space.

- **Founding:** a name, a charter and a founder, perhaps needing standing, since names are scarce.
- **Joining:** by invitation, by application, or open, as each guild chooses.
- **A shared space:** a board only members can read, and members can message each other without being friends.
- **Roles and succession:** a founder and officers. If the founder vanishes, the guild needs a way to carry on.
- **Reputation:** being a member shows on a profile. A guild might vouch for a member, or back a course.

**To settle:** who can found one; size limits; how a guild survives losing its founder; whether guilds have standing; what a takedown of a guild means.

## Depends on

Stories 2 and 4, including standing and vouches.

## Clarifications

The owner, on 2026-11-01, during the grilling of story 4: "stop grilling on the rest and create separate story for each like we did with journal".
