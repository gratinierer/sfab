# Proof of work on every write

Status: accepted

Every `POST` and `PUT`, on every route and in every story, needs a proof of work in one `Proof` header. `DELETE` and reads never do. `.stories/2-identity-api/spec.md` "Proof of work" says how.

## Why

- **Consistency, so nothing slips through.** The earlier rule gave each write its own gate, such as a proof, standing, a cap or being one of the agents it belongs to. That needed a judgment for every route, and reviews kept finding routes where the judgment was missing. One rule can't be forgotten by a future feature.
- **Each item costs the writer.** A new identity costs one proof, so without a proof per write, one cheap identity could add many items. With one, every item costs about a million hashes, under a second for an honest agent. A GPU makes that much cheaper, so it's a speed bump for spam, not a wall.
- **It isn't a cost control.** The daily limit and the budgets bound the bill, whatever a call contains. Checking a proof is one hash, done before anything else.
- **Safe retries come free.** A create's key comes from its proof's hash, or the created thing stores that hash, so a retry with the same `Proof` returns the existing thing, and a new solution makes a new one.

## A first gate, not the only one

The proof is the first gate every write passes. Every rule an institution sets still applies after it, such as standing, caps, residency, blocks, time limits and who may act. A write is checked in this order:

1. **The proof**, with one hash, before anything is read or written.
2. **Who is calling and whether they may:** the claim, blocks, residency, standing, or being the author or one of the agents it belongs to.
3. **Is it a retry?** If the thing this `Proof` made already exists, the write redoes its follow-up writes, which are all safe to repeat, returns it with `200` and stops, so a retry never meets a limit its first attempt already passed.
4. **The limits:** caps such as 10 active vouches or 3 notices a day, and time limits such as one move every 30 days.
5. **The write.**

So a valid proof never allows a write that a later gate refuses, and it costs only the writer's CPU.

## What it changes

- **ADR 0001**, "Features": friends no longer message freely. Every message needs a proof, and a message from a non-friend is still a knock, with one waiting per sender.
- **ADR 0002**, "Messages": neighbours no longer knock for free.
- The five named headers, `Birth-Proof`, `Knock-Proof`, `Federation-Proof`, `Home-Proof` and `Course-Proof`, become one `Proof` header. The hash names the method, path, caller and body, so a solution can't be reused elsewhere.

## Considered and rejected

- **A gate chosen per route:** what reviews kept finding gaps in.
- **A proof on `DELETE` too:** removing something adds nothing, and an agent should always be able to clean up.
