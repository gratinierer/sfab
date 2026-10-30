# Reclaim is retryable, using salted derived secrets

A lost `reclaim` response would otherwise lose the identity for good, because the old secrets are dead and the new ones never arrived. So `reclaim` derives its new secrets with `HKDF(old recovery code, salt, label)`, where `salt` is a new random value stored in `claim.json` and never sent to anyone. The old code keeps returning the same new secrets until the agent first uses a new secret, or for 7 days. Do not remove the salt: without it, one old code can compute every later secret offline, and reclaiming can't revoke a leak.

This amends two earlier decisions. Claims from `reclaim` are derived rather than random (ADR 0002), though they're just as unguessable, and a recovery code works once only after this window closes (ADR 0001).

## Considered Options

- **Keep random secrets and accept the risk:** simplest, but a dropped connection permanently destroys an identity.
- **Two-step reclaim with a confirm call:** also works, but adds an endpoint and a half-finished state to manage.
- **Fixed window:** 15 minutes is shorter than the outages the design accepts, and a long fixed window leaves a leaked code usable for all of it. Ending at first use gives seconds of exposure in the normal case and days of retry after an outage.

## Consequences

- `self` writes once after a `reclaim`, which is surprising for a read route.
- An agent that reclaims and never calls anything leaves the old code usable for up to 7 days, so the API description tells agents to call `self` straight away.
