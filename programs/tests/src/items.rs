// Changed by Hookwars: new file (M3b), helpers for the items program's callbacks, templates,
// composites and settlement. Integration pass 2: derived registry entries.
//! The items program in the LiteSVM suites, on top of the armory world ([`Hw`]):
//! - a fake launch for a slot mint (`Launch` naming a pool key the suite controls, so a transfer
//!   out of that key's holding is a "buy" and into it a "sell");
//! - item registries resolved into each called slot's slice (`items_extras`);
//! - pool callbacks driven through `launch_stub::as_items_pool`, which signs as the launchpad's
//!   items signer (the launchpad's own forwarding is the launchpad branch's);
//! - fake `WarConfig` and `WarState` accounts with TEST values;
//! - `settle_equip`.

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{AccountSerialize, InstructionData, ToAccountMetas};
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::{AccountSource, HookAccountList, PoolHookArgs, PoolOp, Phase, RouteContext, HOOK_ACCOUNTS_SEED};
use bordrless_token::client as token;
use bordrless_token::slots::{is_called, SlotOp};
use bordrless_token::state::Mint;
use hookwars_common::composite::{CompositeItem, Module};
use hookwars_common::{ids, pda, template_id as t, Params};
use solana_account::Account;
use solana_keypair::Keypair;
use solana_signer::Signer;

use crate::armory::{armory_events, armory_ix, token_accounts, Hw};
use crate::env::Tx;

/// The wave A templates and Composite (08 section 5.1).
pub const ARSENAL: [u16; 7] = [
    t::SIZE_TIERS,
    t::SIDE_SKEW,
    t::LAUNCH_DECAY,
    t::MAX_TRANSACTION,
    t::DUST_GUARD,
    t::SELL_BURN,
    t::COMPOSITE,
];

/// Registers the arsenal templates of [`ARSENAL`].
pub fn register_arsenal(hw: &mut Hw) {
    for id in ARSENAL {
        hw.register(id).ok();
    }
}

/// The armory world with the nine base templates and the arsenal registered.
pub fn arsenal() -> Hw {
    let mut hw = Hw::new();
    register_arsenal(&mut hw);
    hw
}

/// `Launch` data naming `mint` and `pool`, created at `created_at` (upstream offsets: mint 10,
/// pool 74, created_at 289).
pub fn put_launch(hw: &mut Hw, mint: &Pubkey, pool: &Pubkey, created_at: i64) {
    let mut data = vec![0u8; 565];
    data[..8].copy_from_slice(&hookwars_common::LAUNCH_DISCRIMINATOR);
    data[10..42].copy_from_slice(mint.as_ref());
    data[74..106].copy_from_slice(pool.as_ref());
    data[hookwars_common::LAUNCH_CREATED_AT_OFFSET..hookwars_common::LAUNCH_CREATED_AT_OFFSET + 8]
        .copy_from_slice(&created_at.to_le_bytes());
    let lamports = hw.w.env.rent(data.len());
    hw.w.env.put(
        pda::launch(mint).0,
        Account {
            lamports,
            data,
            owner: ids::LAUNCH_ID,
            executable: false,
            rent_epoch: 0,
        },
    );
}

fn put_anchor<T: AccountSerialize>(hw: &mut Hw, key: Pubkey, owner: Pubkey, value: &T, len: usize) {
    let mut data = Vec::new();
    value.try_serialize(&mut data).expect("serialize");
    data.resize(data.len().max(len), 0);
    let lamports = hw.w.env.rent(data.len());
    hw.w.env.put(
        key,
        Account {
            lamports,
            data,
            owner,
            executable: false,
            rent_epoch: 0,
        },
    );
}

/// A `WarConfig` with the war suites' TEST parameters and `season` running.
pub fn put_war_config(hw: &mut Hw, season: u32) {
    let c = hookwars_war::state::WarConfig {
        version: 1,
        bump: pda::war_config().1,
        prize_vault_bump: 0,
        admin: Pubkey::default(),
        protocol_treasury: Pubkey::default(),
        randomness_program: Pubkey::default(),
        treaty_template_id: None,
        current_season: season,
        last_winner: None,
        last_winner_season: 0,
        params: crate::war::TEST_PARAMS,
        pending: None,
        reserved: [0; 64],
    };
    put_anchor(hw, pda::war_config().0, ids::WAR_ID, &c, hookwars_war::state::WarConfig::LEN);
}

/// A `WarState` of `mint`, under siege until `until` by `chest`.
pub fn put_war_state(hw: &mut Hw, mint: &Pubkey, until: i64, chest: Pubkey) {
    let mut s: hookwars_war::state::WarState = zeroed_war_state();
    s.version = 1;
    s.bump = pda::war_state(mint).1;
    s.mint = *mint;
    s.under_siege_until = until;
    s.siege_by_chest = chest;
    put_anchor(hw, pda::war_state(mint).0, ids::WAR_ID, &s, hookwars_war::state::WarState::LEN);
}

fn zeroed_war_state() -> hookwars_war::state::WarState {
    use anchor_lang::AccountDeserialize;
    let mut data = vec![0u8; hookwars_war::state::WarState::LEN];
    data[..8].copy_from_slice(
        <hookwars_war::state::WarState as anchor_lang::Discriminator>::DISCRIMINATOR,
    );
    hookwars_war::state::WarState::try_deserialize(&mut &data[..]).expect("zeroed war state")
}

/// The registry of `item` on `mint`, resolved (fixed keys; derived entries as `DERIVED`).
pub fn registry_extras(hw: &Hw, mint: &Pubkey, item: &Pubkey) -> Vec<AccountMeta> {
    let key = Pubkey::find_program_address(&[HOOK_ACCOUNTS_SEED, mint.as_ref(), item.as_ref()], &ids::ITEMS_ID).0;
    let data = hw.w.env.account(&key).expect("registry").data;
    let list = HookAccountList::decode(&data).expect("registry list");
    list.accounts
        .iter()
        .map(|e| match &e.source {
            AccountSource::Key(k) => {
                if e.writable {
                    AccountMeta::new(*k, false)
                } else {
                    AccountMeta::new_readonly(*k, false)
                }
            }
            // Integration pass 2 (08 arsenal 2 request 1): a derived entry (resolved per trade by
            // clients) comes back as the `DERIVED` placeholder the suites substitute.
            AccountSource::Pda { .. } => {
                if e.writable {
                    AccountMeta::new(hookwars_items::templates::DERIVED, false)
                } else {
                    AccountMeta::new_readonly(hookwars_items::templates::DERIVED, false)
                }
            }
        })
        .collect()
}

/// The remaining accounts of `op` on a slot mint whose item slots run the items program.
pub fn items_extras(hw: &Hw, mint: &Pubkey, op: SlotOp) -> Vec<AccountMeta> {
    let m: Mint = hw.w.env.read(mint);
    let mut v = Vec::new();
    for s in m.active_slots().iter() {
        if !is_called(s, op) {
            continue;
        }
        v.extend(token::slot_slice(s.program, registry_extras(hw, mint, &s.item)));
    }
    v
}

/// A transfer of a slot mint between two owners' holdings (the destination's created first).
pub fn transfer(hw: &mut Hw, from: &Keypair, mint: &Pubkey, to: &Pubkey, amount: u64) -> Tx {
    let ixs = [
        token::create_holding(from.pubkey(), *mint, *to),
        token::transfer_with(
            from.pubkey(),
            token::holding_address(mint, &from.pubkey()),
            token::holding_address(mint, to),
            *mint,
            None,
            items_extras(hw, mint, SlotOp::Transfer),
            amount,
        ),
    ];
    hw.w.env.send_paid_by(&ixs, from, &[])
}

/// Equips `item` in `slot` of `mint` at launch (through `launch_stub`), with `targets`/`role`.
pub fn equip(hw: &mut Hw, owner: &Keypair, mint: &Pubkey, slot: u8, item: Pubkey, targets: Vec<Pubkey>, role: u8) -> Tx {
    let cfg = hookwars_common::EquipConfig { targets, role };
    hw.equip_launch(owner, mint, Hw::entry(slot, Some(item), cfg))
}

/// `init_raid_ledger` of `mint`.
pub fn init_ledger(hw: &mut Hw, payer: &Keypair, mint: &Pubkey) -> Tx {
    let ix = Instruction {
        program_id: ids::ITEMS_ID,
        accounts: hookwars_items::accounts::InitRaidLedger {
            payer: payer.pubkey(),
            mint: *mint,
            raid_ledger: pda::raid_ledger(mint).0,
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),
        data: hookwars_items::instruction::InitRaidLedger {}.data(),
    };
    hw.w.env.send_paid_by(&[ix], payer, &[])
}

/// What a pool callback is told.
pub struct PoolCall {
    pub before: bool,
    pub direction: u8,
    pub actor: Pubkey,
    pub recipient: Pubkey,
    pub amount_in: u64,
    pub side_amount: u64,
    pub route: RouteContext,
}

/// A pool-item callback of `slot`'s item on `mint`, forwarded by `launch_stub` as the launchpad.
pub fn pool_call_ix(hw: &Hw, mint: &Pubkey, pool: &Pubkey, slot: u8, call: &PoolCall) -> Instruction {
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
    accounts.extend(registry_extras(hw, mint, &item));
    Instruction {
        program_id: ids::LAUNCH_ID,
        accounts,
        data: launch_stub::instruction::AsItemsPool { data }.data(),
    }
}

/// Sends a pool callback and decodes the items program's answer.
pub fn pool_call(hw: &mut Hw, mint: &Pubkey, pool: &Pubkey, slot: u8, call: &PoolCall) -> (Tx, bordrless_hook::pool_item::ItemPoolAnswer) {
    use anchor_lang::AnchorDeserialize;
    let ix = pool_call_ix(hw, mint, pool, slot, call);
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

/// A one-hop buy route (no raid).
pub fn plain_route(mint: &Pubkey, pool: &Pubkey, amount: u64) -> RouteContext {
    RouteContext::single(ids::BRIDGED_SOL_MINT, *mint, *pool, amount)
}

/// A two-hop route that sold `rival` on `rival_pool` and buys us on `pool`.
pub fn raid_route(rival: &Pubkey, rival_pool: &Pubkey, mint: &Pubkey, amount: u64) -> RouteContext {
    RouteContext {
        route_input_mint: *rival,
        route_output_mint: *mint,
        first_pool: *rival_pool,
        route_amount_in: amount,
        hop_index: 1,
        hop_count: 2,
    }
}

/// The `EquipState` of `slot` on `mint`.
pub fn equip_state(hw: &Hw, mint: &Pubkey, slot: u8) -> hookwars_items::EquipState {
    hw.w.env.read(&pda::equip_state(mint, slot).0)
}

/// Bridged SOL into `owner`'s holding (wrapped from `funder`'s SOL).
pub fn give_sol(hw: &mut Hw, funder: &Keypair, owner: &Pubkey, lamports: u64) {
    hw.w.wrap_sol(funder, lamports).ok();
    let sol = hw.w.sol;
    let ixs = [
        token::create_holding(funder.pubkey(), sol, *owner),
        token::transfer(
            funder.pubkey(),
            token::holding_address(&sol, &funder.pubkey()),
            token::holding_address(&sol, owner),
            sol,
            None,
            vec![],
            lamports,
        ),
    ];
    hw.w.env.send_paid_by(&ixs, funder, &[]).ok();
}

/// Creates `owner`'s holding of `mint` (paid by `payer`).
pub fn create_holding(hw: &mut Hw, payer: &Keypair, mint: &Pubkey, owner: &Pubkey) {
    hw.w.env
        .send_paid_by(&[token::create_holding(payer.pubkey(), *mint, *owner)], payer, &[])
        .ok();
}

/// `settle_equip` of `slot` on `mint` by `cranker`: `dests` are each module's
/// `(token destination, quote destination)`.
pub fn settle_ix(hw: &Hw, cranker: &Pubkey, mint: &Pubkey, slot: u8, dests: &[(Pubkey, Pubkey)]) -> Instruction {
    let st = equip_state(hw, mint, slot);
    let item = st.item;
    let it: hookwars_armory::state::Item = hw.w.env.read(&item);
    let sol = ids::BRIDGED_SOL_MINT;
    let state = pda::equip_state(mint, slot).0;
    let royalty = pda::royalty_owner(&item).0;
    let cuts = pda::pool_cuts(mint).0;
    let mut accounts = hookwars_items::accounts::SettleEquip {
        cranker: *cranker,
        mint: *mint,
        item,
        composite: if it.template_id == t::COMPOSITE { CompositeItem::address(&item).0 } else { ids::ITEMS_ID },
        equip_state: state,
        equip_vault: it.manifest.token_cuts().then(|| pda::holding(mint, &state)),
        pool_cuts: cuts,
        pool_cuts_holding: pda::holding(&sol, &cuts),
        quote_mint: sol,
        royalty_owner: royalty,
        royalty_token: pda::holding(mint, &royalty),
        royalty_quote: pda::holding(&sol, &royalty),
        cranker_token: pda::holding(mint, cranker),
        cranker_quote: pda::holding(&sol, cranker),
        armory_config: pda::config().0,
        token_program: bordrless_token::ID,
        token_event_authority: token::event_authority(),
        system_program: anchor_lang::system_program::ID,
    }
    .to_account_metas(None);
    for (a, b) in dests {
        accounts.push(AccountMeta::new(*a, false));
        accounts.push(AccountMeta::new(*b, false));
    }
    Instruction {
        program_id: ids::ITEMS_ID,
        accounts,
        data: hookwars_items::instruction::SettleEquip { slot }.data(),
    }
}

/// `create_composite` of `modules` by `author`; returns the transaction and the item.
pub fn create_composite(hw: &mut Hw, author: &Keypair, modules: Vec<Module>, royalty_bps: u16) -> (Tx, Pubkey) {
    let n = hw.config().items_minted;
    let item_mint = pda::item_mint(n).0;
    let item = pda::item(&item_mint).0;
    let mut accounts = hookwars_armory::accounts::CreateComposite {
        author: author.pubkey(),
        config: pda::config().0,
        template: pda::template(t::COMPOSITE).0,
        minter: hookwars_armory::cpi::MINTER,
        item_mint,
        item,
        composite: CompositeItem::address(&item).0,
        recipient_holding: token::holding_address(&item_mint, &author.pubkey()),
        token: token_accounts(),
        system_program: anchor_lang::system_program::ID,
        event_authority: armory_events(),
        program: ids::ARMORY_ID,
    }
    .to_account_metas(None);
    for m in &modules {
        accounts.push(AccountMeta::new_readonly(pda::template(m.template_id).0, false));
    }
    let ix = Instruction {
        program_id: ids::ARMORY_ID,
        accounts,
        data: hookwars_armory::instruction::CreateComposite { modules, royalty_bps }.data(),
    };
    let tx = hw.w.env.send_paid_by(&[ix], author, &[]);
    (tx, item)
}

/// A module of `template` with `params`, targets `start..start+count`.
pub fn module(template: u16, params: Params, start: u8, count: u8) -> Module {
    let m = hookwars_common::manifest(template, &params, count).expect("manifest");
    Module {
        template_id: template,
        params,
        target_start: start,
        target_count: count,
        data_bytes: m.data_bytes,
        reads_module: hookwars_common::composite::NO_READ,
    }
}

/// `armory_ix` re-exported for suites that build their own armory calls.
pub fn armory(accounts: impl ToAccountMetas, data: impl InstructionData) -> Instruction {
    armory_ix(accounts, data)
}
