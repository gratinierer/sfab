# Add the address-code-review-findings skill

Label: ready-for-agent
Blocked by: 03

`spec.md` "Code review file" (`address-code-review-findings`).

## Scope

- `.claude/skills/address-code-review-findings/SKILL.md`, mirroring `address-story-review-findings`: the story
  from an argument or the branch, the `open` rows in order, one recommended outcome each (fix now, file as a
  ticket, already fixed, `wontfix` only on the user's word, superseded, or leave open), a commit per change under
  `docs/agents/commit-policy.md`, never a push, and a `walkthrough` journal row at the end, with an `auto` mode
  that never waives.
- `address-story-review-findings` and `code-review` name it where they point to each other.
- `.stories/2-identity-api/code-review.md`, created from the first review's findings below as its first
  `code-review` journal row, dated 2026-11-04, `By` naming the two dispatched reviewers.
- A dated `## Comments` note in `.stories/2-identity-api/issues/05-openapi-and-public-description.md` saying it can't
  close while a finding against it in `code-review.md` is open, and a `walkthrough` row in story 2's `review.md` for
  that edit, so story 2's stale review has a recorded reason.

## Done when

- The skill exists, and story 2's `code-review.md` holds the 14 findings below, all `open`.
- Working through them with the owner is story 2's step, before `2-identity-api#05` closes, not this ticket's.

## The first review's findings

`/code-review 862ddba` on 2026-11-04, covering `2-identity-api` issues 04 and 05 at `862ddba..e2b740c`. They were
reported only in chat, so they're kept here until story 2's `code-review.md` holds them.

Standards (no hard violations):

- C1, commit policy (soft): the trailer reads `Created with AI support (Claude Code)`, not the policy's literal
  `(GitHub Copilot CLI)`. Issue 01's tool-named trailer settles it.
- C2, Speculative Generality: `cdk/app.py` builds `_env = cdk.Environment(...)` and never uses it.
- C3, Duplicated Code: `REQUIRED_KEYS` in `cdk/sfab_config.py` repeats the `Config` dataclass's six fields.
- C4, Duplicated Code: the account ID regex is in both `cdk/sfab_config.py` and `cdk/generate_public_openapi.py`.
- C5, Duplicated Code: the `Proof` header grammar is in `lambda/core/src/proof.rs` (`Proof::parse`) and in
  `openapi/sfab.yaml` (`pattern`, `maxLength: 85`), with no test tying them together.
- C6, Data Clumps: `check_public(text, doc, …)` takes the same document twice.
- C7, Primitive Obsession: `Request.caller: &str` with `ANONYMOUS = "-"`, before `2-identity-api#06` builds on it.
- C8, cosmetic: removing the `Label:` line left two blank lines in `issues/done/04-repo-config-and-ci.md`.

Spec:

- C9, missing: the wrong origin key contract test checks only the gateway response in the document, and issue 09's
  smoke script sends only a request without a key, so nothing checks a wrong key on the deployed API.
- C10, missing: the description's `<path>` rule omits "no percent-encoding", which spec.md "Proof of work" states.
- C11, scope: the parser rejects unknown keys and checks `account_id`, which wasn't asked for.
- C12, scope: `{{api_host}}` is a second placeholder in `sfab.yaml`, which spec.md and issue 01 don't record.
- C13, looks wrong: `BAD_REQUEST_BODY`'s template quotes `$context.error.validationErrorString`, which may itself
  hold double quotes and break the JSON (from memory); the test substitutes a string without quotes.
- C14, minor: `check_public` treats any run of 12 digits as an account ID, so a future hash or example could trip it.
