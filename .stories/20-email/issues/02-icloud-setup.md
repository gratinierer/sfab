# Set up the domain in iCloud+

Label: ready-for-human
Blocked by: 01

`spec.md` "Design". The owner's steps in iCloud's settings and the config, which no agent can do.

## Steps

1. In iCloud settings, Custom Email Domain, add `<domain_name>` as a domain you own, and note the records it
   shows.
2. Compare them with `spec.md` "Design"'s table. If any differs, fix the table and ticket 01's code before going
   on.
3. Once ticket 01 is on the `production` branch, put the `apple-domain=` value, without its prefix, into
   `sfab/config` as `icloud_domain_verification`, and start the `sfab-deploy` build (ADR 0009). Not before: until
   01's code is deployed, the loader refuses the key as unknown and the build fails.
4. Let iCloud verify the domain.
5. Create `watchdog@`, `dmarc@`, `abuse@` and `info@`, with no catch-all. Note the real address limit here
   under `## Comments`, and in `spec.md` if it's lower than four.
6. Add a mail rule moving mail to `dmarc@` into its own folder.

## Done when

- A mail from an outside account to `watchdog@` arrives, and one to an address that doesn't exist bounces.
- A reply sent as `watchdog@` arrives at an outside account with DKIM, SPF and DMARC all `pass` in its headers.
- `docs/aws-facts.md` or this ticket's comments record which record values iCloud really asked for.

## Comments

- 2026-11-21: the first try created the four addresses before the domain was verified, so iCloud treated them as
  existing addresses and sent confirmation mails, which couldn't arrive without an `MX`. The owner restarted
  without existing addresses; create them only after the domain is verified (step 5).
- 2026-11-21: step 1 and 2 done. iCloud showed both `MX` records at priority 10, the `apple-domain=` TXT, the SPF
  TXT `v=spf1 include:icloud.com ~all` and the CNAME `sig1._domainkey` →
  `sig1.dkim.<domain_name>.at.icloudmailadmin.com.`, matching `spec.md` "Design". The token goes into
  `sfab/config` only, never into the repo.
