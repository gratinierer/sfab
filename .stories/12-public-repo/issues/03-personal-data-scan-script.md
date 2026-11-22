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
