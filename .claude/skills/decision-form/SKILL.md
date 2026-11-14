---
name: decision-form
description: Load when an interactive review dialogue needs the user's decision on one item, to ask it as a numbered list of answer options. Used by `address-code-review-findings`, `address-story-review-findings` and `github-pr-review`; not meant to be invoked on its own.
---

# Decision form

A calling skill presents one item at a time and needs the user's decision. It passes its own outcomes and their order;
this skill defines how they are asked.

## The Options block

End the item's message with an **Options** block, after the item's own blocks (header, finding, why it matters,
recommendation):

1. A numbered list of the outcomes that apply to this item, in the caller's fixed order, numbered from 1 each time.
   List an outcome that has a precondition only when the precondition holds.
2. Mark the recommended outcome `(recommended)` and give its concrete change in the same line. The other lines name the
   outcome in a few words.
3. Make every outcome its own number. Never fold two outcomes into one line, such as "Waive or leave open".
4. Close with exactly this line: `Reply with the number of your choice.`

Never close with a question that has "or" in it ("Fix, waive or leave open?"), and never list answers in prose.

## Reading the reply

- A number picks that option.
- Anything else is the user's own words. Say in one line how you read it ("Reading this as: fix R3, but keep the
  heading") and act on that, unless the user corrects you.
- If the reply is a number the list doesn't have, ask the same question again.

## Waive needs a reason

A waiver records the user's reason, not yours. If the user picks the waive option without a reason, ask one open
question, "What is your reason?", with no options, and record the answer as given.

## No widget

The decision is a plain chat message. Don't use `ask_user` or any form widget for it, and in an interactive pass, even in
autopilot mode, ask and wait.
