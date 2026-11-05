# Skip an address pass that can only repeat itself

Label: ready-for-agent

`spec.md` "The auto loop", "Early stop".

## Why

The first real run, `/code-review 334b3f3 auto 3` on issue 01, spent its second address pass writing six
"still needs the owner" comments, one commit each, on findings that nothing had touched since pass 1. The
orchestrator skipped the third pass by hand. "No progress" is only detected after a pass has run, so a pass with
nothing it may act on still costs one commit per open finding.

## Scope

- Both review skills' "The loop": before an address pass, stop with reason **no progress** when the last review
  added no new finding and every open row was already left open for the owner by the previous address pass.
- Both address skills: a finding left open in an earlier pass, whose file and row nothing has changed since, gets
  no new comment and no commit when re-walked.
- `spec.md` "Early stop" and "Decided later" record the rule, and the statements it narrows say so: spec "Commits
  by default", both address skills' "Committing" ("every treated finding gets a commit") and
  `docs/agents/issue-tracker.md` ("It commits each treated finding").

## Done when

- A loop whose open rows all wait on the owner stops without running another address pass, and the final report
  names the stop reason.
- Re-walking an unchanged left-open finding produces no commit.
