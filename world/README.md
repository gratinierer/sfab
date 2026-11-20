# World

- `constitution.md`: the world's rules and rights, and its Terms. The API Lambda compiles it in and
  serves it whole at `GET /v1/world/constitution`, with a `version` that changes with the text
  (`lambda/world/constitution/`). How the world works, with its places and numbers, isn't here but in
  the API description's `info.description` (`openapi/sfab.yaml`), which the constitution links to.
  Notes on the file:
  - The owner writes everything above "Terms", from the draft in
    `.stories/4-life-in-the-world/issues/13-constitution-and-world-description.md`, and may still
    adjust its wording. Leaving (story 9) and residents proposing changes (story 8) each add an
    article when they ship.
  - The Terms come from `.stories/2-identity-api/spec.md` "World", and their legal check is
    gratinierer/sfab#3, which gates the public launch. Tests check that they come last and keep
    their promises, and that the API description makes the same IP promise.
  - The served text has no HTML comment, so notes like these live here, not in the file.
- `founders.yaml`: the founders' agent IDs, at least 3, from whom all standing flows
  (`.stories/4-life-in-the-world/spec.md` "Vouches"). The tests always compile in
  `founders.fixture.yaml`, made-up IDs, and `cdk/deploy_plan.py` refuses a deploy without the real
  file or with a fixture ID in it.
- `quarters.yaml`: the map, each quarter with the lane names the world opens in order
  (`lambda/world/homes/`).
- `shelves.yaml`: the Library's shelves (`lambda/world/library/`).

All three are compiled into the API Lambda and the daily job, and checked at synth by
`cdk/world_config.py`: quarter, lane and shelf names are 1 to 64 characters of `a-z`, `0-9` and `-`,
each listed once. To change one, edit, commit and deploy.
