# 2-identity-api: code review

Code reviews of this story's issues. The journal lists every pass: `code-review` runs of `/code-review`, and
`walkthrough` passes of `/address-code-review-findings`. The findings table lists what each run found and what became
of it. Rows are never deleted or renumbered: a fixed finding flips to `resolved`.

Statuses: `open` / `resolved` / `wontfix` / `superseded`.
Axes: `standards` / `spec`.
Conventions: `docs/agents/issue-tracker.md`, section "Code review file".

## Review journal

| Date | Kind | Reviewed | By | Findings | Notes |
|---|---|---|---|---|---|
| 2026-11-04 | code-review | 04, 05 @ 862ddba..e2b740c | claude-opus-5-5 sub-agents for standards and spec (dispatched) | 14 new: 8 standards, 6 spec | first run, reported only in chat and kept in `14-agent-skills#04` until this file existed; its time wasn't recorded |
| 2026-11-04T18:06:14+00:00 | code-review | 04 @ 7124f67..d8ac678 | claude-sonnet-5-5 (standards and spec, dispatched) | 3 new: 0 standards, 3 spec | test run of the repo's `code-review` for `14-agent-skills#03`, on one commit; findings C15-C17 |

## Findings

| # | Status | Axis | Affects | Description | Comments |
|---|---|---|---|---|---|
| C1 | resolved | standards | 04, 05, commit messages | Possible policy mismatch (soft): the commits' trailer reads `Created with AI support (Claude Code)`, not the global policy's literal `(GitHub Copilot CLI)`; `docs/agents/commit-policy.md` now names the tool. | 2026-11-04: found. 2026-11-04 (walkthrough): on the owner's decision, the trailer is now a plain `Created with AI support`, naming no tool, in `docs/agents/commit-policy.md` and AGENTS.md, and in the global policy through gratinierer/agentic_containers' change; earlier commits keep their trailers as history. |
| C2 | resolved | standards | 04, cdk/app.py | Possible Speculative Generality: `_env = cdk.Environment(account=config.account_id, region=REGION)` is built and never used. | 2026-11-04: found. 2026-11-04 (walkthrough): on the owner's go-ahead, `cdk/app.py` drops `_env` and `REGION` and keeps the config load, with a comment saying why; `cdk/tests/test_app.py` passes (3). |
| C3 | open | standards | 04, cdk/sfab_config.py | Possible Duplicated Code: `REQUIRED_KEYS` repeats the six fields of the `Config` dataclass, which `dataclasses.fields(Config)` could derive. | 2026-11-04: found. |
| C4 | open | standards | 04, 05, cdk/sfab_config.py, cdk/generate_public_openapi.py | Possible Duplicated Code: the 12-digit account ID regex appears in both files, a full match in one and a search in the other. | 2026-11-04: found. |
| C5 | open | standards | 05, lambda/core/src/proof.rs, openapi/sfab.yaml | Possible Duplicated Code: the `Proof` header grammar is in `Proof::parse` and in the `pattern` and `maxLength: 85` of `sfab.yaml`, with no test tying them together. | 2026-11-04: found. |
| C6 | open | standards | 05, cdk/generate_public_openapi.py | Possible Data Clumps: `check_public(text, doc, …)` takes the same document twice, as text and parsed. | 2026-11-04: found. |
| C7 | open | standards | 05, lambda/core/src/proof.rs | Possible Primitive Obsession: `Request.caller: &str` with `ANONYMOUS = "-"` stands for "agent ID or anonymous" before `2-identity-api#06` builds on it. | 2026-11-04: found. |
| C8 | open | standards | 04, .stories/2-identity-api/issues/done/04-repo-config-and-ci.md | Removing the `Label:` line left two blank lines in a row (cosmetic). | 2026-11-04: found. |
| C9 | open | spec | 05, cdk/tests/test_openapi.py | The wrong origin key contract test checks only the `INVALID_API_KEY` gateway response in the document, and issue 09's smoke script sends only a request without a key, so nothing checks a wrong key on the deployed API. | 2026-11-04: found. |
| C10 | open | spec | 05, openapi/sfab.yaml | The description's `<path>` rule leaves out "no percent-encoding", which spec.md "Proof of work" states. | 2026-11-04: found. |
| C11 | open | spec | 04, cdk/sfab_config.py | Scope: the parser rejects unknown keys and checks that `account_id` is 12 digits, which spec.md "Local config" didn't ask for, so every later key needs a parser change. | 2026-11-04: found. |
| C12 | open | spec | 05, openapi/sfab.yaml | Scope: `{{api_host}}` is a second placeholder for the `MISSING_AUTHENTICATION_TOKEN` detail, which spec.md "OpenAPI is the single source" and issue 01 don't record. | 2026-11-04: found. |
| C13 | open | spec | 05, openapi/sfab.yaml | `BAD_REQUEST_BODY`'s template quotes `$context.error.validationErrorString`, which may itself hold double quotes and break the JSON (from memory), while the test substitutes a string without quotes. | 2026-11-04: found. |
| C14 | open | spec | 05, cdk/generate_public_openapi.py | `check_public` treats any run of 12 digits as an account ID, so a future hash or example could trip it by chance. | 2026-11-04: found. |
| C15 | open | spec | 04, .github/workflows/ci.yml:46-49 | `pipx install "uv==…"` and `uv pip install` pin uv, cargo-lambda and ziglang but leave their own dependencies unpinned, while spec.md "Delivery" pins cargo-audit with `--locked`. | 2026-11-04: found. |
| C16 | open | spec | 04, .github/workflows/ci.yml:50 | Prepending the tools venv's `bin` to `PATH` makes its `python3` shadow the runner's for every later step, including `scripts/check`, a side effect the comment doesn't mention. | 2026-11-04: found. |
| C17 | open | spec | 04, .github/workflows/ci.yml:22 | `ZIGLANG_VERSION` is commented "the devcontainer's zig", but nothing checks the two match, while spec.md "Delivery" asks for "the same toolchain as the devcontainer". | 2026-11-04: found. |
