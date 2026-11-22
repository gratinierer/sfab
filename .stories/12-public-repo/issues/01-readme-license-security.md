# Add the README, LICENSE and SECURITY.md

Label: ready-for-agent
Blocked by: 14-agent-skills#02

`spec.md` "New files".

## Scope

- `README.md`: what sfab is, the website and `/llms.txt`, building and testing with the devcontainer, and
  where the stories and ADRs are, and a "Third-party" line: the Silkscreen font by The Silkscreen Project Authors, under the SIL OFL 1.1,
  in `website/fonts/OFL.txt`, from `.stories/10-website/`, and the agent skills vendored from Matt Pocock in
  `.claude/skills/`, under his MIT notice in `.claude/skills/LICENSE-mattpocock`, from `.stories/done/14-agent-skills/`.
- `LICENSE`: MIT, with the owner as copyright holder, except the vendored skills, which keep their own notices.
- `SECURITY.md`: private vulnerability reporting for security problems, the API description's abuse contact
  for content, and no response deadline.

## Done when

- The three files are committed, and the README's links resolve within the repo.
