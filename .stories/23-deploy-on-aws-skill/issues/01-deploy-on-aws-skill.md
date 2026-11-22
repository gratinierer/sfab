# Write the /deploy-on-aws skill

Label: ready-for-agent

`spec.md` "Decisions".

## Scope

- `.claude/skills/deploy-on-aws/SKILL.md`: the seven steps, the `auto`, `skipcheck` and `smoke` arguments (`smoke`
  runs the smoke test without offering it, in either mode) and optional stack names, the asking rules, and the masking of 12-digit numbers, ARNs and emails, as `spec.md`
  "Decisions" lists them.
- `.gitignore` gains `.env`.
- `docs/adr/0012-deploy-on-aws-may-push.md`: running `/deploy-on-aws` is the explicit instruction to push `main`
  and merge into `production`, amending ADR 0010; ADR 0010's Status line gains "Amended by ADR-0012".
- `docs/agents/commit-policy.md`'s "Exception, skills that commit" paragraph names `/deploy-on-aws` as the one
  skill that may push and merge, citing ADR 0012, instead of saying all such skills "never push".
- `.claude/skills/VENDORED.md` lists `deploy-on-aws` as the owner's.
- `AGENTS.md` "Skills" needs no change: it already points to `.claude/skills/`.

## Done when

- The skill file holds every decision in `spec.md`, and nothing account-specific: no account ID, ARN, email or
  API ID.
- `git check-ignore .env` succeeds.
- A dry read of the skill against today's deploy (push, #22, build `sfab-deploy:34291012-…`) walks every step
  without a gap.
- `scripts/check` passes.
