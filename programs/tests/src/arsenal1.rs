// Changed by Hookwars: new file (arsenal waves B and C), helpers for the wave B and C template
// suites (docs/spec/08-arsenal.md section 5.2).
//! On top of the items harness ([`crate::items`]):
//! - a slot mint whose slots fit the wave B and C kinds: 0 Pool, 1 Defense (12 bytes), 2 Reward
//!   (7 bytes, touch), 3 Relation;
//! - a fake launch whose pool is a keypair (so the suite signs buys and sells), or a fake launch
//!   pool at the real launch pool address with an observation ring in its tail;
//! - a delegate on a pool holding, so a suite can move tokens out of a pool it cannot sign for;
//! - range readers and the 07 section 4 checks a suite runs after every step.

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anchor_lang::{AccountDeserialize, AccountSerialize, Discriminator};
use bordrless_hook::{equip_rule, slot_kind};
use bordrless_token::client as token;
use bordrless_token::instructions::SlotInit;
use bordrless_token::slots::SlotOp;
use bordrless_token::state::{Holding, Mint, SlotBounds};
use hookwars_common::{ids, pda};
use solana_account::Account;
use solana_keypair::Keypair;
use solana_signer::Signer;

use crate::armory::{params, Hw};
use crate::env::Tx;
use crate::items::{arsenal, items_extras, put_launch, PoolCall};
use crate::ring::{ring, with_ring, RingEntry};

/// One SOL in lamports.
pub const SOL: u64 = 1_000_000_000;
/// Q64.64 one.
pub const Q64: u128 = 1 << 64;

fn slot(kind: u8, max_cut_bps: u16, data_len: u8, touch: bool) -> SlotInit {
    SlotInit {
        kind,
        equip_rule: equip_rule::VOTE,
        bounds: SlotBounds {
            max_cut_bps,
            may_refuse: true,
            may_write_data: data_len > 0,
            may_answer_touch: touch,
            may_burn: false,
        },
        data_len,
        locked_program: None,
        locked_flags: 0,
        locked_extra_count: 0,
    }
}

/// Slot 0 Pool, 1 Defense (11 bytes plus the epoch byte), 2 Reward (6 plus the epoch, touch),
/// 3 Relation.
pub fn slots() -> Vec<SlotInit> {
    vec![
        slot(slot_kind::POOL, 5_000, 0, false),
        slot(slot_kind::DEFENSE, 0, 12, false),
        slot(slot_kind::REWARD, 0, 7, true),
        slot(slot_kind::RELATION, 0, 0, false),
    ]
}

/// A token for the suites.
pub struct Tok {
    pub owner: Keypair,
    pub mint: Pubkey,
    pub pool: Keypair,
}

/// The armory world with wave A and `ids` registered.
pub fn world(ids: &[u16]) -> Hw {
    let mut hw = arsenal();
    for id in ids {
        hw.register(*id).ok();
    }
    hw
}

/// A slot mint with [`slots`] whose launch names a keypair pool (buys and sells are signed).
pub fn tok(hw: &mut Hw) -> Tok {
    let owner = hw.w.env.funded(100 * SOL);
    let mint = hw.slot_mint(&owner, slots());
    let pool = Keypair::new();
    let now = hw.w.env.now;
    put_launch(hw, &mint, &pool.pubkey(), now);
    hw.w.env.fund(pool.pubkey(), SOL);
    Tok { owner, mint, pool }
}

/// An item of `template` with `p`.
pub fn item(hw: &mut Hw, template: u16, p: &[u32], royalty_bps: u16) -> Pubkey {
    hw.item(template, params(p), royalty_bps).1
}

/// The launch pool address of `mint`.
pub fn launch_pool(mint: &Pubkey) -> Pubkey {
    hookwars_items::templates::launch_pool_address(mint)
}

/// Writes a pool account at `mint`'s launch pool address, owned by the DEX, with `swap_count` and
/// `quote_volume` and a ring of `entries` (price `last_price` since `last_ts`), and a `Launch`
/// naming it. Returns the pool key.
pub fn put_market(hw: &mut Hw, mint: &Pubkey, swap_count: u64, last_price: u128, last_ts: i64, entries: &[RingEntry]) -> Pubkey {
    let key = launch_pool(mint);
    let mut zero = vec![0u8; bordrless_swap::state::Pool::LEN];
    zero[..8].copy_from_slice(bordrless_swap::state::Pool::DISCRIMINATOR);
    let mut p = bordrless_swap::state::Pool::try_deserialize(&mut &zero[..]).expect("zeroed pool");
    p.base_mint = *mint;
    p.quote_mint = ids::BRIDGED_SOL_MINT;
    p.swap_count = swap_count;
    let mut data = Vec::new();
    p.try_serialize(&mut data).expect("pool");
    let data = with_ring(data, &ring(&key, last_price, last_ts, entries));
    let lamports = hw.w.env.rent(data.len());
    hw.w.env.put(
        key,
        Account {
            lamports,
            data,
            owner: bordrless_swap::ID,
            executable: false,
            rent_epoch: 0,
        },
    );
    let now = hw.w.env.now;
    put_launch(hw, mint, &key, now);
    key
}

/// A slot mint whose launch names its real launch pool address (a market from [`put_market`]
/// written later by the suite).
pub fn tok_market(hw: &mut Hw) -> (Keypair, Pubkey) {
    let owner = hw.w.env.funded(100 * SOL);
    let mint = hw.slot_mint(&owner, slots());
    let now = hw.w.env.now;
    put_launch(hw, &mint, &launch_pool(&mint), now);
    (owner, mint)
}

/// Gives `owner`'s holding of `mint` a delegate (written directly: suites cannot sign for a pool
/// address). Returns the delegate.
pub fn delegate(hw: &mut Hw, mint: &Pubkey, owner: &Pubkey) -> Keypair {
    let d = hw.w.env.funded(SOL);
    let key = pda::holding(mint, owner);
    let mut h: Holding = hw.w.env.read(&key);
    h.delegate = Some(d.pubkey());
    h.delegated_amount = u64::MAX;
    let mut data = Vec::new();
    h.try_serialize(&mut data).expect("holding");
    let mut acc = hw.w.env.account(&key).expect("holding account");
    acc.data[..data.len()].copy_from_slice(&data);
    hw.w.env.put(key, acc);
    d
}

/// A transfer of `amount` of `mint` from `from_owner`'s holding to `to`'s, signed by `authority`
/// (the owner or a delegate); the destination holding is created first.
pub fn send(hw: &mut Hw, authority: &Keypair, mint: &Pubkey, from_owner: &Pubkey, to: &Pubkey, amount: u64) -> Tx {
    let extras: Vec<AccountMeta> = items_extras(hw, mint, SlotOp::Transfer);
    let ixs = [
        token::create_holding(authority.pubkey(), *mint, *to),
        token::transfer_with(
            authority.pubkey(),
            token::holding_address(mint, from_owner),
            token::holding_address(mint, to),
            *mint,
            None,
            extras,
            amount,
        ),
    ];
    hw.w.env.send_paid_by(&ixs, authority, &[])
}

/// The bytes of `slot`'s range in `owner`'s holding (without the epoch byte; zeros when stale).
pub fn range(hw: &Hw, mint: &Pubkey, owner: &Pubkey, slot: u8) -> Vec<u8> {
    let h: Holding = hw.w.env.read(&pda::holding(mint, owner));
    let m: Mint = hw.w.env.read(mint);
    bordrless_token::slots::read_range(&h.hook_data, &m.slots[usize::from(slot)])
}

/// The balance of `owner`'s holding of `mint`.
pub fn bal(hw: &Hw, mint: &Pubkey, owner: &Pubkey) -> u64 {
    hw.w.env.holding(mint, owner)
}

/// 07 section 4, invariants 1, 3 and 7 for the holdings a suite uses: the supply is the sum of the
/// holdings named; each equip vault holds exactly its unsettled records; each range keeps its
/// epoch byte (`read_range` gives `data_len - 1` bytes) and a holding of 0 holds a zero range.
pub fn check(hw: &Hw, mint: &Pubkey, owners: &[Pubkey]) {
    let m: Mint = hw.w.env.read(mint);
    let mut all: Vec<Pubkey> = owners.to_vec();
    for s in 0..4u8 {
        all.push(pda::equip_state(mint, s).0);
    }
    all.sort();
    all.dedup();
    let sum: u64 = all.iter().map(|o| bal(hw, mint, o)).sum();
    assert_eq!(sum, m.supply, "supply equals the holdings");
    for s in 0..4u8 {
        let key = pda::equip_state(mint, s).0;
        if let Some(acc) = hw.w.env.account(&key) {
            if acc.owner == ids::ITEMS_ID && !acc.data.is_empty() {
                let st = hookwars_items::EquipState::try_deserialize(&mut &acc.data[..]).expect("equip state");
                let rec: u64 = st.token_unsettled.iter().sum();
                assert_eq!(bal(hw, mint, &key), rec, "equip vault {s} holds its records");
            }
        }
    }
    for o in owners {
        if hw.w.env.account(&pda::holding(mint, o)).is_none() {
            continue;
        }
        for s in 0..4u8 {
            let sl = &m.slots[usize::from(s)];
            let r = range(hw, mint, o, s);
            assert_eq!(r.len(), usize::from(sl.data_len.saturating_sub(1)), "range length");
            if bal(hw, mint, o) == 0 {
                assert!(r.iter().all(|b| *b == 0), "an empty holding keeps no bytes in slot {s}");
            }
        }
    }
}

/// A pool callback: a buy of `side` quote.
pub fn buy(side: u64, mint: &Pubkey, pool: &Pubkey, before: bool) -> PoolCall {
    PoolCall {
        before,
        direction: 1,
        actor: Pubkey::new_unique(),
        recipient: Pubkey::new_unique(),
        amount_in: side,
        side_amount: side,
        route: crate::items::plain_route(mint, pool, side),
    }
}

/// A pool callback: a sell whose quote side is `side`.
pub fn sell(side: u64, mint: &Pubkey, pool: &Pubkey, before: bool) -> PoolCall {
    PoolCall {
        direction: 0,
        ..buy(side, mint, pool, before)
    }
}

/// A ring with price `p` constant from `t0` (one entry at `t0`).
pub fn flat(t0: i64) -> Vec<RingEntry> {
    vec![(t0, 0, 0, 0)]
}

/// A ring with price `p1` from `t0` to `t1` (entries at both), then the header's price after.
pub fn two_prices(t0: i64, t1: i64, p1: u128, swaps_at_t1: u64) -> Vec<RingEntry> {
    vec![(t0, 0, 0, 0), (t1, p1 * (t1 - t0) as u128, 0, swaps_at_t1)]
}

/// Reserves and amounts a pool callback is told (the items harness passes zeros).
#[derive(Clone, Copy, Default)]
pub struct Book {
    pub base_reserve: u64,
    pub quote_reserve: u64,
    pub virtual_base: u64,
    pub virtual_quote: u64,
    pub amount_out: u64,
}

/// A pool-item callback like [`crate::items::pool_call`], with the reserves and amounts of `book`.
pub fn pool_call_book(
    hw: &mut Hw,
    mint: &Pubkey,
    pool: &Pubkey,
    slot: u8,
    call: &PoolCall,
    book: Book,
) -> (Tx, bordrless_hook::pool_item::ItemPoolAnswer) {
    use anchor_lang::{AnchorDeserialize, InstructionData};
    use bordrless_hook::pool_item::ItemPoolContext;
    use bordrless_hook::{Phase, PoolHookArgs, PoolOp};
    let m: Mint = hw.w.env.read(mint);
    let item = m.slots[usize::from(slot)].item;
    let args = PoolHookArgs {
        op: PoolOp::Swap,
        phase: if call.before { Phase::Before } else { Phase::After },
        pool: *pool,
        base_mint: *mint,
        quote_mint: ids::BRIDGED_SOL_MINT,
        actor: call.actor,
        recipient: call.recipient,
        direction: call.direction,
        amount_in: call.amount_in,
        amount_out: book.amount_out,
        base_reserve: book.base_reserve,
        quote_reserve: book.quote_reserve,
        virtual_base: book.virtual_base,
        virtual_quote: book.virtual_quote,
        lp_fee_bps: 30,
        protocol_fee_bps: 0,
        swap_count: 0,
        created_at: 0,
        lp_amount: 0,
        hook_data: vec![],
        route: call.route.clone(),
    };
    let ctx = ItemPoolContext {
        slot,
        item,
        launch_fee_bps: 100,
        launch_cut: 0,
        side_amount: call.side_amount,
    };
    let data = if call.before {
        hookwars_items::instruction::PoolBeforeSwap { args, item_ctx: ctx }.data()
    } else {
        hookwars_items::instruction::PoolAfterSwap { args, item_ctx: ctx }.data()
    };
    let mut accounts = vec![
        AccountMeta::new_readonly(ids::ITEMS_ID, false),
        AccountMeta::new_readonly(hookwars_items::LAUNCH_ITEMS_SIGNER, false),
        AccountMeta::new_readonly(*pool, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(ids::BRIDGED_SOL_MINT, false),
        AccountMeta::new_readonly(call.actor, false),
    ];
    accounts.extend(crate::items::registry_extras(hw, mint, &item));
    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: ids::LAUNCH_ID,
        accounts,
        data: launch_stub::instruction::AsItemsPool { data }.data(),
    };
    let payer = hw.w.env.payer.insecure_clone();
    let tx = hw.w.env.send(&[ix], &[&payer]);
    let rd = if tx.result.is_ok() { tx.return_data() } else { vec![] };
    let answer = if rd.is_empty() {
        Default::default()
    } else {
        bordrless_hook::pool_item::ItemPoolAnswer::try_from_slice(&rd).expect("answer")
    };
    (tx, answer)
}

/// Moves the clock forward to the next time whose UTC hour is `hour`, plus `secs` into it.
pub fn warp_to_hour(hw: &mut Hw, hour: i64, secs: i64) {
    let now = hw.w.env.now;
    let target = hour * 3_600 + secs;
    let into = now.rem_euclid(86_400);
    let delta = (target - into).rem_euclid(86_400);
    if delta > 0 {
        hw.w.env.warp(delta);
    }
}

/// Every pool template's answer, on a buy's and a sell's two callbacks, is a cut on the quote side
/// only (wave A's rule): `(buy before, buy after, sell before, sell after)` cuts.
pub fn four_cuts(hw: &mut Hw, mint: &Pubkey, pool: &Pubkey, slot: u8, side: u64) -> (u64, u64, u64, u64) {
    let a = crate::items::pool_call(hw, mint, pool, slot, &buy(side, mint, pool, true)).1.cut;
    let b = crate::items::pool_call(hw, mint, pool, slot, &buy(side, mint, pool, false)).1.cut;
    let c = crate::items::pool_call(hw, mint, pool, slot, &sell(side, mint, pool, true)).1.cut;
    let d = crate::items::pool_call(hw, mint, pool, slot, &sell(side, mint, pool, false)).1.cut;
    (a, b, c, d)
}

/// `bordrless_core::fee_amount` as the templates apply it.
pub fn fee(amount: u64, bps: u32) -> u64 {
    hookwars_items::templates::fee(amount, bps)
}
