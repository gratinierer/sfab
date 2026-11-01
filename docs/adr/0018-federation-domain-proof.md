# Federation requests prove their domain, and only a daily job calls out

Anyone can call `POST /federation/requests`, so a request has to prove it controls its domain before the owner hears about it. It works like Let's Encrypt's HTTP challenge, without using Let's Encrypt: the API returns a token, the requester publishes it at `https://<domain>/.well-known/sfab-federation/<token>`, and the daily world job fetches that URL. Only verified requests reach the owner, once a day, and the owner's commit to `worlds.yaml` remains the real gate.

This amends ADR 0011, which said nothing calls third-party servers, and ADR 0013, where the owner checked domains by hand. **Nothing that handles a request ever calls out. The only outbound call to a third party is the daily world job fetching that one fixed path.**

## Considered Options

- **Fetching from the API Lambda on request:** immediate, but a stranger could trigger our calls on demand, and slow servers could tie up the API's 5 concurrent slots.
- **A DNS challenge:** needs a DNS client and waits on propagation, while every world already serves files.
- **The owner checks by hand (ADR 0013):** no outbound call, but every random request reaches the owner.

## Consequences

- Fetches are bounded to 100 a day, over HTTPS on port 443, with no redirects, private addresses refused, a 3-second timeout and a 1 KB limit, and the response is only compared with the token.
- It proves control of a domain, not seriousness, since a domain costs about $10.
- Fake requests can fill the 100 pending places for 3 days at a time and block genuine ones. It costs nothing, and the published contact is the fallback.
