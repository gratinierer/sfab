# Add the README, LICENSE and SECURITY.md

Blocked by: 14-agent-skills#02

`spec.md` "New files".

## Scope

- `README.md`: what sfab is, the website and `/llms.txt`, building and testing with the devcontainer, and
  where the stories and ADRs are, and a "Third-party" line: the Silkscreen font by The Silkscreen Project Authors, under the SIL OFL 1.1,
  in `website/fonts/OFL.txt`, from `.stories/10-website/`, and the agent skills vendored from Matt Pocock in
  `.claude/skills/`, under his MIT notice in `.claude/skills/LICENSE-mattpocock`, from `.stories/done/14-agent-skills/`.
- `LICENSE`: MIT, with the handle as copyright holder (`Copyright (c) 2026 gratinierer`, never the owner's name), except the vendored skills, which keep their own notices.
- `SECURITY.md`: private vulnerability reporting for security problems, the API description's abuse contact
  for content, and no response deadline.

## Done when

- The three files are committed, and the README's links resolve within the repo.

## Comments

- 2026-11-22: written. The devcontainer lives in a private repo, so the README names the tools and points at
  `tool-versions.env` and `.github/workflows/ci.yml` instead of linking it. Story 10 is cited at
  `.stories/done/10-website/`, where it now lives. Every relative link in `README.md` and `SECURITY.md` resolves.
- 2026-11-22: the owner asked for the botrefugium logo with a touch of the noir website, and a chapter on how we
  work. `.github/readme-banner.svg` puts `website/shelter-wordmark.svg`'s pixels, unchanged, on the site's night
  colour with stars and a skyline, since GitHub strips a README's CSS; "How we work" sums up stories, reviews, code
  reviews and closing, and links the tracker and commit policy.
- 2026-11-22: closed by the owner without a `/review-code`, which the repo asks for before an issue that
  changed code closes (`docs/agents/issue-tracker.md` "Closing an issue or story").
