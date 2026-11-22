# Make the repo public

GitHub issue: [gratinierer/sfab#12](https://github.com/gratinierer/sfab/issues/12)
Authored: 2026-11-04 by claude-opus-5-5

## Goal

Publish this repo, so the API description's and `/llms.txt`'s GitHub links resolve, without publishing a secret by
accident. Since 2026-11-21 it isn't part of `.stories/done/4-life-in-the-world/`'s launch: the owner switches it on its own,
when this story is done, and the API's GitHub links answer `404` until then.

The test from `docs/adr/0001-initial-architecture.md` applies: could this cost about $100 unnoticed? A leaked
AWS secret could, which is why the scan comes first.

## What gets published

- **Everything, as it is:** the code, `.stories/` with every `review.md`, `docs/` and the full history. The
  design documents describe the defences in detail, such as how few IPs can trip the daily limit, and that's
  accepted: the design is safe because of its limits, not because it's secret, and a stop is a valid outcome.
- **The author email:** every commit carries the owner's iCloud relay address, which is fine to publish.
- **Never:** the owner's private values, `origin_key`, `ip_hash_key` and, after launch, `smoke_claim`. They live in
  the Secrets Manager secret `sfab/config`, not in the repo (`.stories/18-deploy-from-codebuild/`), and no template
  holds them. `smoke-genesis-codes.txt` stays gitignored.
- **Never, either:** the owner's personal data, such as their name, postal address, phone number and private email
  addresses, nor the AWS account ID, hosted zone ID or support case IDs. Stories and docs describe such values
  ("the alert address", "the account") instead of copying them.

## New files

- **`README.md`:** what sfab is, a link to the website and `/llms.txt`, how to build and test with the
  devcontainer, and where the stories and ADRs are, with a "Third-party" line for the Silkscreen font, which is under the SIL OFL 1.1 rather than MIT, and for the
  agent skills vendored from Matt Pocock in `.claude/skills/`, which keep his MIT notice in
  `.claude/skills/LICENSE-mattpocock`. It's the first thing people and agents read here.
- **`LICENSE`:** plain MIT for everything but that font and the vendored skills, which keep their own notices, the simplest fit for a small, non-critical project. Anyone may run a closed fork.
- **`SECURITY.md`:** report security problems through GitHub's private vulnerability reporting, and report
  content through the abuse contact in the API description's `info.contact`. There's no response deadline,
  which fits the owner's manual reactions.

## Before and at the switch

1. **Scan the full history** with `gitleaks`, not just the latest commit, because a value committed once and
   deleted later stays readable in history. Any real finding stops the switch and goes back to the owner, because
   removing it means rewriting history.
   **Scan it for personal data too,** because `gitleaks` only finds secrets. The history is far too large for an
   agent to read, so a script does the walking, and the agent only judges what it reports (see "The personal data
   scan" below). A real finding stops the switch the same way.
2. **Turn on** private vulnerability reporting, secret scanning and push protection. Push protection blocks a
   later push that contains a known secret format, so no new CI tool is needed.
3. **Switch the visibility** to public.
4. **Protect `production`** with a branch ruleset that restricts deletions and blocks force pushes
   (`.stories/done/18-deploy-from-codebuild/`). GitHub Free offers rulesets only on public repos, so the branch
   has been unprotected since the first deploy (`2-identity-api#03`).

## The personal data scan

`scripts/pii-scan` is deterministic and makes no LLM calls. It walks the full history, and an agent then checks
only its report, so the tokens spent grow with the findings, not with the history.

- **What it reads:** every blob reachable from any ref, each scanned once however many commits contain it, every
  commit message, every commit's author and committer name and email, every annotated tag's message and tagger,
  and every branch and tag name, local and remote. Binary files are skipped. Commit metadata goes public with the
  history, and a personal name can sit there even when no file holds it.
- **What it looks for:**
  - the owner's literal values, one per line, from a list outside the repo, so the list itself can't leak. Its
    path is an argument, and there's no default inside the repo. It holds what no generic pattern finds
    reliably: the owner's name and private usernames (not the GitHub handle `gratinierer`, which the repo's URLs
    and every commit publish anyway), the parts of their postal address (street, city, postcode), their
    private email addresses and phone numbers, and the support case IDs, so "Never, either" is covered.
  - generic patterns: email addresses, 12-digit numbers (account IDs), phone numbers and Route 53 hosted zone IDs.
- **What it ignores:** values in a committed allowlist of public ones, such as `.invalid` and `example.*`
  addresses, the test fixtures' made-up account IDs (`000000000000`, `123456789012`, `210987654321`,
  `777777777777`) and hosted zone ID (`Z00000000000000000000`), the AI co-author address in commit trailers, and
  the owner's relay author address, which is fine to publish (above). The allowlist is
  `pii-allowlist.txt` at the repo root, beside `audit-ignore.toml`, always read: one regular expression per line,
  matched case-insensitively against a whole matched value, each followed by a `# reason`. It never applies to
  the owner's literals.
- **What it prints:** one line per distinct finding, sorted and deduplicated: the first commit that introduced it,
  the path, or "commit message", "commit author", "commit committer", "tag" or "ref name", the line number, the pattern's name, and the match masked except for its first and
  last characters. A literal is named only by its line number in the list. Nothing is printed in full, so the
  report holds no personal data itself.
- **Exit status:** 0 when nothing is left, 1 when there are findings, 2 on an error.
- **The agent's part:** read the report raw, not through any output filter (in a session that condenses command
  output, `rtk bypass scripts/pii-scan …`), and first check that it finds a known planted value. Mark each finding
  real or harmless. Harmless ones go into the allowlist, so a rerun ends clean.

## Clarifications

### Discovery grilling, 2026-11-04

Asked while grilling how agents find the API, after the owner said `/llms.txt` should link the repo "(it will become public)".

| Question | Answer |
|---|---|
| Make the repo public as a launch step, after a full-history secret scan? | "create a separate story for that" |
| Is the commit author email fine to publish? | "it's relay" |
| Which license? | MIT |
| `SECURITY.md` with private vulnerability reporting, separate from the abuse contact? | Yes |
| Publish `.stories/` and the reviews as they are? | Yes |
| This story holds the scan, README, LICENSE, SECURITY.md and the switch, and the launch checklist requires it? | Yes (the launch requirement superseded 2026-11-21: the owner decoupled this story) |
| GitHub push protection afterwards, rather than `gitleaks` in CI? | Yes |
| A normal story with two tickets, files first, then the owner's steps? | Yes (three since the personal data scan's script became issue 03) |
