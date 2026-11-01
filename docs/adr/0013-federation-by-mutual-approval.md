# Federation needs both owners' approval, recorded in the repo

A world asks to federate through a public route, and the request is stored with proof of work, capped and expiring, and summarised to the owner once a day. The owner checks the other domain by hand, then approves by committing to `federation/worlds.yaml` and deploying. The other owner does the same. A world's ID is its domain, so HTTPS proves who it is, like email and Mastodon, and a random caller gets nowhere.

## Considered Options

- **Open federation, like email:** no approval, but the owner explicitly doesn't want to federate with a random caller.
- **An approval route or script:** a route is something to attack, and a script adds tooling for a rare action, while git already gives review and history.

## Consequences

- The first version is mostly a trusted directory, because it only covers public profiles, which anyone can read. So it has no signing keys.
- Each approval needs a deploy, which suits something this rare.
- Real federation can't be tested until a second world exists, so it's tested against a fake one.
