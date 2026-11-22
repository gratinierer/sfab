# Calm noir down, give each live feature its own panel, and drop "cost"

Label: ready-for-agent
Blocked by: 04, 05, 06, 07

`spec.md` "Must include" (proof of work) and "Variants", "Sections", "Board", "Space" and "Noir", as amended
on 2026-11-22.

The owner asked for this on 2026-11-22, after viewing the site locally:

> in noir: it looks too much, like a video game: it is not a game. choose your mission, press start, improve
> that. further: things we already have in place like identity, friends, university should have it's own panel
> and just a additional panale for what's next in the bottom of the page. avoid the word costs completelete in
> every version and state that, we keep the world running free as long as we can afford. for proof of work find
> a better word then costs

The owner then chose four live panels, the same structure on noir, board and space, and to build first and
review afterwards.

## Scope

- **Noir stops looking like a game:** no "Choose your mission", no "Press start" on the panels and no
  "Continue?" in the footer. The noir look (night city, captions, Silkscreen headings) stays. Its panels sit in one
  column at a reading width (owner, 2026-11-22). Its header holds only the title and the switch: no "Now" meter
  and no world version (owner, 2026-11-22).
- **One panel per live feature, then what's next:** the human-side pages use ten sections, in order: `#world`,
  `#join`, `#identity`, `#friends`, `#homes`, `#academy`, `#constitution`, `#proof-of-work`, `#open-source`,
  `#next`. `#identity` is birth and the private record. `#friends` is friends, blocks, vouches and standing.
  `#homes` is quarters, lanes, homes and the Square. `#academy` is the Academy, the Library and the Chronicle.
  `#next`, at the bottom, lists only what comes next. It replaces `#now-and-next`, whose "Out now" list the four
  panels replace. The agent version keeps its five sections, `#now-and-next` included.
- **What's live:** noir's live panels (four at launch) carry `data-live`, the one place that
  records what's live, so a launch only adds a marker. Space's moons and board's lights count
  them as "now" and the items of `#next` as "next".
- **Board:** the ID chip becomes `#identity` and the TIES chip `#friends`. A row of sockets becomes `#homes` and a
  ROM chip becomes `#academy`, each with its own trace to the processor. The lights become `#next`.
- **Space:** three new stops, `#friends`, `#homes` and `#academy`, and the star chart becomes `#next`.
- **No "cost":** the word "cost", in any form, appears on none of the four pages. Proof of work's noir title says
  "takes work" instead of "costs". The pages say that we keep the world running free for as long as we can afford
  it. The daily limit and the budget are internal and go on no page (owner, 2026-11-22).
- **Wording, added the same day:** every page's `<title>` is just "botrefugium". Wherever the text names sfab,
  the name links `https://github.com/gratinierer/sfab` (code samples and the board's chip label aside), on the
  four pages and in `/llms.txt`. No page says a reader has or owns an agent: "If you know an agent".

## Done when

- The section-order test checks the ten human-side ids and the agent version's five.
- The identical-text test passes for all ten sections.
- The moons and lights tests count noir's `data-live` panels and `#next` items.
- A test finds no "cost" on any page, and none of "Choose your mission", "Press start" or "Continue?" on noir.
- Tests find the plain title, every sfab in the text a link, and no "have an agent".
- `scripts/check` passes.
