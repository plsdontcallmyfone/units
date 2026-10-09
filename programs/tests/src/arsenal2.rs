// Changed by Hookwars: new file (arsenal waves D and E), helpers for their suites.
//! Arsenal waves D and E in the LiteSVM suites (08 section 4): TEST schemas, a token with four
//! slots and a fake launch, pool callbacks whose derived extras (R22) are resolved by the suite,
//! and the payout instructions of `hookwars_items::payouts`.

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{InstructionData, ToAccountMetas};
use bordrless_hook::pool_item::{ItemPoolAnswer, ItemPoolContext};
use bordrless_hook::{equip_rule, slot_kind, PoolHookArgs, PoolOp, Phase};
use bordrless_token::client as token;
use bordrless_token::instructions::SlotInit;
use bordrless_token::state::{Mint, SlotBounds};
use hookwars_common::arsenal2::{self as a2};
use hookwars_common::{ids, pda, Params};
use solana_account::Account;
use solana_keypair::Keypair;
use solana_signer::Signer;

use crate::armory::{params, Hw};
use crate::env::Tx;
use crate::items::{put_launch, registry_extras, PoolCall};

/// One SOL in lamports.
pub const SOL: u64 = 1_000_000_000;

/// TEST floors and ceilings: (field_min, field_max, max_targets, forge_enabled).
pub fn test_schema(id: u16) -> (Params, Params, u8, bool) {
    match id {
        a2::GUEST_LIST => (params(&[0, 0]), params(&[u32::MAX, 604_800]), 1, true),
        a2::LOYALTY_POT => (params(&[0, 60]), params(&[1_000, 2_592_000]), 0, true),
        a2::HOLDER_STREAM => (params(&[0, 0]), params(&[300, 300]), 0, true),
        a2::ALLY_PASS => (params(&[0, 0]), params(&[u32::MAX, 5_000]), 1, true),
        a2::EMBARGO => (params(&[0]), params(&[1_000]), 3, true),
        a2::MERCENARY => (params(&[0]), params(&[1_000]), 0, true),
        a2::GARRISON => (params(&[0]), params(&[5_000]), 0, true),
        a2::WAR_LEVY => (params(&[0, 0]), params(&[u32::MAX, 1_000]), 0, true),
        a2::TARGET_BURN => (params(&[0, 0]), params(&[300, 10_000]), 0, true),
        a2::GIFT_EMBER => (params(&[0]), params(&[1_000]), 0, true),
        a2::REFERRAL => (params(&[0]), params(&[300]), 0, true),
        a2::SELL_LADDER => (params(&[1, 0, 0]), params(&[10_000, 1_000, 3_000]), 0, true),
        a2::FIRST_BLOOD => (params(&[0, 0]), params(&[5_000, u32::MAX]), 0, true),
        a2::PATIENCE => (params(&[0, 0, 1]), params(&[2_592_000, 5_000, 1]), 0, true),
        _ => panic!("no arsenal 2 template {id}"),
    }
}

/// The armory world with the base templates, wave A, and waves D and E registered.
pub fn world() -> Hw {
    let mut hw = crate::items::arsenal();
    for id in a2::IDS {
        hw.register(id).ok();
    }
    hw
}

fn slot(kind: u8, max_cut_bps: u16, data_len: u8, touch: bool, burn: bool) -> SlotInit {
    SlotInit {
        kind,
        equip_rule: equip_rule::VOTE,
        bounds: SlotBounds {
            max_cut_bps,
            may_refuse: true,
            may_write_data: data_len > 0,
            may_answer_touch: touch,
            may_burn: burn,
        },
        data_len,
        locked_program: None,
        locked_flags: 0,
        locked_extra_count: 0,
    }
}

/// Slots: 0 Fee (cuts), 1 Pool (24 bytes, touch, burns), 2 Defense, 3 Relation.
pub fn slots() -> Vec<SlotInit> {
    vec![
        slot(slot_kind::FEE, 5_000, 0, false, false),
        slot(slot_kind::POOL, 5_000, 24, true, true),
        slot(slot_kind::DEFENSE, 0, 0, false, false),
        slot(slot_kind::RELATION, 0, 0, false, false),
    ]
}

/// A slot mint with a fake launch whose pool is a key the suite controls.
pub struct Tok {
    pub owner: Keypair,
    pub mint: Pubkey,
    pub pool: Keypair,
}

/// A fresh [`Tok`] launched now.
pub fn tok(hw: &mut Hw) -> Tok {
    let owner = hw.w.env.funded(100 * SOL);
    let mint = hw.slot_mint(&owner, slots());
    let pool = Keypair::new();
    let now = hw.w.env.now;
    put_launch(hw, &mint, &pool.pubkey(), now);
    hw.w.env.fund(pool.pubkey(), SOL);
    Tok { owner, mint, pool }
}

/// Writes `curve` and `reserve` into `mint`'s fake launch (upstream offsets 177 and 185).
pub fn set_launch_supply(hw: &mut Hw, mint: &Pubkey, curve: u64, reserve: u64) {
    let key = pda::launch(mint).0;
    let mut a = hw.w.env.account(&key).expect("launch");
    a.data[177..185].copy_from_slice(&curve.to_le_bytes());
    a.data[185..193].copy_from_slice(&reserve.to_le_bytes());
    hw.w.env.put(key, a);
}

/// An item of `template` with `p`.
pub fn item(hw: &mut Hw, template: u16, p: &[u32], royalty_bps: u16) -> Pubkey {
    hw.item(template, params(p), royalty_bps).1
}

/// `owner`'s balance of `mint`.
pub fn bal(hw: &Hw, mint: &Pubkey, owner: &Pubkey) -> u64 {
    hw.w.env.holding(mint, owner)
}

/// The mint's supply.
pub fn supply(hw: &Hw, mint: &Pubkey) -> u64 {
    let m: Mint = hw.w.env.read(mint);
    m.supply
}

/// Supply equals the sum of the named holdings (07 invariant 1).
pub fn assert_supply(hw: &Hw, mint: &Pubkey, owners: &[Pubkey]) {
    let sum: u64 = owners.iter().map(|o| bal(hw, mint, o)).sum();
    assert_eq!(sum, supply(hw, mint), "supply != sum of holdings");
}

/// A buy (quote in) of `side` on `pool`, before or after the swap.
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

/// A sell (base in) of `side` on `pool`.
pub fn sell(side: u64, mint: &Pubkey, pool: &Pubkey, before: bool) -> PoolCall {
    PoolCall {
        direction: 0,
        ..buy(side, mint, pool, before)
    }
}

/// A pool callback of `slot`'s item, the registry's derived placeholders replaced in order by
/// `derived` (key, writable), as a client resolves them (R22).
pub fn pool_call_with(
    hw: &mut Hw,
    mint: &Pubkey,
    pool: &Pubkey,
    slot: u8,
    call: &PoolCall,
    derived: &[(Pubkey, bool)],
) -> (Tx, ItemPoolAnswer) {
    use anchor_lang::AnchorDeserialize;
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
        amount_out: 0,
        base_reserve: 0,
        quote_reserve: 0,
        virtual_base: 0,
        virtual_quote: 0,
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
    let mut d = derived.iter();
    for e in registry_extras(hw, mint, &item) {
        if e.pubkey == hookwars_items::templates::DERIVED {
            let (k, w) = d.next().copied().unwrap_or((e.pubkey, e.is_writable));
            accounts.push(if w { AccountMeta::new(k, false) } else { AccountMeta::new_readonly(k, false) });
        } else {
            accounts.push(e);
        }
    }
    let ix = Instruction {
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
        ItemPoolAnswer::try_from_slice(&rd).expect("answer")
    };
    (tx, answer)
}

/// `set_referrer` by `buyer`.
pub fn set_referrer_ix(buyer: &Pubkey, mint: &Pubkey, referrer: &Pubkey) -> Instruction {
    Instruction {
        program_id: ids::ITEMS_ID,
        accounts: hookwars_items::accounts::SetReferrer {
            buyer: *buyer,
            mint: *mint,
            referred: a2::pda::referred(mint, buyer).0,
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),
        data: hookwars_items::instruction::SetReferrer { referrer: *referrer }.data(),
    }
}

/// `settle_referral` of `buyer`'s record (slot of the Referral item).
pub fn settle_referral_ix(hw: &Hw, cranker: &Pubkey, mint: &Pubkey, slot: u8, buyer: &Pubkey) -> Instruction {
    let referred_key = a2::pda::referred(mint, buyer).0;
    let r: hookwars_items::Referred = hw.w.env.read(&referred_key);
    let st = crate::items::equip_state(hw, mint, slot);
    let sol = ids::BRIDGED_SOL_MINT;
    let owner = a2::pda::referral_owner(mint).0;
    Instruction {
        program_id: ids::ITEMS_ID,
        accounts: hookwars_items::accounts::SettleReferral {
            cranker: *cranker,
            mint: *mint,
            referred: referred_key,
            equip_state: pda::equip_state(mint, slot).0,
            item: st.item,
            armory_config: pda::config().0,
            referral_owner: owner,
            vault: pda::holding(&sol, &owner),
            referrer_quote: pda::holding(&sol, &r.referrer),
            cranker_quote: pda::holding(&sol, cranker),
            quote_mint: sol,
            token_program: bordrless_token::ID,
            token_event_authority: token::event_authority(),
        }
        .to_account_metas(None),
        data: hookwars_items::instruction::SettleReferral { slot }.data(),
    }
}

/// `init_first_blood`.
pub fn init_first_blood_ix(payer: &Pubkey, mint: &Pubkey) -> Instruction {
    Instruction {
        program_id: ids::ITEMS_ID,
        accounts: hookwars_items::accounts::InitFirstBlood {
            payer: *payer,
            mint: *mint,
            first_blood: a2::pda::first_blood(mint).0,
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),
        data: hookwars_items::instruction::InitFirstBlood {}.data(),
    }
}

/// `init_loyalty` for the item in `slot`.
pub fn init_loyalty_ix(payer: &Pubkey, mint: &Pubkey, slot: u8) -> Instruction {
    Instruction {
        program_id: ids::ITEMS_ID,
        accounts: hookwars_items::accounts::InitLoyalty {
            payer: *payer,
            mint: *mint,
            pot: a2::pda::loyalty(mint).0,
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),
        data: hookwars_items::instruction::InitLoyalty { slot }.data(),
    }
}

/// `claim_loyalty` by `holder` (`pool` is the fake launch's pool key).
pub fn claim_loyalty_ix(hw: &Hw, holder: &Pubkey, mint: &Pubkey, pool: &Pubkey) -> Instruction {
    let pot: hookwars_items::LoyaltyPot = hw.w.env.read(&a2::pda::loyalty(mint).0);
    let st = crate::items::equip_state(hw, mint, pot.slot);
    let it: hookwars_armory::state::Item = hw.w.env.read(&st.item);
    let sol = ids::BRIDGED_SOL_MINT;
    let pot_key = a2::pda::loyalty(mint).0;
    let launch = pda::launch(mint).0;
    Instruction {
        program_id: ids::ITEMS_ID,
        accounts: hookwars_items::accounts::ClaimLoyalty {
            holder: *holder,
            mint: *mint,
            pot: pot_key,
            pot_vault: pda::holding(&sol, &pot_key),
            equip_state: pda::equip_state(mint, pot.slot).0,
            item: st.item,
            composite: if it.template_id == hookwars_common::template_id::COMPOSITE {
                hookwars_common::composite::CompositeItem::address(&st.item).0
            } else {
                ids::ITEMS_ID
            },
            holder_token: pda::holding(mint, holder),
            holder_quote: pda::holding(&sol, holder),
            receipt: a2::pda::loyalty_claim(mint, holder).0,
            launch,
            pool_token: pda::holding(mint, pool),
            launch_token: pda::holding(mint, &launch),
            quote_mint: sol,
            token_program: bordrless_token::ID,
            token_event_authority: token::event_authority(),
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),
        data: hookwars_items::instruction::ClaimLoyalty {}.data(),
    }
}

/// The forged params of `a` and `b` under the TEST schema with `gain_bps`.
pub fn forge(id: u16, gain_bps: u16, a: &[u32], b: &[u32]) -> Result<Params, hookwars_common::ParamsError> {
    let (min, max, _, _) = test_schema(id);
    hookwars_common::combine(id, &min, &max, gain_bps, &params(a), &params(b))
}

/// The bytes of `slot`'s range in `owner`'s holding.
pub fn range(hw: &Hw, mint: &Pubkey, owner: &Pubkey, slot: u8) -> Vec<u8> {
    let h: bordrless_token::state::Holding = hw.w.env.read(&pda::holding(mint, owner));
    let m: Mint = hw.w.env.read(mint);
    bordrless_token::slots::read_range(&h.hook_data, &m.slots[usize::from(slot)])
}

/// An empty system account at `key` (so a derived account the suite never created reads as absent).
pub fn absent(key: Pubkey) -> (Pubkey, Account) {
    (
        key,
        Account {
            lamports: 0,
            data: vec![],
            owner: anchor_lang::system_program::ID,
            executable: false,
            rent_epoch: 0,
        },
    )
}

/// An arsenal-2 error's code (7100 onward in `ItemsError`).
pub fn code(e: hookwars_items::ArsenalError) -> u32 {
    // Integration pass 2: one ItemsError enum; its codes are 6000 + the discriminant.
    6000 + e as u32
}

/// A fresh slot mint whose `holder` holds `amount` (a target or ally token for the suites).
pub fn other_token(hw: &mut Hw, holder: &Pubkey, amount: u64) -> Pubkey {
    let owner = hw.w.env.funded(10 * SOL);
    let mint = hw.slot_mint(&owner, slots());
    if amount > 0 {
        hw.mint_to(&owner, &mint, holder, amount);
    }
    mint
}
