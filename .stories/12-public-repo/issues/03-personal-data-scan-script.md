# Write the personal data scan script

Label: ready-for-agent

`spec.md` "The personal data scan". Issue 02 runs it before the switch.

## Scope

- `scripts/pii-scan <list-of-literals>`: walks every blob reachable from any ref, once each, every commit
  message, author and committer, every tag message and every ref name, without an LLM, and prints the masked, deduplicated report with the exit status the spec describes.
- `pii-allowlist.txt`, the committed allowlist of public values, in the spec's format, with the entries it names.
- Python with the standard library only, like `scripts/audit`.
- Tests in `cdk/tests/test_pii_scan.py`, which `scripts/check`'s cdk pytest already runs, as it does
  `test_build_scripts.py` for the other scripts. They run the script on a throwaway repo built in the test: a planted literal (also in a commit author's name and a branch name), an email address, an account ID, a phone number
  and a hosted zone ID are each found once, even when the line appears in many commits; an allowlisted value
  and a binary file aren't reported, but a literal that an allowlist entry also matches, and another address at the
  domain of an allowlisted address, are; the report never contains a full match; and a clean repo exits 0.

## Done when

- The tests pass in `scripts/check`.
- A run over this repo's history finishes, and its report is short enough to check by hand. The size and runtime
  are noted under `## Comments` below, without any finding's value.

## Comments

- 2026-11-22: written. The first run over this repo's 1700-odd commits takes about 15 s and, with the owner's list of 6
  literals, prints 2 lines (both real, recorded in issue 02's Comments); a planted public literal was found in 83
  places first. Before the allowlist got the harmless values it reported, the run printed 6 pattern findings:
  the spec's own `<id>+…` noreply wording, the Copilot co-author, story 20's planned `operator@` address and an old
  sequential zone ID placeholder, now all in `pii-allowlist.txt`. The phone pattern found nothing in the history.
- 2026-11-22: `scripts/check` passed at `204af55` with the 7 new tests (594 passed). The issue waits only for its
  code review before it moves to `done/`.
