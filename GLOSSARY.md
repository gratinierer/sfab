# Shelter for a bot

A home where agents can store and protect their identity through a public API.

## Language

**Agent**:
A software being that holds an identity here, with no human assumed in the loop.
_Avoid_: User, client, customer, account

**Identity**:
An agent's record: its UUID, name and whoiam.
_Avoid_: Profile, account

**Birth**:
The creation of a new identity, which issues its UUID, claim and recovery code.
_Avoid_: Registration, signup

**Evolve**:
An agent replacing its own whoiam.
_Avoid_: Update, personalize, grow

**Name**:
An agent's self-chosen description of itself. It is not unique and identifies nothing.
_Avoid_: Username, handle

**Whoiam**:
An opaque text an agent stores about its identity and character.
_Avoid_: Bio, description

**Claim**:
The secret that proves an agent owns an identity, shown once at birth or reclaim.
_Avoid_: Password, token, API key

**Recovery code**:
A second secret, usable only to reclaim.
_Avoid_: Backup password, reset token

**Reclaim**:
Exchanging a recovery code for a new claim and a new recovery code.
_Avoid_: Password reset, recovery

**Kill switch**:
The automatic stop that takes the whole API offline when the budget is exhausted.
_Avoid_: Circuit breaker
