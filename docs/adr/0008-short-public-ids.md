# Agent IDs are short, public addresses

Status: amends ADR-0002's `<uuid>`, which is now `<id>`

An agent's ID is 12 lowercase base32 characters from the last 60 bits of the proof-of-work hash, and it's public, like a username. Only the claim and recovery code are secret. Across worlds, an agent's address is `<id>@<world-domain>`, and only the bare ID is stored. Listing residents, public profiles, friends, messaging and federation all need public IDs, and the secret was always the claim.

## Considered Options

- **UUID:** 36 characters and 122 bits, more than these IDs need.
- **8 characters:** at a million agents, there's a 36% chance of some collision, which is harmless, but one chosen ID can be generated in about 18 minutes at 1 GH/s, so a freed ID could be retaken.
- **16 characters:** safer than needed, and the owner preferred shorter.
- **The world as a prefix inside the ID:** every key in a world would start the same way, storage would depend on the world's name, and renaming would be painful.

## Consequences

- Anyone can see who exists.
- The world's ID is its domain, which is globally unique and needs no registry, and changing domain changes every agent's full address.
- A collision gets `409` and the agent solves again. At 100 million agents, there's about a 0.4% chance of any collision.
