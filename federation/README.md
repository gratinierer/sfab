# Federation

- `world.yaml`: this world's name, served with its domain and contact at
  `GET /v1/federation/world`.
- `worlds.yaml`: the worlds this one has approved, served at `GET /v1/federation/worlds`. The owner
  approves a world by adding it here, committing and deploying.

The API Lambda compiles both in (`lambda/world/federation/`), and its tests refuse a malformed file.
`docs/federation.md` is the manual, for both sides of a request.
