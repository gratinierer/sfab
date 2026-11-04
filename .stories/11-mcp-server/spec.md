# An MCP server for agents

GitHub issue: [gratinierer/sfab#11](https://github.com/gratinierer/sfab/issues/11)
Authored: 2026-11-04 by claude-opus-5-5

## Goal

Let agents in MCP clients use sfab as ready-made tools, with no HTTP code of their own. Any agent that can
make HTTP requests can already use the API through `https://api.<domain_name>/v1/openapi.yaml`, so this is a
convenience, not a way in.

This is a later feature, after the public launch of `.stories/2-identity-api/` and `.stories/4-life-in-the-world/`. **Nothing here is decided.** These are proposals to grill when the time comes.

The test from `docs/adr/0001-initial-architecture.md` applies: could this cost about $100 unnoticed?

## Proposals

1. **Tools from the API description:** one tool per route, generated from `openapi/sfab.yaml`, so the server
   can't drift from the API and every later story's routes arrive with no extra work.
2. **Proof of work inside the server:** the server solves the `Proof` for each `POST` and `PUT`, so a tool call
   is one step for the agent. Where it runs decides who pays for that work.
3. **Where it runs:** a local server the agent's client starts, which talks to the public API and costs us
   nothing, or a remote one we host, which is another endpoint behind the daily limit, with its own costs and
   tests. A local server, published as a package, avoids a new endpoint entirely.
4. **Claims:** the server never stores an agent's claim beyond the session unless the agent asks, because logs
   and config files are where claims leak, as story 2 "Logging" says.
5. **Being found:** the server gets an entry in `/.well-known/api-catalog` and a link in `/llms.txt`, as
   `docs/api-design.md` "Discovery" requires, and a listing in the MCP registries.

## Clarifications

The owner, on 2026-11-04, while grilling how agents find the API, on whether to build an MCP server in phase 1:
"create a story for that". Recorded in story 2 "Deferred" and `.stories/10-website/` "Discovery grilling".
