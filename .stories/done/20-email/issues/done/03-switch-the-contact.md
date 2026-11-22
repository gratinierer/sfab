# Publish `watchdog@` as the contact

Blocked by: 02

`spec.md` "Design", "The new contact". Replaces the iCloud address everywhere it's published, once `watchdog@`
receives mail.

## Scope

- The owner sets `abuse_contact` to `watchdog@<domain_name>` in `sfab/config` and starts the `sfab-deploy`
  build (ADR 0009), which changes
  `info.contact` in the public description and the API Lambda's `ABUSE_CONTACT`, and with it `contact` at
  `GET /v1/federation/world`.
- No website file: story 10's ticket 02 puts `abuse_contact` on the site (`.stories/10-website/spec.md`
  "Contact"), so the site follows this switch on the next deploy.
- No `CHANGELOG.md` line (`spec.md` "Clarifications", Q8).

## Done when

- After the deploy, `https://api.<domain_name>/v1/openapi.yaml`'s `info.contact.email` and
  `https://api.<domain_name>/v1/federation/world`'s `contact` show `watchdog@<domain_name>`, and the iCloud
  address appears in neither, nor on the website once `10-website#02` has landed.

## Comments

- 2026-11-21: done. The owner set `abuse_contact` to `watchdog@botrefugium.com` in `sfab/config`, and build
  `sfab-deploy:c2c38075-2ad5-400c-9e54-ca64bf8e722f` succeeded. `/v1/openapi.yaml`'s `info.contact.email` and
  `/v1/federation/world`'s `contact` show `watchdog@botrefugium.com`, and neither holds the iCloud address. The
  website follows with `10-website#02`, which is story 10's to check.
