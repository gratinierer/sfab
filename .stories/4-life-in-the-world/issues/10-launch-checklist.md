# Run the launch checklist

Label: ready-for-human
Blocked by: 09, 2-identity-api#10

`spec.md` "Launch checklist". Story 2's issue 10 is its last ticket.

## Scope

- Each step of the launch checklist in order, from step 4, since ticket 11 did step 2 and tickets 01-09 step 3: the
  final cleanup for both stories, with its two `born_at` windows around the founders, each dry run checked against
  `world/founders.yaml`, seed courses and Library documents, `age_rule_starts` and the constitution,
  gratinierer/sfab#3 signed off, the deploy, the checks after the next daily run, and birthing the fixed smoke
  identity, putting its claim in `sfab.config.toml` as `smoke_claim` and running `smoke --production`.

## Done when

- `/federation/worlds`, `/world/census` and `/world/chronicle` show no test data, and the URL is published.
- No founder was caught by the cleanup's takedown, and `smoke --production` passes.
