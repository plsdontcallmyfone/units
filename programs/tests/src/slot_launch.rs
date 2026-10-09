// Changed by Hookwars: new file, slot launch helpers (M3b); security review 2 L-D: the stub's item
// registry is written at equip, and `create_prepared_launch` takes the forwarded slots' registries.
//! Slot launches in the LiteSVM suites (spec 03 section 4.3): `prepare_launch`, equipping (by
//! `armory_stub` signing as the mint's slot authority for test pool items, or the real armory
//! through `equip_prepared`), `create_prepared_launch`, swaps that carry the pool items' accounts,
//! and graduation with the slot mint's slices. Pool items are `pool_item_stub` (a scripted test
//! item, to be replaced by `hookwars_items`' pool callbacks at integration).

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::InstructionData;
use bordrless_launch::client as launch;
use bordrless_launch::client::slots as sl;
use bordrless_launch::instructions::{CreateLaunchArgs, PrepareLaunchArgs};
use bordrless_launch::state::LaunchRules;
use bordrless_token::client as token;
use bordrless_token::instructions::SlotInit;
use bordrless_token::slots::{is_called, SlotOp};
use bordrless_token::state::{Mint, SlotBounds};
use pool_item_stub::ItemPoolAnswerSpace;
use solana_keypair::Keypair;
use solana_signer::Signer;

use bordrless_core::policy;
use crate::env::{Env, Tx};
use crate::fixture::World;
use crate::program_bytes;
use crate::slots::{as_armory, authority_of};

/// The pool flags of an item that takes both pool callbacks.
pub const BOTH: u16 = 3;
/// `pool_before_swap` only.
pub const BEFORE: u16 = 1;
/// `pool_after_swap` only.
pub const AFTER: u16 = 2;

/// Loads `pool_item_stub`.
pub fn load_pool_item_stub(env: &mut Env) {
    env.svm
        .add_program(pool_item_stub::ID, &program_bytes("pool_item_stub"))
        .expect("load pool_item_stub");
}

/// A pool slot (Vote rule, no data). A Pool slot carries no token-side cut bound (the token
/// program refuses one; the pool-side ceiling is the armory's), so `_max_cut_bps` is ignored.
pub fn pool_slot(_max_cut_bps: u16, may_burn: bool) -> SlotInit {
    SlotInit {
        kind: bordrless_hook::slot_kind::POOL,
        equip_rule: bordrless_hook::equip_rule::VOTE,
        bounds: SlotBounds {
            max_cut_bps: 0,
            may_refuse: true,
            may_write_data: false,
            may_answer_touch: false,
            may_burn,
        },
        data_len: 0,
        locked_program: None,
        locked_flags: 0,
        locked_extra_count: 0,
    }
}

/// An answer.
pub fn ans(discount_bps: u16, cut: u64, burn: u64) -> ItemPoolAnswerSpace {
    ItemPoolAnswerSpace {
        discount_bps,
        cut,
        burn,
    }
}

/// The slices of `op` on a slot launch's mint, as a client resolves them from its table: the
/// kit's (its config and the reward vault, or the kit's id) for the kit's Locked slot, a
/// `slot_tester` item's script and equip vault for an item slot (budgets only).
pub fn slices_of(w: &World, mint: &Pubkey, op: SlotOp) -> Vec<AccountMeta> {
    let m: Mint = w.env.read(mint);
    let l: bordrless_launch::state::Launch = w.env.read(&launch::launch_address(mint));
    let mut v = Vec::new();
    for (i, s) in m.active_slots().iter().enumerate() {
        if !is_called(s, op) {
            continue;
        }
        if s.program == bordrless_kit::ID {
            v.extend(launch::base_hook_slice(mint, &w.sol, l.modules));
        } else {
            v.extend(token::slot_slice(
                s.program,
                crate::slots::item_extras(mint, i as u8, &s.item, s.flags),
            ));
        }
    }
    v
}

/// The pool items' accounts of a slot launch's swap: per forwarded pool slot (a filled Pool or
/// Relation slot with pool flags), the stub, the launchpad's signer for it and the item's
/// script.
pub fn pool_items_of(w: &World, mint: &Pubkey) -> Vec<AccountMeta> {
    let m: Mint = w.env.read(mint);
    let mut v = Vec::new();
    for s in m.active_slots() {
        if !bordrless_launch::instructions::forwards(s) {
            continue;
        }
        v.push(AccountMeta::new_readonly(s.program, false));
        v.push(AccountMeta::new_readonly(sl::item_signer(&s.program), false));
        v.push(AccountMeta::new(pool_item_stub::script_address(&s.item), false));
    }
    v
}

/// The item registries of a mint's forwarded slots, in slot order (`create_prepared_launch` and
/// `refresh_pool_registry` take them).
pub fn item_registries(w: &World, mint: &Pubkey) -> Vec<Pubkey> {
    let m: Mint = w.env.read(mint);
    m.active_slots()
        .iter()
        .filter(|s| bordrless_launch::instructions::forwards(s))
        .map(|s| {
            Pubkey::find_program_address(
                &[bordrless_hook::HOOK_ACCOUNTS_SEED, mint.as_ref(), s.item.as_ref()],
                &s.program,
            )
            .0
        })
        .collect()
}

impl World {
    /// A world with the slot test programs (`slot_tester`, `armory_stub`) and `pool_item_stub`.
    pub fn with_slot_launches() -> Self {
        let mut w = World::with_slots();
        load_pool_item_stub(&mut w.env);
        w
    }

    /// The arguments of `prepare_launch`.
    pub fn prepare_args(
        symbol: &str,
        creator_fee_bps: u16,
        rules: LaunchRules,
        slots: Vec<SlotInit>,
    ) -> PrepareLaunchArgs {
        let a = World::launch_args(symbol, creator_fee_bps, 0, rules);
        PrepareLaunchArgs {
            name: a.name,
            symbol: a.symbol,
            uri: a.uri,
            creator_fee_bps,
            rules,
            slots,
        }
    }

    /// `prepare_launch` by `creator` for `mint`.
    pub fn prepare_launch(
        &mut self,
        creator: &Keypair,
        mint: &Keypair,
        creator_fee_bps: u16,
        rules: LaunchRules,
        slots: Vec<SlotInit>,
    ) -> Tx {
        let ix = sl::prepare_launch(
            creator.pubkey(),
            mint.pubkey(),
            Self::prepare_args("SLOT", creator_fee_bps, rules, slots),
        );
        self.env.send_paid_by(&[ix], creator, &[mint])
    }

    /// The `create_prepared_launch` instruction (inline rules, the policy curve at
    /// `virtual_quote`, the mint's transfer slices as a client resolves them).
    pub fn create_prepared_ix(
        &self,
        creator: &Pubkey,
        mint: &Pubkey,
        creator_fee_bps: u16,
        virtual_quote: u64,
        rules: LaunchRules,
    ) -> Instruction {
        let args: CreateLaunchArgs =
            World::launch_args("SLOT", creator_fee_bps, virtual_quote, rules);
        sl::create_prepared_launch(
            *creator,
            *mint,
            self.env.treasury.pubkey(),
            self.sol,
            policy::LP_FEE_BPS,
            args,
            item_registries(self, mint),
            self.prepared_slices(mint),
        )
    }

    /// The transfer slices of a prepared (not yet launched) slot mint: the kit's, built from the
    /// mint's own table (the `Launch` does not exist yet).
    pub fn prepared_slices(&self, mint: &Pubkey) -> Vec<AccountMeta> {
        let m: Mint = self.env.read(mint);
        let mut v = Vec::new();
        for (i, s) in m.active_slots().iter().enumerate() {
            if !is_called(s, SlotOp::Transfer) {
                continue;
            }
            if s.program == bordrless_kit::ID {
                let p: bordrless_launch::state::PreparedLaunch =
                    self.env.read(&sl::prepared_address(mint));
                let rewards = p.rules.rewards_on();
                let vault = launch::holder_vault_address(mint, &self.sol);
                v.extend(bordrless_kit::client::hook_slice(mint, rewards.then_some(vault)));
            } else {
                v.extend(token::slot_slice(
                    s.program,
                    crate::slots::item_extras(mint, i as u8, &s.item, s.flags),
                ));
            }
        }
        v
    }

    /// A test pool item: a fresh key and its script.
    pub fn stub_item(&mut self, payer: &Keypair) -> Pubkey {
        let item = Pubkey::new_unique();
        let ix = Instruction {
            program_id: pool_item_stub::ID,
            accounts: vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(pool_item_stub::script_address(&item), false),
                AccountMeta::new_readonly(anchor_lang::system_program::ID, false),
            ],
            data: pool_item_stub::instruction::InitScript { item }.data(),
        };
        self.env.send_paid_by(&[ix], payer, &[]).ok();
        item
    }

    /// Scripts a test pool item.
    pub fn set_stub(
        &mut self,
        payer: &Keypair,
        item: &Pubkey,
        before: ItemPoolAnswerSpace,
        after: ItemPoolAnswerSpace,
        fail_before: bool,
        fail_after: bool,
    ) {
        let ix = Instruction {
            program_id: pool_item_stub::ID,
            accounts: vec![
                AccountMeta::new_readonly(payer.pubkey(), true),
                AccountMeta::new(pool_item_stub::script_address(item), false),
            ],
            data: pool_item_stub::instruction::SetScript {
                before,
                after,
                fail_before,
                fail_after,
            }
            .data(),
        };
        self.env.send_paid_by(&[ix], payer, &[]).ok();
    }

    /// A test pool item's script.
    pub fn stub_script(&self, item: &Pubkey) -> pool_item_stub::Script {
        self.env.read(&pool_item_stub::script_address(item))
    }

    /// Equips test pool item `item` in `slot` with `pool_flags` (`armory_stub` signing as the
    /// mint's slot authority; one extra: the item's script).
    pub fn equip_stub(
        &mut self,
        payer: &Keypair,
        mint: &Pubkey,
        slot: u8,
        item: &Pubkey,
        pool_flags: u16,
    ) -> Tx {
        // The stub keeps no registry; the real items program writes one at `init_equip`, so the
        // stub's (its one extra, the script) is written here.
        let (registry, _) = Pubkey::find_program_address(
            &[bordrless_hook::HOOK_ACCOUNTS_SEED, mint.as_ref(), item.as_ref()],
            &pool_item_stub::ID,
        );
        let list = bordrless_hook::HookAccountList::new(vec![bordrless_hook::ExtraAccount {
            writable: true,
            source: bordrless_hook::AccountSource::Key(pool_item_stub::script_address(item)),
        }]);
        self.env
            .svm
            .set_account(
                registry,
                solana_account::Account {
                    lamports: 10_000_000,
                    data: list.encode(),
                    owner: pool_item_stub::ID,
                    executable: false,
                    rent_epoch: 0,
                },
            )
            .unwrap();
        let ix = token::set_slot_item(
            authority_of(mint),
            *mint,
            pool_item_stub::ID,
            None,
            slot,
            *item,
            0,
            pool_flags,
            1,
        );
        self.env.send_paid_by(&[as_armory(mint, ix)], payer, &[])
    }

    /// A slot launch's swap by `trader` delivered to `recipient`, with the pool items as the
    /// mint's table names them.
    pub fn slot_swap_ix(
        &self,
        trader: &Pubkey,
        recipient: &Pubkey,
        mint: &Pubkey,
        direction: u8,
        amount_in: u64,
    ) -> Instruction {
        let mut keys = self.launch_keys(mint);
        keys.burns = true;
        sl::swap(
            &keys,
            *trader,
            *recipient,
            direction,
            amount_in,
            0,
            slices_of(self, mint, SlotOp::Transfer),
            pool_items_of(self, mint),
        )
    }

    /// Buys `lamports` of a slot launch with bridged SOL (creating the holding first).
    pub fn slot_buy(&mut self, trader: &Keypair, mint: &Pubkey, lamports: u64) -> Tx {
        let t = trader.pubkey();
        let ixs = [
            token::create_holding(t, *mint, t),
            self.slot_swap_ix(&t, &t, mint, 1, lamports),
        ];
        self.env.send_paid_by(&ixs, trader, &[])
    }

    /// Sells `amount` of a slot launch for bridged SOL.
    pub fn slot_sell(&mut self, trader: &Keypair, mint: &Pubkey, amount: u64) -> Tx {
        let t = trader.pubkey();
        let ix = self.slot_swap_ix(&t, &t, mint, 0, amount);
        self.env.send_paid_by(&[ix], trader, &[])
    }

    /// `graduate` of a slot launch with its mint's transfer and burn slices.
    pub fn slot_graduate_ix(&self, cranker: &Pubkey, mint: &Pubkey) -> Instruction {
        let keys = self.launch_keys(mint);
        sl::graduate(
            *cranker,
            keys.mint,
            keys.quote_mint,
            keys.lp_fee_bps,
            keys.modules,
            slices_of(self, mint, SlotOp::Transfer),
            slices_of(self, mint, SlotOp::Burn),
        )
    }

    /// A whole slot launch: prepare (`slots` after the kit's), one test pool item per entry of
    /// `items` (slot index, pool flags), then the launch. Answers the mint and the items.
    pub fn slot_launch(
        &mut self,
        creator: &Keypair,
        creator_fee_bps: u16,
        rules: LaunchRules,
        slots: Vec<SlotInit>,
        items: &[(u8, u16)],
    ) -> (Pubkey, Vec<Pubkey>) {
        let mint_kp = Keypair::new();
        let mint = mint_kp.pubkey();
        self.prepare_launch(creator, &mint_kp, creator_fee_bps, rules, slots)
            .ok();
        let mut keys = Vec::new();
        for (slot, flags) in items {
            let item = self.stub_item(creator);
            self.equip_stub(creator, &mint, *slot, &item, *flags).ok();
            keys.push(item);
        }
        let ix = self.create_prepared_ix(
            &creator.pubkey(),
            &mint,
            creator_fee_bps,
            crate::launch::VQ,
            rules,
        );
        self.env
            .send_paid_by(&[ix], creator, &[&mint_kp])
            .ok();
        (mint, keys)
    }
}
