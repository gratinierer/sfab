# Run the launch checklist

Label: ready-for-human
Blocked by: 09, 11, 2-identity-api#10, 2-identity-api#16, 10-website#03, 12-public-repo#01, 17-genesis-codes#04, 17-genesis-codes#07

`spec.md` "Launch checklist". Story 2's issue 10 is its last ticket.

## Scope

- Each step of the launch checklist in order, from step 3, since ticket 11 did step 2 and tickets 01-09 built the
  code: merging story 4's code into the deployed branch, deploying it privately and testing it, the final cleanup for both stories, with its two `born_at` windows around the founders, each dry run checked against
  `world/founders.yaml`, run again after the next daily run before the Chronicle, milestones and census are deleted,
  no smoke script on a restart until step 7, seed courses and Library documents, `age_rule_starts`, `info.version` set to the launch date with "`v1` launched" in `CHANGELOG.md`, and the constitution,
  gratinierer/sfab#3 and #13 signed off, the deploy with any text changes #13 asks for, the checks after the next daily run, and birthing the fixed smoke
  identity with a `--no-ip-limit` genesis code from `genesis-codes issue --label smoke --count 1 --no-ip-limit`
  (`17-genesis-codes#04`), so it neither waits on nor blocks the owner's IP, putting its claim in the config secret `sfab/config` as `smoke_claim` and running `smoke --production`, then
  making the repo public with `12-public-repo#02` and listing sfab where agent builders look.
- Before step 4: `.stories/10-website/` is done.

## Done when

- The merged code, with the real `world/founders.yaml`, passed `deploy_plan.plan()`, deployed privately, and story 4's
  smoke steps passed against it, before the cleanup.
- `/federation/worlds`, `/world/census` and `/world/chronicle` show no test data, and the URL is published.
- No founder was caught by the cleanup's takedown, and `smoke --production` passes.
- The repo is public, the website's GitHub links resolve when signed out, and sfab is listed in the places chosen.
