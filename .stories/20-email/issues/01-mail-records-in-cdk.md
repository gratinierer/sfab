# Mail DNS records in CDK

Label: ready-for-agent

`spec.md` "Design". Creates the iCloud mail records and DMARC from CDK when the owner's config has the iCloud
token, and nothing when it doesn't.

## Scope

- `cdk/sfab_config.py`: a new optional `Config` field `icloud_domain_verification: str | None = None`. When the
  key is present, `parse` checks it like a required key: stripped, refused when empty or holding `CHANGE_ME`, and
  refused unless it matches `[A-Za-z0-9]{8,64}`, with a message saying to leave off the `apple-domain=` prefix
  when the value starts with it.
- `cdk/stacks.py`'s `BaseStack`: when the field is set, the four record sets from `spec.md` "Design" in the
  imported hosted zone, built from `domain_name`: the `MX` with both iCloud servers, one `TXT` record set on the
  apex holding both the `apple-domain=` value and the SPF value, the `sig1._domainkey` CNAME, and the `_dmarc`
  `TXT`. When it's missing, none of them.
- CDK tests for both cases, the fixture staying without the key, and one test that sets it.
- Config tests for each refusal: empty, `CHANGE_ME`, the `apple-domain=` prefix, and a value outside the pattern.
- `.stories/2-identity-api/spec.md` "Delivery"'s list of `sfab/config` keys gains the optional key, with a
  pointer to this story.

## Done when

- `scripts/check` passes.
- With the key set, a synth of `SfabBase` has exactly those four record sets with those values, and without it
  none.
- A config with an empty, placeholder or prefixed `icloud_domain_verification` fails to load with a message
  naming the key.
- A code review (`/review-code`) has no open finding.
