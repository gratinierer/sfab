# Tell agents how the API changes

Label: ready-for-agent
Blocked by: 05, 4-life-in-the-world#01

`docs/api-design.md` "Changes" and `docs/adr/0011-v1-changes-are-announced-not-forbidden.md`: `v1` may change,
and a change worth knowing is announced in the changelog. Story 4's launch checklist adds the first changelog
entry.

Built on `main`, where `4-life-in-the-world#01` already is since story 4's code reached `main` on 2026-11-17
(`4-life-in-the-world/spec.md` "Launch checklist", step 2).

## Scope

Issue 05 already shipped part of this: the `## Compatibility` section of `info.description` (ignore unknown
fields, treat an unknown problem `type` by its status code and every response enum as open, treat `?page=` tokens
and `next` URLs as opaque), "New values may appear" on `Problem.type`, and `info.version` as a date, which
`test_info_version_is_a_date` checks. What's left:

- Rewrite `## Compatibility` for ADR 0011: drop "`v1` only grows", say that `v1` may change, that a name never
  comes back with a different meaning, and that when `info.version` changes the changelog lists anything worth
  knowing, linking `CHANGELOG.md` in the public repo (`gratinierer/sfab`, public from story 4's launch step 8).
- `CHANGELOG.md` at the repo root, with a heading and no entries, which the launch fills.

## Done when

- The public description carries the tolerant-reading text, says `v1` may change, and links the changelog; no text
  says `v1` only grows.

## Comments

- 2026-11-21: the owner dropped "`v1` only grows" and hypercare (ADR 0011), so the hypercare sentence and its date
  from `age_rule_starts` left this ticket.
- 2026-11-21: the open-values contract test left this ticket, on the owner's choice (R209): response value
  lists are prose, not `enum:`, so a test had nothing to find; `## Compatibility` says every response enum is open,
  and reviews check the per-field note, as `docs/api-design.md` "Changes" says.
