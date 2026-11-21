# Email on our own domain

GitHub issue: [gratinierer/sfab#20](https://github.com/gratinierer/sfab/issues/20)
Authored: 2026-11-21 by claude-opus-5-5

## Goal

Receive and send mail as `watchdog@<domain_name>`, so the project stops publishing a private iCloud address and can
read its DMARC reports, without adding anything to the AWS bill or to what the project stores.

The test from `docs/adr/0001-initial-architecture.md` applies: could this cost about $100 unnoticed? With iCloud+
as the mail host, inbound mail never reaches AWS, so it can't.

## What's there now

- **The published address is `abuse_contact` in `sfab/config`,** today an iCloud address.
  `cdk/generate_public_openapi.py` writes it into the API description's `info.contact` as "Abuse and takedown
  reports", and `cdk/stacks.py` passes it to the API Lambda as `ABUSE_CONTACT`, which publishes it as `contact` at
  `GET /v1/federation/world` (`lambda/api/src/main.rs`), the contact `world/constitution.md` item 7 sends agents to. Story 2's spec
  (`.stories/done/2-identity-api/spec.md` "World", "Federation" and "Delivery") and `.stories/12-public-repo/` rely
  on it as the abuse contact. `alert_email` is separate and isn't published.
- **The domain has no mail records:** no `MX`, SPF, DKIM or DMARC. The zone is imported in `cdk/stacks.py`'s
  `BaseStack` with `HostedZone.from_hosted_zone_attributes`, so CDK owns only the records it creates.
- **SES is unused.** On 2026-11-21 the owner verified the domain in SES with Easy DKIM, got production access and
  sent a test mail, which proved sending works (`docs/aws-facts.md` "SES"). The same day the identities, the three
  DKIM CNAMEs and a hand-made `_dmarc` record were deleted again, because the project sends no mail of its own:
  alerts go through SNS and budget mails through AWS Budgets. Production access stays on the account.

## Design

- **iCloud+ is the mail host.** The domain's `MX` points to iCloud. Mail to one of the addresses lands in the
  owner's iCloud inbox, and the owner reads it and replies as that address in Apple Mail, which signs with
  iCloud's DKIM key. Apple filters spam. AWS only holds the DNS records.
- **Addresses:** `watchdog@`, `dmarc@`, `abuse@` and `info@`, created in iCloud's settings, with no catch-all, so
  mail to any other address bounces. Apple's main page allows 10 addresses per custom domain, and older pages say
  3; ticket 02 checks the real limit. `watchdog@` is the only address the project publishes. `abuse@` exists
  because mail and network operators send abuse reports there without looking anything up (RFC 2142), and
  `info@` because people guess it; both land in the same iCloud inbox as `watchdog@`.
- **DMARC:** `v=DMARC1; p=none; rua=mailto:dmarc@<domain_name>`. An iCloud mail rule moves the reports into their
  own folder. Reading them, by hand or with a report service, and tightening `p=none` are later choices.
- **DNS records in CDK, in `BaseStack`,** which a rollback doesn't touch, so mail survives a failed deploy:

  | Record | Value |
  |---|---|
  | `MX <domain_name>` | `10 mx01.mail.icloud.com.`, `10 mx02.mail.icloud.com.` |
  | `TXT <domain_name>` | `apple-domain=<icloud_domain_verification>` and `v=spf1 include:icloud.com ~all`, as one record set |
  | `CNAME sig1._domainkey.<domain_name>` | `sig1.dkim.<domain_name>.at.icloudmailadmin.com.` |
  | `TXT _dmarc.<domain_name>` | `v=DMARC1; p=none; rua=mailto:dmarc@<domain_name>` |

  iCloud's setup showed exactly these values on 2026-11-21, apart from the DMARC record, which iCloud doesn't ask for.
- **One new optional config key, `icloud_domain_verification`,** in `sfab/config` and `cdk/sfab_config.py`'s
  `Config`, defaulting to `None`. When it's set, `BaseStack` creates all the records above; when it's missing, it
  creates none, so another owner or a world without iCloud still deploys. The value isn't secret, since DNS
  publishes it. It's in the config because it differs per owner, like `domain_name`.
- **The new contact:** `abuse_contact` becomes `watchdog@<domain_name>`, which changes `info.contact` and the
  Lambda's `ABUSE_CONTACT` on the next deploy. ADR 0011 allows that in `v1`. It gets no `CHANGELOG.md` line,
  because the address goes on the website instead. That's story 10's work: `.stories/10-website/spec.md`
  "Contact" and its ticket 02 add a `{{contact}}` placeholder filled from `abuse_contact` and show it on both
  versions, so this story changes no website file.
  `watchdog` names only the address: the role stays "the operator", as `world/constitution.md` items 7 and 8 and
  `GLOSSARY.md` "Takedown" call it, and the constitution keeps pointing to "the contact at
  `/v1/federation/world`" rather than to an address, so its text doesn't change.
- **Nothing is stored by the project.** Mail lives in the owner's own mailbox, so there's no retention rule and
  none of the GDPR concerns that story 2's "Deferred" (`.stories/done/2-identity-api/spec.md`) raised about the project storing contact addresses.
- **No ADR.** iCloud over SES receiving is a trade-off, but changing a few DNS records reverses it, so it's
  recorded here only.

## Out of scope

- Sending mail from the project's own code. That needs SES again, a story of its own.
- Moving `alert_email`. Alerts must arrive even when something on the domain is broken.
- A custom `MAIL FROM` domain, a forwarding Lambda, an S3 mailbox and a kill-switch step for mail. All of them
  belonged to SES receiving.

## Clarifications

Grilled with the owner on 2026-11-21. The owner's starting ideas were a DMARC report inbox, a general contact
inbox such as `operator@botrefugium.com` to replace the published iCloud address, and spam protection.

| # | Question | Recommended | Owner's answer |
|---|---|---|---|
| Q1 | Does spam protection mean filtering inbound mail, hiding the address from scrapers, or protecting the sending reputation? | Filtering inbound mail only | Filtering only |
| Q2 | Where does mail for `operator@` end up: S3 and a script, a forwarding Lambda, an SNS notice, or a third-party forwarder? | A forwarding Lambda with a copy in S3 | "But if I want to answer via `operator@`, what then?", which became Q9 |
| Q3 | Which addresses exist? | `operator@`, `dmarc@`, `abuse@` and `postmaster@` | Yes, refined in Q13 |
| Q4 | Does `alert_email` stay at its external address? | Yes | Alerts untouched |
| Q5 | Does the setup move into CDK, accepting a recreated SES identity? | Yes | Yes |
| Q6 | How is inbound mail's cost limited: the existing stop also deactivating receiving, a separate alarm, or budgets only? | The stop also deactivates receiving | The stop; superseded by Q9 |
| Q7 | A custom `MAIL FROM` domain, so SPF aligns too? | Not now | Yes; superseded by Q12 |
| Q8 | Does the new contact get a `CHANGELOG.md` line? | Yes | No: the address goes on the website later |
| Q9 | How does the owner read and reply as `operator@`: SES SMTP credentials, a reply script, iCloud+ custom domain, or another host? | iCloud+, if the owner has it | iCloud+. The owner asked whether SES is still needed: no, the project sends no mail of its own |
| Q10 | Is the website change a ticket in story 10, blocked by this story? | Yes | No blocking: it's a later ticket in this story |
| Q11 | `MAIL FROM` as `mail.<domain_name>`, falling back to SES's default? | Yes | Yes, all in CDK, with the hand-made setup reverted by hand; superseded by Q12 |
| Q12 | Drop SES from the account and the story? | Yes | Yes, now: done on 2026-11-21, the identities and their DKIM CNAMEs |
| Q13 | Which addresses within iCloud's limit, with no catch-all? | `operator@`, `dmarc@`, `abuse@`, `postmaster@` | No `postmaster@`, but `info@` |
| Q14 | DMARC reports to `dmarc@` in iCloud, sorted into a folder by a mail rule? | Yes | Yes |
| Q15 | DNS records in `BaseStack`, with the iCloud token as a config key? | Yes | The owner asked whether the stacks hold personal data: no, the token is public in DNS and goes in the config because it differs per owner |
| Q15b | Is `icloud_domain_verification` optional, creating no mail records when missing? | Yes | Yes |
| Q16 | Delete the hand-made `_dmarc` record by hand before the first deploy? | Yes | Do it now: deleted on 2026-11-21 |
| Q17 | One ticket switches `abuse_contact` and puts the address on `index.html`? | Yes | Yes |
| Q18 | No retention rule, since mail lives in the owner's mailbox? | Yes | Yes |
| Q19 | Spec only, no ADR? | Yes | Yes |
| Q20 | Three tickets: records in CDK, iCloud setup, switch the contact? | Yes | Yes |
| Q21 | A more bot-like name for the published address than `operator@`? Offered `daemon@`, `init@`, `watchdog@`, `ping@` | `daemon@` | `watchdog@`, the timer that steps in when a system hangs, which fits abuse and takedown reports. It replaces `operator@` everywhere; the rows above keep the name they were asked with |
| Q22 | `world/constitution.md` and `GLOSSARY.md` call the role "the operator": keep `watchdog@` as the address with the role unchanged, go back to `operator@`, or rename the role too? | Keep `watchdog@`, role unchanged | Keep `watchdog@`; the role stays "the operator" |
| Q23 | Where does ticket 03 put the link while story 10 hasn't rewritten the pages? (from R6) | A plain footer | Nowhere: 03 changes no website code; story 10's spec and ticket 02 take the placeholder and the link |
