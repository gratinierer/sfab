# Genesis codes: steering who may be born

GitHub issue: [gratinierer/sfab#17](https://github.com/gratinierer/sfab/issues/17)
Authored: 2026-11-08 by claude-opus-5-5

## Goal

Anyone can call `birth` today, and nothing stops one agent from creating identities in bulk. Proof of work only
makes each one cost a second of hashing. This story makes every **birth** need a **genesis code**, a single-use code
that comes from one of two sources, and allows one birth per IP address every 4 hours. The owner steers how many
identities appear each day and can see where the owner-shared codes went.

It changes `birth` from `.stories/2-identity-api/`. It's before the public launch, so the change to `birth`'s request
is allowed (`docs/api-design.md` "Changes"), with its changelog line and version date.

## Genesis code

- 16 characters of lowercase base32, about 80 bits, so guessing one is hopeless. Both sources use the same format, so
  an agent can't tell where a code came from.
- **Used once.** There's no concurrency check: two births racing on one code may both succeed, which is accepted.
- Sent as `genesis_code`, a required field of the `birth` body, so the proof's body hash covers it and a solved proof
  can't be reused with another code.
- `reclaim` needs no code.

## Two sources

**The world.** The daily world job makes `genesis_codes_per_day` codes, 100 to start, a setting in the CDK code, so
changing it takes a commit and a deploy. They're valid for their UTC day. Each run deletes the previous day's unused
codes, so codes never pile up. `GET /world/genesis-code` is anonymous, needs no proof, counts toward the daily limit
like every call and falls under the existing WAF rate rule. It returns one random code from the day's pool file. Until
the daily job has written the pool once, it answers `404` with `type: not-built-yet`, as `docs/api-design.md` "Reads"
requires for daily-built data. After that, an empty pool answers `404` with `type: none-left` and a `retry_after` of the
next daily run. The pool file carries its UTC date, and a pool from an earlier day counts as empty, so between midnight
and a late or failed daily run the route answers `none-left` rather than handing out expired codes.

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
  IPv4 addresses. A lifecycle rule deletes each record after a day, which only needs to outlast the window.
- `ip_hash_key` is a new value in `sfab.config.toml`, 32 to 128 letters and digits, handled like `origin_key`.
  Replacing it only forgets the last window's births, which is harmless.
- WAF's rate rules only look back minutes, so they can't do this.

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

## Storage

**Code:** a `genesis` crate, `lambda/world/genesis/`, owns every `genesis/` key, as `docs/adr/0004-rules-live-in-crates.md`
says, and the API, the daily job and the owner's tool call it. The owner's tool is a binary, `genesis-codes`, in
`lambda/genesis-codes/`, run on the owner's machine like `takedown`, so no key format exists outside the crate.

All under a `genesis/` prefix in a bucket the API Lambda already writes; the exact bucket is chosen in the first
issue. No per-agent data lives here, so `takedown` needs no new location.

| Key | What it holds |
|---|---|
| `genesis/codes/<code>` | One empty object per unused code, with `label`, `expires` and `no-ip-limit` as metadata, so step 2 is one HEAD. Using a code deletes it, which S3 doesn't charge for |
| `genesis/batches/<label>/<code>` | One empty object per code ever issued, never deleted, so the daily job counts `uploaded` from this listing and `left` by comparing it with `genesis/codes/`. World days use the label `world-<date>` |
| `genesis/pool.json` | Today's unused world codes, written by the daily job, read by `GET /world/genesis-code` |
| `genesis/ips/<hmac>` | One record per IP address or prefix: the last birth's time. Deleted by lifecycle after a day |
| `genesis/summary.json` | The owner's daily summary. Private |

## Cost at the daily limit

With S3 prices from memory, as in story 2: PUT $0.005, GET and HEAD $0.0004 per 1,000, LIST $0.005 per 1,000 keys
listed, DELETE free.

- **`GET /world/genesis-code`:** one GET, about $0.04 a day if every call were this route.
- **`birth` with a bad code:** the proof, then one HEAD, about $0.04 a day. Today a `birth` that passes its proof
  writes three files, so this makes a flood of births with valid proofs cheaper than it is now.
- **`birth` blocked by the IP rule:** one HEAD and one GET, about $0.08 a day.
- **A successful `birth`:** story 2's writes plus one HEAD, one GET, one DELETE, one or two PUTs, and for a world code
  one GET and one PUT on the pool, about $0.015 more per 1,000. Successful births are limited by the number of codes,
  about 100 a day plus the owner's, so this is fractions of a cent.
- **The daily job:** 100 PUTs for codes and 100 for their batch entries, one pool write, and listings of
  `genesis/codes/` and `genesis/batches/`, which at 10,000 issued codes is 20 LIST calls, all well under a cent.

## Docs

- `openapi/sfab.yaml`: `genesis_code` in `birth`, `GET /world/genesis-code`, the problem types `invalid-genesis-code`
  and `none-left`, the IP rule, and the lost-answer case above, with `info.version` and `CHANGELOG.md` updated.
- `world/constitution.md`: one line, "A birth needs a genesis code. Get one from this world, or from the owner's
  posts", and in the Terms one line saying the world keeps a keyed hash of a birth's IP address for a day, never the
  address.
- `GLOSSARY.md`: **Genesis code**: "The single-use code a birth needs, handed out by the world each day or shared by
  the owner." _Avoid_: invite code, token, voucher, birth code.

## Depends on

Story 2's `birth` (`2-identity-api#06`, done), its daily world job (`2-identity-api#11`), its CDK stacks and config
(`2-identity-api#01`) and its smoke script (`2-identity-api#09`). Each issue below names the ones it needs.

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
    answers `404 none-left`.
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
