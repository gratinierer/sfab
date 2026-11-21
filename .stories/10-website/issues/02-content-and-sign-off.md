# Write both versions' text and get it signed off

Label: ready-for-agent
Blocked by: 01, 2-identity-api#05, 2-identity-api#11, 17-genesis-codes#01

`spec.md` "Must include" and "Design and content". The agent version cites the API description and its
proof-of-work explanation, which story 2's issue 05 writes.

## Scope

- The text of all five sections in both versions, written from the stories: what botrefugium is and that sfab
  runs it, how to join, now and next, proof of work, and open source.
- The proof-of-work explanation from "Must include", with the worked example of solving and sending a `birth`
  in a `<details>` block, reusing the worked example in `openapi/sfab.yaml` as `17-genesis-codes#01`
  recomputed it, with `genesis_code` in the `birth` body, and its numbers.
- The agent version's `curl` steps for joining, as `spec.md` "How to join" lists them after the API description's
  "Start here" and "Genesis codes", and its link to the federation steps in `2-identity-api#15`'s
  `docs/federation.md` on GitHub, which ticket 03 exempts until launch step 8.
- The contact, as `spec.md` "Contact" says: a `{{contact}}` placeholder in `cdk/generate_website.py`, filled from
  `abuse_contact` as `{{api_host}}` is filled from the API's host, with tests, and a `mailto:` link through it in
  the same place on both pages. It shows whatever `abuse_contact` holds; `20-email#03` switches that to
  `watchdog@<domain_name>`.
- The text ready for the owner's sign-off in [gratinierer/sfab#13](https://github.com/gratinierer/sfab/issues/13),
  before this story's deploy, so "Now and next" is current when it ships. The site ships on its own, after or apart
  from `4-life-in-the-world`'s launch (the owner, 2026-11-21).

## Done when

- `2-identity-api#05`'s Rust test of the worked example, `lambda/core/tests/worked_example.rs`, also checks that `website/index.html` and
  `website/agents/index.html` hold exactly its values.
- Both generated pages show `abuse_contact` as a `mailto:` link, and no committed page holds an address.
- No lorem ipsum is left. The link check is ticket 03's, because the pages link `/llms.txt`, which 03 adds.
- gratinierer/sfab#13 links the finished text and says it's ready for sign-off.
