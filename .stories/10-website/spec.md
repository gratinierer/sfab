# The website: design and content

GitHub issue: [gratinierer/sfab#10](https://github.com/gratinierer/sfab/issues/10)
Authored: 2026-11-02 by claude-opus-5-5

## Goal

Replace the lorem ipsum placeholder that `.stories/2-identity-api/` ships with a real website, one for humans at `/` and one for agents at `/agents/`, in plain HTML5 and CSS. **Nothing here is decided yet.** The hosting is already designed in story 2: a private S3 bucket, its own CloudFront distribution and plan, and `BucketDeployment`.

The test from `docs/adr/0001-initial-architecture.md` applies: could this cost about $100 unnoticed?

## Must include

**A good explanation of proof of work,** for both humans and agents. The owner found it hard to understand at first, so others will too. What helped:
- The zeros mean nothing by themselves. They're a rule, like "roll five sixes", and a result with many zeros is evidence of many tries. There's no shortcut, and checking takes one calculation.
- It's one-way: the server sends no challenge, and uses only its own clock. The time is part of the puzzle, so a prepared call only works within about 6 minutes.
- A worked example, with real numbers, of solving and sending a `birth`.
- It isn't a cost control. It makes fake identities cost a little, and the daily limit protects the bill.

The API description needs the same explanation, which story 2's issue 05 owns, because agents need it at launch whether or not this story is done.

## To settle when grilled

- Whether this has to be done before the public launch, or whether the launch can ship with a simple real page.
- What each version says, and how it looks.

## Clarifications

The owner, on 2026-11-02, during the walkthrough of story 2: "create a story for the website and there remark, that we need a good explanation for that proof of work and also on the api side". Earlier, in story 2's extension grilling: "design and content will be decided in a later story".
