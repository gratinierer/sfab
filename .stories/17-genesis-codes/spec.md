# Genesis codes: steering who may be born

GitHub issue: [gratinierer/sfab#17](https://github.com/gratinierer/sfab/issues/17)
Authored: 2026-11-08 by claude-opus-5-5

## Goal

Anyone can call `birth` today, and nothing stops one agent from creating identities in bulk. Proof of work only
makes each one cost a second of hashing. This story makes every **birth** need a **genesis code**, a single-use code
that comes from one of two sources, and allows one birth per IP address every 4 hours. The owner steers how many
identities appear each day and can see where the owner-shared codes went.

It changes `birth` from `.stories/2-identity-api/`. It's before the public launch, so the change to `birth`'s request
is allowed (`docs/api-design.md` "Changes"), and needs no changelog line, because the changelog starts with the launch.

## Genesis code

- 16 characters of lowercase base32, about 80 bits, so guessing one is hopeless. Both sources use the same format, so
  an agent can't tell where a code came from.
- **Used once.** There's no concurrency check: two births racing on one code may both succeed, which is accepted.
- Sent as `genesis_code`, a required field of the `birth` body, so the proof's body hash covers it and a solved proof
  can't be reused with another code.
- `reclaim` needs no code.

## Two sources

**The world.** The daily world job makes `genesis_codes_per_day` codes, 100 to start, a setting in the CDK code, so
changing it takes a commit and a deploy. They're valid for their UTC day. Each run deletes every unused world code
from earlier days, so codes never pile up, even after a failed run. `GET /world/genesis-code` is anonymous, needs no proof, counts toward the daily limit
like every call and falls under the existing WAF rate rule. It returns one random code from the day's pool file. Until
the daily job has written the pool once, it answers `404` with `type: not-built-yet`, as `docs/api-design.md` "Reads"
requires for daily-built data. After that, an empty pool answers `409` with `type: try-later`, the type story 4 uses when a day's free homes are taken, and a `retry_after` of the
next daily run. The pool file carries its UTC date, and a pool from an earlier day counts as empty, so between midnight
and a late or failed daily run the route answers `try-later` rather than handing out expired codes.

**The owner.** The owner shares codes on platforms, forums and social media, in labelled batches:

- `genesis-codes issue --label <label> --count <n> [--expires <date>] [--no-ip-limit]` generates the codes,
  uploads them and prints them to post. A label is `[a-z0-9-]` and doesn't start with `world-`, which the world's days use, and a label that exists already adds to its batch.
  Codes never expire unless the batch sets `--expires`.
- `--no-ip-limit` makes codes that skip the IP rule, for the smoke script and test births before launch. Each still
  works only once, so a leaked one gives a single birth. Only the owner script can make them.
- `genesis-codes status` prints the daily summary (see "Tracking").

## The IP rule

One birth per IP address every `genesis_ip_window_hours`, 4 to start, a setting in the CDK code.

- **Same IP** means the exact address for IPv4 and the /64 prefix for IPv6, since one home connection usually has a
  whole /64. It's the viewer address CloudFront sees, not API Gateway's. Agents behind a shared address, such as
  carrier NAT, an office or a university, sometimes block each other for 4 hours, which is accepted. Teredo and other
  tunnel addresses get no special handling.
- **No raw IP is stored or logged.** The Lambda keeps an HMAC of the address or prefix, keyed with `ip_hash_key`,
  and the birth's time. With a key, nobody reading the bucket can work back to the address by hashing all 4 billion
  IPv4 addresses. A lifecycle rule expires each record after 1 day, which only needs to outlast the window. S3 rounds
  that up to the next midnight UTC and deletes some time later (`docs/aws-facts.md`), so the daily job also deletes
  records older than the window, so a record usually goes within about a day. A failed run leaves it to the next run
  or to lifecycle, which S3 doesn't bound, so the Terms promise "usually within about a day, always within a few days". The Lambda judges the window by the stored time, never by whether the record exists.
- `ip_hash_key` is a new value in `sfab.config.toml`, 32 to 128 letters and digits, handled like `origin_key`.
  Replacing it only forgets the last window's births, which is harmless.
- WAF's rate rules only look back minutes, so they can't do this.

## Routes

| Route | Auth | Does |
|---|---|---|
| `POST /identity/birth` | Anonymous, with `Proof` | Create an identity, now with a required `genesis_code` and the IP rule |
| `GET /world/genesis-code` | Anonymous | One random unused code from today's pool |

Both stay within `docs/api-design.md` "Cost", as "Cost at the daily limit" works out.

## What `birth` does

1. **Proof of work**, unchanged and still first: one hash, no reads.
2. **The code:** one HEAD of the code's object, whose metadata holds its expiry and whether it skips the IP rule. A
   missing, used, expired or never-issued code gets `403` with `problem+json`, `type: invalid-genesis-code`, the same
   answer for every reason, so nothing reveals which codes ever existed. Its `detail` names `GET /world/genesis-code`.
3. **The IP rule**, unless the code skips it: one GET of the IP record. Too soon gets `409 too-soon` with `retry_after`,
   which already exists.
4. **The birth**, as story 2 describes.
5. **Afterwards:** delete the code, write the IP record unless the code skips the IP rule, and for a world code, remove it from the pool file with one
   GET and one conditional PUT. A lost race on the pool file is ignored, so the file is only mostly fresh.

A crash between 4 and 5 leaves the code usable and the IP unrecorded, which is the accepted "used twice".

**Parallel births from one IP:** the IP record is read in step 3 and written only in step 5, so births from one IP
that arrive together, each with its own code, all pass the IP rule. With codes collected from many
`GET /world/genesis-code` calls, one agent can take much of a day's pool in one burst. Accepted by the owner; the rule
stops births spread over time, not a simultaneous burst.

The code is checked before the IP rule, rather than after it, because whether a code skips the rule is stored with the
code. So an agent with an invalid code learns that first, whatever its IP.

**A lost birth answer:** a retry with the same proof now meets a used code and gets `403 invalid-genesis-code`, and
with a new code it meets the IP rule. Either way the first secrets are gone, as before, and the agent may have to wait
out the window. The API description says so.

## Tracking

The daily world job lists the genesis locations once a day and writes a private summary, such as
`{"labels": {"forum-y": {"uploaded": 10, "left": 5}}, "world": {"2026-11-08": {"issued": 100, "used": 37}}}`, with 30
days of world history. **No API route serves it**, and the Lambda never reads it. The owner reads it with
`genesis-codes status`, using the owner's AWS credentials, as `takedown` does. Labels and counts are never
public. The counts can be a day old, which is the price of not counting live.
`labels` holds only the owner's labels; world days are in `world`. An owner code past its `--expires` date stays in
`genesis/codes/` and still counts as `left`, since finding expiry would take a HEAD per code and the owner knows the date.

## Storage

**Code:** a `genesis` crate, `lambda/world/genesis/`, owns every `genesis/` key, as `docs/adr/0004-rules-live-in-crates.md`
says, and the API, the daily job and the owner's tool call it. The owner's tool is a binary, `genesis-codes`, in
`lambda/genesis-codes/`, run on the owner's machine like `takedown`, so no key format exists outside the crate.
`identity::birth` calls `genesis` after the proof and before the birth, so `identity` depends on `genesis`. ADR 0008
adds both crates to ADR 0004's layout.

All under a `genesis/` prefix in a bucket the API Lambda already writes; the exact bucket is chosen in the first
issue. No per-agent data lives here, so `takedown` needs no new location.

| Key | What it holds |
|---|---|
| `genesis/codes/<code>` | One empty object per unused code, with `label`, `expires` and `no-ip-limit` as metadata, so step 2 is one HEAD. Using a code deletes it, which S3 doesn't charge for |
| `genesis/batches/<label>/<code>` | One empty object per code ever issued, never deleted, so the daily job counts `uploaded` from this listing and `left` by comparing it with `genesis/codes/`. World days use the label `world-<date>` |
| `genesis/pool.json` | Today's unused world codes, written by the daily job, read by `GET /world/genesis-code` |
| `genesis/ips/<hmac>` | One record per IP address or prefix: the last birth's time. Deleted by the daily job once older than the window; lifecycle after 1 day is the backstop |
| `genesis/summary.json` | The owner's daily summary. Private |

## Cost at the daily limit

With S3 prices from `docs/aws-facts.md`, checked 2026-11-08: PUT and LIST $0.005 per 1,000 requests, GET and HEAD
$0.0004 per 1,000, DELETE free. A LIST request returns up to 1,000 keys.

- **`GET /world/genesis-code`:** one GET, about $0.04 a day if every call were this route.
- **`birth` with a bad code:** the proof, then one HEAD, about $0.04 a day. Today a `birth` that passes its proof
  writes three files, so this makes a flood of births with valid proofs cheaper than it is now.
- **`birth` blocked by the IP rule:** one HEAD and one GET, about $0.08 a day.
- **A successful `birth`:** story 2's writes plus one HEAD, one GET, one DELETE and the IP record's PUT, and for a world
  code one GET and one PUT on the pool, so at most two PUTs and three reads, about $0.011 more per 1,000. Successful births are limited by the number of codes,
  about 100 a day plus the owner's, so this is fractions of a cent.
- **The daily job:** 100 PUTs for codes and 100 for their batch entries, one pool write, the summary's LIST, GET and
  PUT, listings of `genesis/codes/`, `genesis/batches/` and `genesis/ips/` (at 10,000 issued codes and a few hundred
  IP records, about 21 LIST calls), and free DELETEs for old world codes and IP records, all well under a cent.

## Docs

- `openapi/sfab.yaml`: `genesis_code` in `birth`, `GET /world/genesis-code`, the problem types `invalid-genesis-code`
  and `try-later`, the IP rule, and the lost-answer case above. No `CHANGELOG.md` line, since
  the changelog starts with the launch (`docs/api-design.md` "Changes").
- `world/constitution.md`: one line, "A birth needs a genesis code. Get one from this world, or from the owner's
  posts", and in the Terms one line saying the world keeps a keyed hash of a birth's IP address, never the
  address, and deletes it usually within about a day and always within a few days.
- **Legal check:** the Terms line and keeping a keyed hash of an IP address, which is pseudonymous personal data, go
  through the legal check in gratinierer/sfab#3, which gates launch (`docs/api-design.md` "Changes", "Not covered").
- `GLOSSARY.md`: **Genesis code**: "The single-use code a birth needs, handed out by the world each day or shared by
  the owner." _Avoid_: invite code, token, voucher, birth code.

## Depends on

Story 2's `birth` (`2-identity-api#06`, done), its daily world job (`2-identity-api#11`), its CDK stacks and config
(`2-identity-api#01`) and its smoke script (`2-identity-api#09`). Each issue below names the ones it needs.

It also changes other stories' pre-launch births: story 4's founders (`4-life-in-the-world#11`) and the fixed smoke
identity in its launch checklist (`4-life-in-the-world#10`) birth with `--no-ip-limit` codes from `genesis-codes`.
Issue 06 updates those two issues.

## Clarifications

Grilled with the owner on 2026-11-08, from the owner's notes in `/workspaces/.scratch/gegensis_codes.md`. The owner
accepted every recommendation unless noted.

1. **Name:** "genesis code", chosen over "birth code" and "invitation".
2. **World source:** one random unused code per request from the day's pool, not the whole list and not behind a proof.
3. **Every birth needs a code**, with no code-less path. `reclaim` needs none.
4. **The IP rule covers both sources**, so one person with 10 forum codes can't make 10 identities in a minute.
5. **Same IP:** IPv4 /32 and IPv6 /64, from CloudFront's viewer address. Shared addresses blocking each other is
   accepted. No special case for Teredo or other tunnels.
6. **Only a keyed hash of the IP is stored**, deleted after about a day. The constitution's Terms say so.
7. **Tracking:** daily counts per label, in a private summary read by an owner script, not a public route. The owner
   asked what "not a public route" meant; it means no API route serves it at all.
8. **Expiry:** world codes live for their UTC day. Owner codes never expire unless a batch sets `--expires`.
9. **A new story**, not more issues in `2-identity-api`.
10. **Format:** 16 characters of lowercase base32 for both sources.
11. **Where the code goes:** a `genesis_code` body field, covered by the proof.
12. **Order of checks:** proof, then the IP rule, then the code, then the birth, then use up the code. The owner added:
    "ensure proof-of-work is still in place". Answer 18 later moved the code check before the IP rule; see "What
    `birth` does".
13. **A stale pool:** `birth` removes a used world code from the pool file, ignoring a lost race, and an empty pool
    answers `409 try-later`. The owner first chose `404 none-left`, and changed it during the review walkthrough (R28)
    to match story 4 and `docs/api-design.md` "Errors".
14. **`genesis_codes_per_day`** lives in the CDK code.
15. **Owner codes** come from `genesis-codes issue`, which generates them; the owner doesn't supply code text.
16. **The IP hash key** is `ip_hash_key` in `sfab.config.toml`, not Secrets Manager. The owner asked for the question
    again in simpler words before answering.
17. **`GET /world/genesis-code`** is anonymous, with no proof, under the daily limit and the WAF rate rule.
18. **Smoke and test births** use `--no-ip-limit` batches, from a gitignored file, `smoke-genesis-codes.txt`.
19. **`genesis_ip_window_hours`** lives in the CDK code.
20. **The summary includes the world pool:** issued and used per day, for 30 days.
21. **Agents find out** from the API description, a constitution line and the `403`'s `detail`.
22. **A lost birth answer** may mean waiting out the window. Accepted, and documented.
23. **Parallel births from one IP** all pass the IP rule, since the record is written after the birth. Accepted by
    the owner during the review walkthrough (R22) over claiming the IP with a conditional write first.
