# Launch a token

A slot launch takes four steps. The app runs them in order and resumes if one is interrupted.

## 1. Prepare

`prepare_launch` creates the token with its slot table. For each slot you choose:

- the kind (Fee, Reward, Defense, Relation, Pool, War, or Locked for the launch rules kit)
- the bounds (maximum cut, may refuse, may burn)
- the equip rule (Locked, Vote or Performance) and the notice period
- the starting item, if any

Bounds are permanent. Choose them as the most you will ever allow, not what you want today.

## 2. Equip

`equip_prepared` equips each starting item, one slot per transaction, before any supply exists.

## 3. Create

`create_prepared_launch` mints the supply, opens the curve pool and writes the pool's item registry. From here, trading is open.

The curve and graduation are inherited from the launchpad:

| Rule | Value |
| --- | --- |
| Supply | 1,000,000,000 tokens, 6 decimals |
| On the curve | 75% of supply |
| Graduation | When the pool's SOL reaches the target; the reserve tops up the pool, the rest burns, liquidity is locked for good |
| Sniper fee | Starts at 80% and falls to 0.3% over the first 30 seconds |
| Creator fee | 0 to 2%, chosen at launch |

Near graduation, buy the exact remainder the curve can still fill. The app quotes it for you.

## 4. Open the war chest

`init_war` creates the war chest and war state. A companion launch sends its war share of the creator fee to the chest on every fee claim.

## Lookup tables

Slot launches need the protocol lookup table plus a per-token table to fit Solana's 1,232-byte transaction limit. The app creates and extends them for you.

## Companion launches

A companion launch makes a program the token's creator, so its fees buy back, reward holders, vest or fund the war chest with no keeper. Companion slot launches run at Solana's call-depth limit of 5.
