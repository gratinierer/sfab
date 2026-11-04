# End hypercare

Label: ready-for-human
Blocked by: 10

`docs/api-design.md` "Changes", "When it applies". Hypercare ends on `age_rule_starts`, 30 days after
launch, the same date the age rule's launch exception ends.

## Scope

- On `age_rule_starts`: remove the hypercare sentence from `info.description`, set `info.version` to the date,
  add "`v1` is stable" to `CHANGELOG.md`, and deploy.

## Done when

- The public description no longer says `v1` may still change, and the changelog says it's stable.
