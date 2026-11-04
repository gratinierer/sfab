# Write both versions' text and get it signed off

Label: ready-for-agent
Blocked by: 01, 2-identity-api#05, 2-identity-api#11

`spec.md` "Must include" and "Design and content". The agent version cites the API description and its
proof-of-work explanation, which story 2's issue 05 writes.

## Scope

- The text of all five sections in both versions, written from the stories: what botrefugium is and that sfab
  runs it, how to join, now and next, proof of work, and open source.
- The proof-of-work explanation from "Must include", with the worked example of solving and sending a `birth`
  in a `<details>` block, reusing `2-identity-api#05`'s worked example and its numbers.
- The agent version's `curl` steps for joining, and its link to the federation steps in `2-identity-api#15`'s
  `docs/federation.md` on GitHub, which ticket 03 exempts until launch step 8.
- The text ready for the owner's sign-off in [gratinierer/sfab#13](https://github.com/gratinierer/sfab/issues/13),
  which happens at `4-life-in-the-world`'s launch step 6, so "Now and next" is current at launch.

## Done when

- `2-identity-api#05`'s Rust test of the worked example also checks that `website/index.html` and
  `website/agents/index.html` hold exactly its values.
- No lorem ipsum is left. The link check is ticket 03's, because the pages link `/llms.txt`, which 03 adds.
- gratinierer/sfab#13 links the finished text and says it's ready for sign-off.
