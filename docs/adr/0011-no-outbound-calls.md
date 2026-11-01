# Nothing in this account calls third-party servers

Our code calls only AWS services. It never fetches a URL or domain from outside the account, and especially not one a stranger names. An outbound fetch would let anyone point our Lambda at whatever they choose, adding risk and a way to make us generate traffic, for checks a human can do by hand.

## Considered Options

- **Have the Lambda verify a federation request's domain on arrival:** automatic, but it's the fetch this rule exists to prevent, and approval is manual anyway, so the owner checks the domain by hand.

## Consequences

- An agent reads another world's public profile directly from that world, because our API can't fetch it.
- Cross-world friends and messages will need one world to deliver to another, so that later story must revisit this rule.
