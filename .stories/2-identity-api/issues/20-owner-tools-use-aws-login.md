# Let the owner's Rust tools use `aws login` credentials

Label: ready-for-agent
Blocked by:

The owner runs `genesis-codes` and `takedown` from a machine signed in with `aws login`, as `spec.md` "Operations"
and "First deploy" step 9 assume. Neither tool can read those credentials: on 2026-11-20 `genesis-codes issue`
stopped with "the credentials provider was not properly configured: ProfileFile provider could not be built: This
behavior requires following cargo feature(s) enabled: credentials-login". The Python tools had the same gap, which
`boto3[crt]` fixed (`docs/aws-facts.md`, the row on boto3 and `aws login`).

Until this is fixed, the workaround is to export the session's credentials first:
`eval "$(aws configure export-credentials --format env)"`.

## Scope

- Add the `credentials-login` feature to `aws-config` in `lambda/genesis-codes/Cargo.toml` and
  `lambda/takedown/Cargo.toml` (:14-16), and only there: the Lambdas run on their role and don't need it, so their
  dependency stays as it is. Check whether the feature pulls in new crates, and that `scripts/audit` stays clean.
- Add a `docs/aws-facts.md` row: the Rust SDK's `aws-config` reads `aws login` credentials only with its
  `credentials-login` feature, citing the error above, `docs` if the SDK's docs say so, else `tested` once the owner's
  run below works.
- Drop the workaround from wherever the docs give it, if anywhere.

## Done when

- `scripts/check` passes, and the two tools build with the feature, the Lambdas without it.
- The owner runs `genesis-codes status` and `takedown --help`, or a dry run, with `aws login` credentials and no
  exported keys, and both reach AWS. This needs the owner's machine, so if the agent can't check it, it leaves a
  comment here saying so.

## Comments

- 2026-11-20: found during the first deploy. Issue 10's takedown drill and story 4's launch cleanup run `takedown`
  several times, so this matters before then, but the workaround keeps it from blocking either.
