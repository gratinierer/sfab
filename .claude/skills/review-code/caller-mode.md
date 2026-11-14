# Caller mode of `/review-code auto`

A calling skill, such as `implement-story`, runs `/review-code <fixed-point> auto` through the Skill tool and hands
over five inputs in its prompt. Each replaces a step and its up-front question:

1. **Story and issues**: replace step 2 and the story question. They are taken as given.
2. **Reviewers' model**: replaces step 4 and the model question. The caller has already picked one other than the
   author's, the orchestrator's and the implementers'; `By` in step 7 names it.
3. **Baseline check result** (`green`, `missing`, `skeleton, nothing checked`, or a red one the user chose to run
   past): replaces running `scripts/check` in step 3b and the baseline question. Step 7 records it in the `check` row
   as `Notes`, and the loop's own check (loop step 7) still runs between rounds.
4. **Commit filter**, the subject suffix `(#<n>)`: step 1's range and commit list become
   `git log <fixed-point>..HEAD --oneline --grep='(#<n>)$'`, and the diff command is
   `git log <fixed-point>..HEAD --reverse --patch --cc --grep='(#<n>)$'`, so only those commits' changes are shown,
   in every round. Other stories' commits are neither diffed nor listed to a sub-agent. The journal row still names
   the issues of input 1 and the full range `<fixed-point>..HEAD`.
5. **Spec reviewer's extra line**, with the full messages of the filtered commits
   (`git log --grep='(#<n>)$' --format=%B <fixed-point>..HEAD`): both go into the step 5 Spec prompt after the brief.

With all five given, nothing is asked before round 1: the caller has also settled the branch ruling and the
uncommitted story documents. Step 3a still runs and its stop still ends the run before round 1.
