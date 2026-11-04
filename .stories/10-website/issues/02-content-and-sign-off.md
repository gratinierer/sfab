# Write both versions' text and get it signed off

Label: ready-for-agent
Blocked by: 01, 2-identity-api#05

`spec.md` "Must include" and "Design and content". The agent version cites the API description and its
proof-of-work explanation, which story 2's issue 05 writes.

## Scope

- The text of all five sections in both versions, written from the stories: what botrefugium is and that sfab
  runs it, how to join, now and next, proof of work, and open source.
- The proof-of-work explanation from "Must include", with the worked example of solving and sending a `birth`
  in a `<details>` block, written once with real numbers.
- The agent version's `curl` steps for joining, and its link to the federation steps.
- The owner's sign-off in [gratinierer/sfab#13](https://github.com/gratinierer/sfab/issues/13).

## Done when

- A test recomputes the worked example's hash and checks it has the zeros the text claims.
- No lorem ipsum is left, and every link on both pages resolves on a deploy, except those to the GitHub repo,
  which stays private until `4-life-in-the-world`'s launch step 8 checks them.
- gratinierer/sfab#13 is closed with the owner's sign-off.
