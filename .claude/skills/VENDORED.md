# Vendored skills

The skills this repo's workflow uses, copied in once on 2026-11-04 and now this repo's own, so a session doesn't
depend on what the devcontainer links in. Upstream changes are pulled in by hand when wanted. See
`.stories/14-agent-skills/` and `docs/adr/0005-skills-live-in-the-repo.md`.

Matt Pocock's skills are under the MIT licence in `LICENSE-mattpocock`, "Copyright (c) 2026 Matt Pocock". The
skills from agentic_containers are the owner's.

| Skill | Source | Commit | `skills-lock.json` hash |
|---|---|---|---|
| `address-story-review-findings` | gratinierer/agentic_containers, `.devcontainer/general_devcontainer/resources/ai/skills/address-story-review-findings/` | `eaf96f3` | |
| `github-pr-review` | gratinierer/agentic_containers, `.devcontainer/general_devcontainer/resources/ai/skills/github-pr-review/` | `eaf96f3` | |
| `review-story` | gratinierer/agentic_containers, `.devcontainer/general_devcontainer/resources/ai/skills/review-story/` | `eaf96f3` | |
| `code-review` | mattpocock/skills, `skills/engineering/code-review/` | `4ae2317` | `c2745e2be960` |
| `codebase-design` | mattpocock/skills, `skills/engineering/codebase-design/` | `4ae2317` | `aa11a96c0845` |
| `domain-modeling` | mattpocock/skills, `skills/engineering/domain-modeling/` | `4ae2317` | `b3eb40152b81` |
| `grill-with-docs` | mattpocock/skills, `skills/engineering/grill-with-docs/` | `4ae2317` | `35e62aa423aa` |
| `grilling` | mattpocock/skills, `skills/productivity/grilling/` | `4ae2317` | `e40cc1d348f6` |
| `improve-codebase-architecture` | mattpocock/skills, `skills/engineering/improve-codebase-architecture/` | `4ae2317` | `f768cb44209e` |
| `prototype` | mattpocock/skills, `skills/engineering/prototype/` | `4ae2317` | `d3fc74689bd9` |
| `research` | mattpocock/skills, `skills/engineering/research/` | `4ae2317` | `5d85b64eccee` |
| `to-tickets` | mattpocock/skills, `skills/engineering/to-tickets/` | `4ae2317` | `5f871bdc46a9` |
| `wayfinder` | mattpocock/skills, `skills/engineering/wayfinder/` | `4ae2317` | `22338d9356ef` |

Matt Pocock's ten were identical to mattpocock/skills at `4ae2317` (2026-10-31) when copied; the hash is the first 12
characters of each one's `computedHash` in the devcontainer's `skills-lock.json`.

## Changes from upstream

- `code-review`, `to-tickets` and `wayfinder`: "run `/setup-matt-pocock-skills`" points to `docs/agents/issue-tracker.md`
  instead, and `to-tickets` to `docs/agents/triage-labels.md` too.
- `address-story-review-findings`: "the repo's commit policy" names `docs/agents/commit-policy.md`.

`address-code-review-findings` isn't vendored: it was written in this repo, after `address-story-review-findings`.

Later changes made in this repo, such as `code-review` writing `code-review.md`, are recorded in git history.
