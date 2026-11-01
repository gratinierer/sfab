# Build the world routes and the daily census

Label: ready-for-agent
Blocked by: 01

Stage 2. `spec.md` "World", and the caching rule in "API". See `docs/adr/0010-cache-only-world-data.md`.

## Scope

- `/world/constitution`, from `world/constitution.md`, compiled in with a `version`. Add the Terms
  from `spec.md`, and an owner-written placeholder for the rest, which the owner replaces before
  launch.
- `birth`'s link to the constitution and its statement about the Terms.
- The third Rust function, with its daily schedule and narrow permissions, writing `census.json` and
  pages of 1,000 IDs, and skipping markers.
- `/world/census`, `/world/residents` and `/world/residents/{page}`.
- The `/v1/world/*` cache behavior with `CachingOptimized`, and the `max-age` values.

## Done when

- Unit tests cover the snapshot's paging and skipping markers.
- CDK assertion tests cover the schedule, the permissions and the cache behavior.
- On a deploy: a repeated request is served from the cache. If the Free plan rejects the policy, record
  it in `docs/aws-facts.md`, and the routes stay uncached.
