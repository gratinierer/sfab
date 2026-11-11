# `/code-review` takes its answers from a calling skill

Label: ready-for-agent

`spec.md` "`/code-review` gains a caller mode": `/implement-story` (issue 01) settles everything before it
implements, and its end-of-run `code-review ... auto 3` must then run without asking and review only the story's
own commits on a shared branch.

## Scope

- `.claude/skills/code-review/SKILL.md` documents a caller mode: a skill that follows it inline can supply, along
  with the fixed point and `auto [X]`,
  - the story and the issues to name (step 2 takes them as named, as in its case 1);
  - the reviewers' model (step 4 uses it, still checking it is neither the code's author nor the orchestrator);
  - the baseline `scripts/check` result, and whether the owner waived check gating;
  - the owner's branch ruling;
  - the answer on uncommitted story documents: that the caller committed them, or the owner's choice;
  - a commit filter: only commits whose subject (not any message line, as `--grep` would match) ends `(#<n>)`,
    optionally with a base per issue: a commit whose body names an issue ("Issue NN of story N") is kept only after
    that issue's base, and one naming no issue is kept from the fixed point;
  - an extra line for the Spec sub-agent's brief, which then also gets the full commit messages (`git log
    <fixed-point>..HEAD`, not `--oneline`, limited to the filtered commits). `/implement-story` uses it to have each commit body's "Seams:" line
    checked against its issue's Scope and "Done when".
- With a commit filter, both sub-agents review only the filtered commits' changes, merge commits included, in a
  form that shows a merge's conflict resolutions (for instance `git log -p --cc` over the filtered commits, or an
  equivalent the skill states; plain `git log -p` prints no diff for a merge), other stories' commits in the
  range don't trigger step 2's "spans two stories, ask", step 4 finds the author from the filtered commits only (so
  another story's author can't reject the supplied model), and the journal's `Notes` say the range was filtered.
- With all answers supplied, "Up front, then unattended" asks nothing; any answer missing is asked as today.
- Without caller mode, behaviour is unchanged.
- `VENDORED.md` "Changes from upstream" for `code-review` mentions the caller mode.

## Done when

- `code-review`'s skill file states the caller mode, the commit filter, the extra Spec brief and the journal note.
- A run given every answer has no step that asks the user before its final report.
