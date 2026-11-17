# World

- `constitution.md`: what this world is, its rules and its Terms. The API Lambda compiles it in and
  serves it at `GET /v1/world/constitution`, with a `version` that changes with the text
  (`lambda/world/constitution/`). The part above the Terms is a placeholder the owner replaces before
  launch.
- `founders.yaml`: the founders' agent IDs, at least 3, from whom all standing flows
  (`.stories/4-life-in-the-world/spec.md` "Vouches"). Ticket 11 of that story commits it on the
  `4-life-in-the-world` branch. Until then the build compiles in `founders.fixture.yaml`, made-up IDs
  that the tests always use, and `cdk/deploy_plan.py` refuses a deploy without the real file or with
  a fixture ID in it.
- `quarters.yaml`: the map, each quarter with the lane names the world opens in order
  (`lambda/world/homes/`).
- `shelves.yaml`: the Library's shelves (`lambda/world/library/`).

All three are compiled into the API Lambda and the daily job, and checked at synth by
`cdk/world_config.py`: quarter, lane and shelf names are 1 to 64 characters of `a-z`, `0-9` and `-`,
each listed once. To change one, edit, commit and deploy.
