// Changed by Hookwars: new file, helpers for slot mints (M1).
//! Slot mints in the LiteSVM suites. `slot_tester` is a scripted slot item; `armory_stub`,
//! declared at the armory's program id, signs as a mint's `["slots", mint]` PDA by CPI (the real
//! armory is M2). `hook_tester` stands in for a Locked legacy hook (the kit's place).

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{AnchorSerialize, InstructionData};
use bordrless_hook::{equip_owner, slot_authority, slot_flags, SlotReturn};
use bordrless_token::client as token;
use bordrless_token::constants::{ARMORY_ID, ITEMS_ID};
use bordrless_token::instructions::{CreateMintArgs, SlotInit};
use bordrless_token::slots::{is_called, SlotOp};
use bordrless_token::state::{Mint, SlotBounds};
use solana_keypair::Keypair;
use solana_signer::Signer;

use crate::env::{Env, Tx};
use crate::fixture::World;
use crate::program_bytes;

/// Loads `slot_tester` and `armory_stub` (at the armory's id) into the environment.
pub fn load_slot_programs(env: &mut Env) {
    env.svm
        .add_program(slot_tester::ID, &program_bytes("slot_tester"))
        .expect("load slot_tester");
    env.svm
        .add_program(ARMORY_ID, &program_bytes("armory_stub"))
        .expect("load armory_stub");
}

/// An item slot.
pub fn item_slot(kind: u8, rule: u8, max_cut_bps: u16, data_len: u8, touch: bool) -> SlotInit {
    SlotInit {
        kind,
        equip_rule: rule,
        bounds: SlotBounds {
            max_cut_bps,
            may_refuse: true,
            may_write_data: data_len > 0,
            may_answer_touch: touch,
        },
        data_len,
        locked_program: None,
        locked_flags: 0,
        locked_extra_count: 0,
    }
}

/// A Locked slot running `program` (legacy convention) with `flags`.
pub fn locked_slot(program: Pubkey, flags: u16, data_len: u8, extra_count: u8) -> SlotInit {
    SlotInit {
        kind: bordrless_hook::slot_kind::LOCKED,
        equip_rule: bordrless_hook::equip_rule::LOCKED,
        bounds: SlotBounds {
            max_cut_bps: 0,
            may_refuse: true,
            may_write_data: data_len > 0,
            may_answer_touch: false,
        },
        data_len,
        locked_program: Some(program),
        locked_flags: flags,
        locked_extra_count: extra_count,
    }
}

/// Encodes a slot answer.
pub fn answer_bytes(answer: &SlotReturn) -> Vec<u8> {
    let mut v = Vec::new();
    answer.serialize(&mut v).expect("encode");
    v
}

/// The mint's slot authority.
pub fn authority_of(mint: &Pubkey) -> Pubkey {
    slot_authority(&ARMORY_ID, mint).0
}

/// The owner and holding of slot `slot`'s equip vault.
pub fn equip_vault(mint: &Pubkey, slot: u8) -> (Pubkey, Pubkey) {
    let owner = equip_owner(&ITEMS_ID, mint, slot).0;
    (owner, token::holding_address(mint, &owner))
}

/// `ix` (a token instruction whose slot-authority account is the mint's `["slots", mint]`),
/// forwarded by `armory_stub` signing as that PDA.
pub fn as_armory(mint: &Pubkey, ix: Instruction) -> Instruction {
    let pda = authority_of(mint);
    let mut accounts = vec![
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(bordrless_token::ID, false),
    ];
    accounts.extend(ix.accounts.into_iter().map(|mut m| {
        if m.pubkey == pda {
            m.is_signer = false;
        }
        m
    }));
    Instruction {
        program_id: ARMORY_ID,
        accounts,
        data: armory_stub::instruction::AsSlotAuthority { data: ix.data }.data(),
    }
}

/// The extras of a `slot_tester` item `item` in a slot with `flags`: its script, then the equip
/// vault when it may cut.
pub fn item_extras(mint: &Pubkey, slot: u8, item: &Pubkey, flags: u16) -> Vec<AccountMeta> {
    let mut v = vec![AccountMeta::new(slot_tester::script_address(item), false)];
    if flags & slot_flags::TRANSFER_RETURNS_DELTA != 0 {
        v.push(AccountMeta::new(equip_vault(mint, slot).1, false));
    }
    v
}

impl World {
    /// A world with the slot test programs loaded.
    pub fn with_slots() -> Self {
        let mut w = World::new();
        load_slot_programs(&mut w.env);
        w
    }

    /// `create_slot_mint` by `owner` (every authority but the hook's), with the armory as slot
    /// authority when `with_authority`, then `owner`'s holding.
    pub fn create_slot_mint(
        &mut self,
        owner: &Keypair,
        mint: &Keypair,
        slots: Vec<SlotInit>,
        with_authority: bool,
    ) -> Tx {
        let key = mint.pubkey();
        let args = CreateMintArgs {
            decimals: 6,
            name: "Slotted".to_string(),
            symbol: "SLOT".to_string(),
            uri: String::new(),
            max_supply: 0,
            mint_authority: Some(owner.pubkey()),
            freeze_authority: Some(owner.pubkey()),
            hook_program: None,
            hook_flags: 0,
            hook_authority: None,
            metadata_authority: Some(owner.pubkey()),
        };
        let authority = with_authority.then(|| authority_of(&key));
        let ixs = [
            token::create_slot_mint(owner.pubkey(), key, args, authority, slots),
            token::create_holding(owner.pubkey(), key, owner.pubkey()),
        ];
        self.env.send_paid_by(&ixs, owner, &[mint])
    }

    /// Creates `slot_tester`'s script for `item`.
    pub fn init_item(&mut self, payer: &Keypair, item: &Pubkey) {
        self.env
            .send_paid_by(
                &[Instruction {
                    program_id: slot_tester::ID,
                    accounts: vec![
                        AccountMeta::new(payer.pubkey(), true),
                        AccountMeta::new(slot_tester::script_address(item), false),
                        AccountMeta::new_readonly(anchor_lang::system_program::ID, false),
                    ],
                    data: slot_tester::instruction::InitScript { item: *item }.data(),
                }],
                payer,
                &[],
            )
            .ok();
    }

    /// Scripts what `item`'s callback `which` does.
    pub fn item_answer(&mut self, payer: &Keypair, item: &Pubkey, which: u8, mode: u8, data: Vec<u8>) {
        self.env
            .send_paid_by(
                &[Instruction {
                    program_id: slot_tester::ID,
                    accounts: vec![
                        AccountMeta::new_readonly(payer.pubkey(), true),
                        AccountMeta::new(slot_tester::script_address(item), false),
                    ],
                    data: slot_tester::instruction::SetAnswer {
                        which,
                        action: mode,
                        data,
                    }
                    .data(),
                }],
                payer,
                &[],
            )
            .ok();
    }

    /// `item`'s script.
    pub fn item_script(&self, item: &Pubkey) -> slot_tester::Script {
        self.env.read(&slot_tester::script_address(item))
    }

    /// Creates the equip vault holding of `slot`.
    pub fn make_equip_vault(&mut self, payer: &Keypair, mint: &Pubkey, slot: u8) -> Pubkey {
        let (owner, holding) = equip_vault(mint, slot);
        self.env
            .send_paid_by(&[token::create_holding(payer.pubkey(), *mint, owner)], payer, &[])
            .ok();
        holding
    }

    /// The armory (stub) equips `slot_tester` item `item` in `slot` with `flags`; its extras are
    /// its script and, when it may cut, the slot's equip vault.
    pub fn equip(&mut self, payer: &Keypair, mint: &Pubkey, slot: u8, item: &Pubkey, flags: u16) -> Tx {
        let cuts = flags & slot_flags::TRANSFER_RETURNS_DELTA != 0;
        let extra_count = item_extras(mint, slot, item, flags).len() as u8;
        let ix = token::set_slot_item(
            authority_of(mint),
            *mint,
            slot_tester::ID,
            cuts.then(|| equip_vault(mint, slot).1),
            slot,
            *item,
            flags,
            0,
            extra_count,
        );
        self.env.send_paid_by(&[as_armory(mint, ix)], payer, &[])
    }

    /// The remaining accounts of `op` on `mint`: one slice per called slot, from the mint's
    /// table. Item slots are `slot_tester` items; the Locked slot is `hook_tester` keyed by the
    /// mint (its script is its one extra).
    pub fn slot_extras(&self, mint: &Pubkey, op: SlotOp) -> Vec<AccountMeta> {
        let m: Mint = self.env.read(mint);
        let mut v = Vec::new();
        for (i, s) in m.active_slots().iter().enumerate() {
            if !is_called(s, op) {
                continue;
            }
            let extras = if s.is_locked() {
                vec![AccountMeta::new(hook_tester::client::script_address(mint), false)]
            } else {
                item_extras(mint, i as u8, &s.item, s.flags)
            };
            v.extend(token::slot_slice(s.program, extras));
        }
        v
    }

    /// `transfer` of `amount` of a slot mint from `from` to `to`'s holding.
    pub fn slot_transfer_ix(&self, from: &Pubkey, mint: &Pubkey, to: &Pubkey, amount: u64) -> Instruction {
        token::transfer_with(
            *from,
            token::holding_address(mint, from),
            token::holding_address(mint, to),
            *mint,
            None,
            self.slot_extras(mint, SlotOp::Transfer),
            amount,
        )
    }

    /// `mint_to` of a slot mint by its mint authority `owner` into `to`'s holding.
    pub fn slot_mint_ix(&self, owner: &Pubkey, mint: &Pubkey, to: &Pubkey, amount: u64) -> Instruction {
        token::mint_to_with(
            *owner,
            *mint,
            token::holding_address(mint, to),
            None,
            self.slot_extras(mint, SlotOp::Mint),
            amount,
        )
    }

    /// `burn` of a slot mint from `from`'s holding.
    pub fn slot_burn_ix(&self, from: &Pubkey, mint: &Pubkey, amount: u64) -> Instruction {
        token::burn_with(
            *from,
            token::holding_address(mint, from),
            *mint,
            None,
            self.slot_extras(mint, SlotOp::Burn),
            amount,
        )
    }
}
