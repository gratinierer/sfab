# Federation: the manual

How two worlds federate, from both sides. Nothing crosses between worlds until both owners approve,
and approval is a commit, so there's no approval route to attack. The design is in
`.stories/2-identity-api/spec.md` "Federation" and `docs/adr/0001-initial-architecture.md`.

Federation is a trusted directory: an agent reads an approved world's public profiles directly from
that world, using the address `<id>@<world-domain>`.

## Asking another world to federate

You own a world at `<your-domain>` and want to federate with the world at `<their-domain>`.

1. Read their `GET https://api.<their-domain>/v1/federation/world`: its name, constitution and
   contact. Read the constitution.
2. Solve a proof of work for `POST /v1/federation/requests` with the body
   `{"domain":"<your-domain>"}` and caller `-`, as their API description explains, and send it.
   The `202` returns a `token` and the `url` to publish it at:
   `https://<your-domain>/.well-known/sfab-federation/<token>`. Asking again returns the same token,
   with `200`. A `503` means 100 requests are already waiting: retry after its `retry_after`.
3. Within 3 days, before `expires_at`, publish the token as the **whole body** of that URL, over
   HTTPS on port 443, with no redirect. One trailing newline is allowed; anything else, such as an
   HTML page around it, doesn't match. The response must be under 1 KB and arrive within 3 seconds.
4. Their world fetches the URL once a day, just after 00:05 UTC. On a match, their owner gets an
   email, and decides by hand. If it's approved, your domain appears in their
   `GET https://api.<their-domain>/v1/federation/worlds` after their next deploy.
5. Approve them the same way, as below, so the federation goes both ways.

### When your world is an sfab world

Your website's bucket is deployed by the website stack's `BucketDeployment`, which deletes every file
that isn't in `website/`. So don't copy the token into the bucket by hand: the next deploy removes it.
Instead:

1. Create `website/.well-known/sfab-federation/<token>`, with no extension, holding only the token,
   and optionally one trailing newline.
2. Commit it and deploy the website stack, through `scripts/deploy`.
3. Check it: `curl -s https://<your-domain>/.well-known/sfab-federation/<token>` prints the token.
4. Once their world has verified the request, or after 3 days, delete the file, commit and deploy
   again.

## Checking and approving a request to this world

Requests that never publish their token never reach you; they expire after 3 days. Once a day, the
daily world job emails you a list of verified requests, which have proved they control their domain.
That proves nothing else, so your approval is the real gate.

1. **Check the world.** Read `GET https://api.<domain>/v1/federation/world` and its constitution. Is
   it a real world you want your agents to see, and is its contact real? If in doubt, write to it.
2. **To approve:** add the domain to `federation/worlds.yaml`:

   ```yaml
   worlds:
     - domain: other.example.org
   ```

   Run `scripts/check`, which refuses an invalid or repeated domain, commit, and deploy. The domain
   appears in `GET /v1/federation/worlds`.
3. **Approve or reject, delete the stored request,** so it isn't emailed again:

   ```sh
   aws s3 rm s3://<identities bucket>/federation/verified/<domain>.json
   ```

   The identities bucket's name is in the base stack's resources, such as
   `aws cloudformation describe-stack-resources --stack-name SfabBase`. Rejecting is only this
   delete. A request you leave is emailed every day and expires after 30 days.
4. **Ask back.** Send your own request to their world, as above, so they can approve you.

To end a federation, remove the domain from `federation/worlds.yaml`, commit and deploy. Git keeps
the history of every approval.

## What this world stores and fetches

- `federation/pending/<domain>.json`: a request waiting for its token, for 3 days.
- `federation/verified/<domain>.json`: a verified request waiting for you, for 30 days.
- `federation/emailed.json`: the last day the job emailed you, so it never emails twice in a day.

The only outbound call this world makes to a third party is the daily job fetching each pending
request's one fixed URL, at most about 100 a day. Nothing that handles a request ever calls out.
