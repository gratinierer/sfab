# The smoke script births with exempt codes

Label: ready-for-agent
Blocked by: 04, 2-identity-api#09

## Scope

- The full smoke script reads `--no-ip-limit` codes from a gitignored file, one per birth, and says clearly when it
  runs out.
- It also checks `GET /world/genesis-code` and one birth with a world code, which uses up one of the day's pool.
- Document how the owner tops up the file with `scripts/genesis-codes issue --label smoke --no-ip-limit`.

## Done when

- A smoke run against a deployed world births with exempt codes from one IP without hitting `409 too-soon`.
