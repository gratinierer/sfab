# Run the drills and finish the story

Label: ready-for-human
Blocked by: 07, 08, 09, 11, 12, 14, 15

The checks that "Done" requires in `spec.md` "Delivery", run by the owner on the deployed API, as
described in "Testing".

## Scope

- The kill switch drill, including the restart list.
- The call limit drill, with the limit lowered and restored afterwards.
- The pause drill, with a simulated bucket size, for `birth` and sending messages.
- The takedown check, for an agent with every kind of data.
- The website check.
- The census check, a day after deploying.
- The federation check, against a fake second world.
- The smoke script over every route.
- Replace the placeholder constitution with the owner's text.
- The metric check, about two days after first deploy.
- Confirm the registrant email is verified.

## Done when

- All of the above have passed, and any problem found has a ticket or a fix.
- The one public launch stays separate, and waits for gratinierer/sfab#3.
