# Set up the domain in iCloud+

Blocked by: 01

`spec.md` "Design". The owner's steps in iCloud's settings and the config, which no agent can do.

## Steps

1. In iCloud settings, Custom Email Domain, add `<domain_name>` as a domain you own, and note the records it
   shows.
2. Compare them with `spec.md` "Design"'s table. If any differs, fix the table and ticket 01's code before going
   on.
3. Check the hosted zone holds none of the records 01 creates: no `MX` or `TXT` on the apex, and no
   `sig1._domainkey` or `_dmarc` record (`aws route53 list-resource-record-sets --hosted-zone-id <hosted_zone_id>`).
   CloudFormation fails `SfabBase`'s update on a record that already exists, so if one is there, stop and decide
   whether to delete it or keep it, rather than deploying. On 2026-11-21 there were none.
   Once ticket 01 is closed, move `production` forward to `main`, as `docs/restart-after-a-stop.md` step 1
   describes a deploy. Then put the `apple-domain=` value, without its prefix, into `sfab/config` as
   `icloud_domain_verification`, and start the `sfab-deploy` build (ADR 0009). Not in the other order: a build of a
   `production` without 01's code refuses the key as unknown and fails.
4. Let iCloud verify the domain.
5. Create `watchdog@`, `dmarc@`, `abuse@` and `info@`, with no catch-all. Note the real address limit here
   under `## Comments`, and in `spec.md` if it's lower than four. Under a lower limit, drop `info@` first, then
   `abuse@`; keep `watchdog@`, the published address, and `dmarc@`, where the DMARC record's `rua` points.
6. Add a mail rule moving mail to `dmarc@` into its own folder.

## Done when

- A mail from an outside account to `watchdog@` arrives, and one to an address that doesn't exist bounces.
- A reply sent as `watchdog@` arrives at an outside account with DKIM and DMARC `pass` in its headers. SPF is
  noted but not required, because iCloud may send with its own envelope sender, and DMARC passes on DKIM alone.
- `docs/aws-facts.md` or this ticket's comments record which record values iCloud really asked for.

## Comments

- 2026-11-21: the first try created the four addresses before the domain was verified, so iCloud treated them as
  existing addresses and sent confirmation mails, which couldn't arrive without an `MX`. The owner restarted
  without existing addresses; create them only after the domain is verified (step 5).
- 2026-11-21: step 1 and 2 done. iCloud showed both `MX` records at priority 10, the `apple-domain=` TXT, the SPF
  TXT `v=spf1 include:icloud.com ~all` and the CNAME `sig1._domainkey` →
  `sig1.dkim.<domain_name>.at.icloudmailadmin.com.`, matching `spec.md` "Design". The token goes into
  `sfab/config` only, never into the repo. It's 16 letters and digits, so it fits ticket 01's
  `[A-Za-z0-9]{8,64}`.
- 2026-11-21: step 3 done. The zone held none of the four record sets. `icloud_domain_verification` is in
  `sfab/config`; PR #21 moved `production` to `0bfe291` (main at `547fbcb`), and build
  `sfab-deploy:24856b58-11d5-4b17-9c56-9c696f748737` succeeded. Route 53 holds the `MX`, the apex `TXT` (verification and
  SPF), the `sig1._domainkey` CNAME and the `_dmarc` TXT with the values in `spec.md` "Design", and public DNS
  (8.8.8.8) answers the `MX` and `_dmarc`.
- 2026-11-21: steps 4 to 6 done, by the owner's report "everything works": iCloud verified the domain, the four
  addresses `watchdog@`, `dmarc@`, `abuse@` and `info@` exist with no catch-all, and the `dmarc@` rule is in place.
  Mail from outside to the addresses arrived, a mail to an unknown address bounced, and a reply as `watchdog@` showed
  DKIM and DMARC `pass`. iCloud allowed all four, so `spec.md` keeps them; the exact limit it shows wasn't noted.
