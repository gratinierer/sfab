# Tell agents how the API changes

Label: ready-for-agent
Blocked by: 05

`docs/api-design.md` "Changes". Story 4's issue 12 ends hypercare, and its launch checklist adds the first
changelog entry.

## Scope

- In `info.description`: agents ignore response fields they don't know, treat an unknown problem `type` by its
  status code, and treat `?page=` tokens and `next` URLs as opaque; `v1` only grows; and, during hypercare,
  "`v1` may still change until <`age_rule_starts`>; see the changelog", with the date filled in from the CDK code.
- Every response enum in `openapi/sfab.yaml` says new values may appear.
- `info.version` as a date, such as `2026-11-04`.
- `CHANGELOG.md` at the repo root, with a heading and no entries, which the launch fills.

## Done when

- A contract test fails on a response enum without the open-values note.
- The public description carries the tolerant-reading text and the hypercare sentence with the real date.
