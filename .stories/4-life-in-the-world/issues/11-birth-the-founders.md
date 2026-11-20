# Birth the founders

Label: ready-for-human
Blocked by: 17-genesis-codes#04

`spec.md` "Launch checklist", step 2, and "Vouches: trust". A deploy of story 4's code needs at least 3 real
founders; the code itself was written before, against a fixture of made-up IDs (ticket 01).

## Scope

- On the private world, before story 2's drills on the owner's instruction (see Comments), birth at least 3 founder agents and keep their claims and
  recovery codes safe. Each birth needs a genesis code: issue one `--no-ip-limit` code per founder with
  `genesis-codes issue --label founders --count <n> --no-ip-limit` (`17-genesis-codes#04`), so the births from
  one IP don't meet the 4-hour IP rule.
- Commit their IDs to `world/founders.yaml` on `main`, where story 4's code already is (`spec.md` "Launch
  checklist", step 2). IDs are public, so committing them is fine.
- The founders make no other writes before launch, so nothing of theirs carries into it.

## Done when

- `world/founders.yaml` holds at least 3 real IDs, each of which `GET /identity/self` accepts with its claim.

## Comments

- 2026-11-20: born before story 2's drills, not after them (`Blocked by: 2-identity-api#10`), on the owner's
  instruction. Story 4's code reached `main` and `production` on 2026-11-17, ahead of this ticket, so every deploy
  but `SfabDeployer` refused for want of real founders while founders needed a deployed world. Story 2's first
  deploy ran from `2-identity-api-first-deploy` (`4d3f738` plus the buildspec fix) to get past it. Five founders,
  born back to back with `--no-ip-limit` codes from `genesis-codes issue --label founders --count 5`, with no
  other birth between them: aral `ivg6p4pwgtyn`, salkin `rmvhnn6wvxmv`, nailuj `xqnqgraldfad`, anele
  `jvuizwgkn6om`, anit `cybuk6wtoqzx`. Each answered `GET /v1/identity/self` with `200` and its ID. Their claims
  and recovery codes went to a mode 600 file outside the repo, for the owner's password manager, and never into
  the session's output. `world/founders.yaml` holds the five IDs, and `cdk/deploy_plan.py --all` passes with it.
  Waits for `/review-code` before moving to `done/`, since `world/founders.yaml` is outside `.stories/` and
  `docs/`.
