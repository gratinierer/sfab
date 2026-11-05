# Review skills commit by default, and can loop unattended

Status: accepted

`/review-story`, `/code-review`, `/address-story-review-findings` and `/address-code-review-findings` commit their
own work without asking: a review commits its record file once it is written, and an address pass commits once per
treated finding. `nocommit` opts out. `/review-story <story> auto [X]` and `/code-review <fixed-point> auto [X]` run
review → address → review unattended, up to X address passes (default 3), and ask the owner only about design
decisions and important findings. See `.stories/15-review-auto-loop/`.

## Why

- **The owner is the bottleneck, not the reviewer.** Story reviews here run three or more passes, and each pass
  needed a hand-started walkthrough, a yes per commit and a hand-started re-review. Most of those answers were
  "yes".
- **Asking about commits had stopped meaning anything.** Every walkthrough ended with "commit?" and the answer was
  always the same. A default plus `nocommit` keeps the choice without the ritual.
- **A loop needs commits.** Re-reviews cover the fix range `<previous HEAD>..HEAD`, and progress is measured
  against it, so an unattended loop can't work on an uncommitted tree.
- **Per-finding commits keep history bisectable**, which is why the walkthroughs go one finding at a time.

## Consequences

- `docs/agents/commit-policy.md` carries an exception to "never commit unless asked": running one of these skills
  is the ask, for that skill's own commits only. Pushing still always needs an explicit request.
- Commits stage only the paths a finding or review touched. A code file with unrelated uncommitted edits keeps its
  finding open; uncommitted story documents are offered for one commit before a loop starts.
- Each loop round's review runs on a fresh sub-agent on a model other than the author's and the orchestrator's, so
  the orchestrator never certifies its own fixes.
- History gets many small `Address R<n>: …` commits instead of one batch per walkthrough.

## Considered and rejected

- **Commits only in `auto` mode:** the owner wanted the same behaviour interactively, and two commit rules for one
  skill invite mistakes.
- **A loop that ends on an address pass:** its fixes would go unreviewed, `review.md` would be stale for the
  implementation gate, and the code-review closing gate wouldn't be met.
- **`auto nocommit`:** without commits the fix range is empty and progress can't be measured, so it is refused.
