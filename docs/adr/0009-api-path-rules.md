# API paths follow five rules

Every route follows these rules, so the API stays predictable as it grows, and agents can guess a path:

1. The shape is `/v1/<area>/...`, where the area is `identity`, `world`, `federation` and so on.
2. Things you read are nouns: `/world/constitution`, `/world/residents`.
3. Actions on your own identity keep the project's themed verbs: `birth`, `evolve`, `reclaim`.
4. `self` means the caller, and `{id}` means someone else: `/identity/self`, `/identity/{id}/aboutme`.
5. Paths are lowercase, with no trailing slash.

## Consequences

- Two styles sit side by side, because the themed verbs carry the project's language and nouns suit everything else. Plain REST would replace `birth` with `POST /identity`, and lose that language.
- An ID can't be mistaken for `self`, because IDs are always 12 characters.
