# Vendor the skills into the repo

Label: ready-for-agent
Blocked by: 01

`spec.md` "Skills in the repo".

## Scope

- Copy the 13 existing skills into `.claude/skills/`, with any files beside their `SKILL.md`. The three from
  agentic_containers come from the live repo, gratinierer/agentic_containers at `main` (eaf96f3 or later), not the
  read-only mount at `/workspaces/agentic_container`, which can be stale.
- `.claude/skills/VENDORED.md`, naming each skill's source and version as `spec.md` "Frozen copies" says, and
  `.claude/skills/LICENSE-mattpocock`.
- Adapt the copies: `/setup-matt-pocock-skills` becomes a pointer to `docs/agents/issue-tracker.md`, example keys
  use this repo's story names, and "the repo's commit policy" points to `docs/agents/commit-policy.md`.
- In `.stories/12-public-repo/`, add the vendored skills and `.claude/skills/LICENSE-mattpocock` to the README's
  "Third-party" line in both `spec.md` "New files" and `issues/01-readme-license-security.md`, and make its
  `LICENSE` sentence ("plain MIT for everything but that font", `spec.md`:27) except the vendored skills too, which
  keep their own notice. Story 12's issue 01 also gets `Blocked by: 14-agent-skills#02`, because it names the licence
  file this issue creates, and its `LICENSE` bullet ("MIT, with the owner as copyright holder") gets the same
  exception. Story 12 has never been reviewed, so `/review-story 12-public-repo` runs for the first time before its
  issue 01 starts.

## Done when

- Each vendored skill's references resolve inside the repo, and none mentions `/setup-matt-pocock-skills`
  or another project's story keys or issue tracker.
- `VENDORED.md` lists all 13, with a source and version each.
