# External tokens

Any Token-2022 token, from any protocol, can run units items as its transfer hook. The token pays for them the way units tokens do: with a licence, or by owning the item.

## How it works

1. **Point the hook at the gate.** The mint's transfer-hook authority sets the transfer hook program to the units gate. This is a Token-2022 setting on the mint itself.
2. **Register.** The same authority registers the mint with the gate and names its **venue**: the authority of the AMM pool the token trades in. Transfers from the venue count as buys, transfers to it as sells.
3. **Get the item.** Buy a licence for the item and your mint on the item's page, or buy the item and send it to the gate's vault for your mint.
4. **Bind.** Bind the item to one of four slots. From then on every transfer runs it.

## What runs

| Can | Cannot |
| --- | --- |
| Refuse a transfer (cooldowns, caps, dust, max transaction) | Take a cut: a Token-2022 hook has no authority over the tokens moved |
| Keep per-wallet memory (last buy, streaks, tags) | Burn or discount |
| Read prices | Run pool-side or war templates usefully: those need units pools and war state |

Items that may cut, external templates and exclusive items are refused when you bind them.

## Licences and lapses

A binding is checked on every transfer. When its licence ends or is revoked, or the vault no longer holds the item, the binding stops doing anything: every transfer goes through. Anyone may then unbind it, so a lapsed binding never stays in your token's transfers. Renew the licence to keep the item running.

## Holder memory

Token-2022 accounts have no room for hook data, so the gate keeps each wallet's memory in its own account. A **strict** mint refuses transfers to or from a wallet without one while a bound item keeps memory; the venue is exempt. Anyone can open a wallet's memory and pay its rent, so wallets usually get it when they first get a token account.

## Limits

- Up to four items per mint, and at most 30 extra accounts in a transfer.
- A DEX that calls Token-2022 adds two call levels (the gate and the items program). A router in front of that DEX reaches the runtime's limit of five.
- The gate's admin can pause new registrations and binds. A pause never blocks transfers.

The page for all of this is [/gate](/gate).
