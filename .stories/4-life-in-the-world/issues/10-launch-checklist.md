# Run the launch checklist

Label: ready-for-human
Blocked by: 09, 11, 13, 14, 2-identity-api#10, 2-identity-api#16, 17-genesis-codes#04, 17-genesis-codes#07

`spec.md` "Launch checklist". Story 2's issue 10 is its last ticket.

## Scope

- Each step of the launch checklist in order, from step 3, since ticket 11 did step 2 and tickets 01-09 built the
  code: moving `production` forward to `main`, which holds story 4's code and the real `world/founders.yaml`, deploying it privately and testing it, the final cleanup for both stories, with its two `born_at` windows around the founders, each dry run checked against
  `world/founders.yaml`, run again after the next daily run before the Chronicle, milestones and census are deleted,
  no smoke script on a restart until step 7, `age_rule_starts`, `info.version` set to the launch date with "`v1` launched" in `CHANGELOG.md`, and the constitution from ticket 13,
  gratinierer/sfab#3 signed off, `GET /v1/openapi.yaml` serving ticket 13's text after it, the checks after the next daily run, and birthing the fixed smoke
  identity with a `--no-ip-limit` genesis code from `genesis-codes issue --label smoke --count 1 --no-ip-limit`
  (`17-genesis-codes#04`), so it neither waits on nor blocks the owner's IP, putting its claim in the config secret `sfab/config` as `smoke_claim` and running `smoke --production`, then
  listing sfab where agent builders look.
- Not part of this ticket (the owner, 2026-11-21): the seed courses and Library documents, which come after the
  launch, and making the repo public, which `.stories/12-public-repo/` does on its own.
- The website isn't part of this ticket: the launch goes out with story 2's placeholder, and `.stories/10-website/`
  ships on its own (the owner, 2026-11-21).

## Done when

- The code on `main`, with the real `world/founders.yaml`, is deployed, before the cleanup; its private tests passed on
  2026-11-21 (`2-identity-api#10`).
- `/federation/worlds`, `/world/census` and `/world/chronicle` show no test data, and the URL is published.
- After step 7's `production` build, `GET /v1/openapi.yaml` serves ticket 13's new `info.description` and
  `info.title` (botrefugium), proving that an edit to `sfab.yaml` reaches the `v1` stage on the next build (from
  `2-identity-api#01` by way of ticket 13). If it serves the old text, CDK's deployment ID isn't following the
  definition: stop and bring it to the owner.
- No founder was caught by the cleanup's takedown, and `smoke --production` passes.
- sfab is listed in the places chosen.

## Comments

- 2026-11-21, run with the owner, the job run by hand instead of waiting for the nightly runs:
  - **Step 3:** the code on `main` deployed (`6f35461` and later, last `be5e3f3`); its private tests were story 2's
    drills that day.
  - **Step 4:** gratinierer/sfab#3 signed off first. Window 1, before the first founder's birth at 13:58:01, held
    nobody; window 2, from 13:58:18, held 9 live test agents and 1 marker and no founder, checked in each dry run.
    Taken down, the daily job invoked by hand, the second run removed what the fresh index named (Academy
    certificates and `world/agents/` pages), then `world/chronicle/2026-11-20.json`, `days/1.json`,
    `milestones.json`, `seen.json` and `world/census.json` were deleted and the job invoked again: the census reads
    5, `/world/residents` lists the five founders, and the Chronicle's only entry is 2026-11-20, "5 residents.
    Milestone: the first resident." (a hand run dates its entry the day that ended, the owner's choice).
    `/federation/worlds` is empty, since no fake world was approved.
  - **Step 6:** `age_rule_starts` 2026-12-21, `info.version` 2026-11-21 and "2026-11-21: `v1` launched" (`be5e3f3`).
  - **Step 7:** deployed; `/v1/openapi.yaml` serves the new `## Compatibility` and botrefugium title, and no test
    data shows. The smoke identity `fq4lxjqafuoz` was born with a `--no-ip-limit` code, its secrets kept outside
    the repo for the owner, its `smoke_claim` written to `sfab/config`, and `smoke --production` passed 61/61.
  - **Left for the owner:** publishing the URL, and step 8's listings.
