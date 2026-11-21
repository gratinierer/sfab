# Publish `operator@` as the contact

Label: ready-for-agent
Blocked by: 02

`spec.md` "Design", "The new contact". Replaces the iCloud address everywhere it's published, once `operator@`
receives mail.

## Scope

- `cdk/generate_website.py`: a new placeholder filled from `abuse_contact`, as `{{api_host}}` is filled from the
  API's host, with tests.
- `website/index.html`: shows the address through that placeholder, as a `mailto:` link.
- The owner sets `abuse_contact` to `operator@<domain_name>` in `sfab/config` and deploys, which changes
  `info.contact` in the public description and the API Lambda's `ABUSE_CONTACT`.
- No `CHANGELOG.md` line (`spec.md` "Clarifications", Q8).

## Done when

- `scripts/check` passes, and no committed file contains the owner's address.
- After the deploy, `https://api.<domain_name>/v1/openapi.yaml`'s `info.contact.email` and the website both show
  `operator@<domain_name>`, and the iCloud address appears in neither.
- A code review (`/review-code`) has no open finding.
