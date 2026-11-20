# Tell agents how the API changes

Label: ready-for-agent
Blocked by: 05, 4-life-in-the-world#01

`docs/api-design.md` "Changes". Story 4's issue 12 ends hypercare, and its launch checklist adds the first
changelog entry.

Built on `main`, where `4-life-in-the-world#01` already is since story 4's code reached `main` on 2026-11-17
(`4-life-in-the-world/spec.md` "Launch checklist", step 2).

## Scope

Issue 05 already shipped part of this: the `## Compatibility` section of `info.description` (`v1` only grows,
ignore unknown fields, treat an unknown problem `type` by its status code and every response enum as open, treat
`?page=` tokens and `next` URLs as opaque), "New values may appear" on `Problem.type`, and `info.version` as a
date, which `test_info_version_is_a_date` checks. What's left:

- The hypercare sentence in `info.description`, "`v1` may still change until <`age_rule_starts`>; see the
  changelog", with the date filled in from the CDK code.
- The open-values note on every response enum that later issues add, enforced by the contract test below.
- `CHANGELOG.md` at the repo root, with a heading and no entries, which the launch fills.

## Done when

- A contract test fails on a response enum without the open-values note.
- The public description carries the tolerant-reading text and the hypercare sentence, its date generated from
  `age_rule_starts`, so the deploy at launch step 7 carries the real date.
