// Changed by Hookwars: new file (gating, docs/spec/18-gating.md part C): the external gate harness.
//! Hand-built Token-2022 instructions (a mint with the `TransferHook` extension, accounts, mint
//! and `TransferChecked` with the hook's extra accounts resolved the way the interface does) and
//! the gate's instructions, on top of the economy world.

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{InstructionData, ToAccountMetas};
use hookwars_common::gate::{self as g, GATE_ID};
use hookwars_common::t22::TOKEN_2022_ID;
use hookwars_common::{ids, pda};
use hookwars_gate::{Binding, MintGate};
use solana_keypair::Keypair;
use solana_signer::Signer;

use crate::economy::{Ew, SOL};
use crate::{program_bytes, Tx};

/// A mint with the `TransferHook` extension: base, padding, account type, one TLV (4 + 64).
pub const HOOKED_MINT_LEN: usize = 165 + 1 + 4 + 64;
/// A token account of such a mint: base, account type, `TransferHookAccount` (4 + 1).
pub const HOOKED_ACCOUNT_LEN: usize = 165 + 1 + 4 + 1;
/// A plain Token-2022 mint.
pub const PLAIN_MINT_LEN: usize = 82;
/// A plain Token-2022 account.
pub const PLAIN_ACCOUNT_LEN: usize = 165;
pub const DECIMALS: u8 = 6;

fn system_create(payer: &Pubkey, key: &Pubkey, lamports: u64, len: usize, owner: &Pubkey) -> Instruction {
    anchor_lang::solana_program::system_instruction::create_account(payer, key, lamports, len as u64, owner)
}

/// `TransferHookInstruction::Initialize` (extension 36, sub 0).
pub fn init_transfer_hook_ix(mint: &Pubkey, authority: &Pubkey, program: &Pubkey) -> Instruction {
    let mut data = vec![36, 0];
    data.extend_from_slice(authority.as_ref());
    data.extend_from_slice(program.as_ref());
    Instruction { program_id: TOKEN_2022_ID, accounts: vec![AccountMeta::new(*mint, false)], data }
}

/// `InitializeMint2` (20) with no freeze authority.
pub fn init_mint2_ix(mint: &Pubkey, authority: &Pubkey, decimals: u8) -> Instruction {
    let mut data = vec![20, decimals];
    data.extend_from_slice(authority.as_ref());
    data.push(0);
    Instruction { program_id: TOKEN_2022_ID, accounts: vec![AccountMeta::new(*mint, false)], data }
}

/// `InitializeAccount3` (18).
pub fn init_account3_ix(account: &Pubkey, mint: &Pubkey, owner: &Pubkey) -> Instruction {
    let mut data = vec![18];
    data.extend_from_slice(owner.as_ref());
    Instruction {
        program_id: TOKEN_2022_ID,
        accounts: vec![AccountMeta::new(*account, false), AccountMeta::new_readonly(*mint, false)],
        data,
    }
}

/// `MintTo` (7).
pub fn mint_to_ix(mint: &Pubkey, account: &Pubkey, authority: &Pubkey, amount: u64) -> Instruction {
    let mut data = vec![7];
    data.extend_from_slice(&amount.to_le_bytes());
    Instruction {
        program_id: TOKEN_2022_ID,
        accounts: vec![AccountMeta::new(*mint, false), AccountMeta::new(*account, false), AccountMeta::new_readonly(*authority, true)],
        data,
    }
}

/// `TransferChecked` (12), with `extra` appended (the hook's resolved accounts).
pub fn transfer_checked_ix(source: &Pubkey, mint: &Pubkey, destination: &Pubkey, owner: &Pubkey, amount: u64, extra: Vec<AccountMeta>) -> Instruction {
    let mut data = vec![12];
    data.extend_from_slice(&amount.to_le_bytes());
    data.push(DECIMALS);
    let mut accounts = vec![
        AccountMeta::new(*source, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new(*destination, false),
        AccountMeta::new_readonly(*owner, true),
    ];
    accounts.extend(extra);
    Instruction { program_id: TOKEN_2022_ID, accounts, data }
}

/// The gate's instruction.
pub fn gate_ix(accounts: impl ToAccountMetas, data: impl InstructionData) -> Instruction {
    Instruction { program_id: GATE_ID, accounts: accounts.to_account_metas(None), data: data.data() }
}

/// The gate world: the economy world, the gate loaded and its config opened.
pub struct Gw {
    pub ew: Ew,
}

impl Default for Gw {
    fn default() -> Self {
        Self::new()
    }
}

impl Gw {
    pub fn new() -> Self {
        let mut ew = Ew::new();
        ew.hw.w.env.svm.add_program(GATE_ID, &program_bytes("hookwars_gate")).expect("load hookwars_gate");
        let d = ew.deployer();
        ew.hw.w.env.set_upgrade_authority(GATE_ID, Some(d.pubkey()));
        let ix = gate_ix(
            hookwars_gate::accounts::InitConfig {
                authority: d.pubkey(),
                config: config(),
                program_data: hookwars_common::programdata_address(&GATE_ID),
                system_program: anchor_lang::system_program::ID,
            },
            hookwars_gate::instruction::InitConfig { admin: d.pubkey(), require_holder_state_default: true },
        );
        ew.hw.w.env.send(&[ix], &[&d]).ok();
        // The templates the suites bind (TEST schemas).
        for id in [17u16, 19, 22, 26] {
            let data = hookwars_armory::instruction::RegisterTemplate { args: crate::armory::Hw::template_args(id) }.data();
            ew.hw.queue(&data, &[ids::ITEMS_ID]);
        }
        for id in [17u16, 19, 22, 26] {
            ew.hw.register(id).ok();
        }
        Self { ew }
    }

    pub fn send(&mut self, payer: &Keypair, ixs: &[Instruction]) -> Tx {
        self.ew.send(payer, ixs)
    }

    pub fn funded(&mut self) -> Keypair {
        self.ew.funded(10 * SOL)
    }

    /// A Token-2022 mint whose transfer hook is `hook` (authority `authority`), minted by
    /// `authority`.
    pub fn hooked_mint(&mut self, authority: &Keypair, hook: &Pubkey) -> Pubkey {
        let mint = Keypair::new();
        let rent = self.ew.hw.w.env.rent(HOOKED_MINT_LEN);
        let ixs = [
            system_create(&authority.pubkey(), &mint.pubkey(), rent, HOOKED_MINT_LEN, &TOKEN_2022_ID),
            init_transfer_hook_ix(&mint.pubkey(), &authority.pubkey(), hook),
            init_mint2_ix(&mint.pubkey(), &authority.pubkey(), DECIMALS),
        ];
        self.ew.hw.w.env.send_paid_by(&ixs, authority, &[&mint]).ok();
        mint.pubkey()
    }

    /// A plain Token-2022 mint (no hook).
    pub fn plain_mint(&mut self, authority: &Keypair) -> Pubkey {
        let mint = Keypair::new();
        let rent = self.ew.hw.w.env.rent(PLAIN_MINT_LEN);
        let ixs = [
            system_create(&authority.pubkey(), &mint.pubkey(), rent, PLAIN_MINT_LEN, &TOKEN_2022_ID),
            init_mint2_ix(&mint.pubkey(), &authority.pubkey(), DECIMALS),
        ];
        self.ew.hw.w.env.send_paid_by(&ixs, authority, &[&mint]).ok();
        mint.pubkey()
    }

    /// A token account of `mint` for `owner`, with `amount` minted into it by `mint_authority`.
    pub fn account(&mut self, mint: &Pubkey, owner: &Pubkey, mint_authority: &Keypair, amount: u64, hooked: bool) -> Pubkey {
        let acct = Keypair::new();
        let len = if hooked { HOOKED_ACCOUNT_LEN } else { PLAIN_ACCOUNT_LEN };
        let rent = self.ew.hw.w.env.rent(len);
        let mut ixs = vec![
            system_create(&mint_authority.pubkey(), &acct.pubkey(), rent, len, &TOKEN_2022_ID),
            init_account3_ix(&acct.pubkey(), mint, owner),
        ];
        if amount > 0 {
            ixs.push(mint_to_ix(mint, &acct.pubkey(), &mint_authority.pubkey(), amount));
        }
        self.ew.hw.w.env.send_paid_by(&ixs, mint_authority, &[&acct]).ok();
        acct.pubkey()
    }

    /// `amount` of a Token-2022 account.
    pub fn balance(&self, account: &Pubkey) -> u64 {
        self.ew.hw.w.env.account(account).map_or(0, |a| u64::from_le_bytes(a.data[64..72].try_into().unwrap()))
    }

    pub fn register_ix(&self, payer: &Pubkey, authority: &Pubkey, mint: &Pubkey, venue: Pubkey, strict: Option<bool>) -> Instruction {
        gate_ix(
            hookwars_gate::accounts::RegisterMint {
                payer: *payer,
                authority: *authority,
                config: config(),
                mint: *mint,
                mint_gate: g::mint_gate(mint).0,
                extra_metas: g::extra_metas(mint).0,
                system_program: anchor_lang::system_program::ID,
            },
            hookwars_gate::instruction::RegisterMint { venue, strict },
        )
    }

    pub fn register(&mut self, authority: &Keypair, mint: &Pubkey, venue: Pubkey, strict: Option<bool>) -> Tx {
        let ix = self.register_ix(&authority.pubkey(), &authority.pubkey(), mint, venue, strict);
        self.send(authority, &[ix])
    }

    #[allow(clippy::too_many_arguments)]
    pub fn bind_ix(&self, authority: &Pubkey, mint: &Pubkey, slot: u8, kind: u8, item: &Pubkey, proof: &Pubkey, targets: Vec<Pubkey>, extras: &[Pubkey]) -> Instruction {
        let mut ix = gate_ix(
            hookwars_gate::accounts::Bind {
                payer: *authority,
                authority: *authority,
                config: config(),
                mint_gate: g::mint_gate(mint).0,
                extra_metas: g::extra_metas(mint).0,
                item: *item,
                proof: *proof,
                system_program: anchor_lang::system_program::ID,
            },
            hookwars_gate::instruction::Bind { slot, binding_kind: kind, targets, role: 0 },
        );
        ix.accounts.extend(extras.iter().map(|k| AccountMeta::new_readonly(*k, false)));
        ix
    }

    pub fn unbind_ix(&self, signer: &Pubkey, mint: &Pubkey, slot: u8, proof: &Pubkey) -> Instruction {
        gate_ix(
            hookwars_gate::accounts::Unbind {
                signer: *signer,
                mint_gate: g::mint_gate(mint).0,
                extra_metas: g::extra_metas(mint).0,
                proof: *proof,
                system_program: anchor_lang::system_program::ID,
            },
            hookwars_gate::instruction::Unbind { slot },
        )
    }

    pub fn open_holder_ix(&self, payer: &Pubkey, mint: &Pubkey, owner: &Pubkey) -> Instruction {
        gate_ix(
            hookwars_gate::accounts::OpenHolder {
                payer: *payer,
                mint_gate: g::mint_gate(mint).0,
                holder: g::holder(mint, owner).0,
                system_program: anchor_lang::system_program::ID,
            },
            hookwars_gate::instruction::OpenHolder { owner: *owner },
        )
    }

    pub fn open_holder(&mut self, payer: &Keypair, mint: &Pubkey, owner: &Pubkey) -> Tx {
        let ix = self.open_holder_ix(&payer.pubkey(), mint, owner);
        self.send(payer, &[ix])
    }

    pub fn mint_gate(&self, mint: &Pubkey) -> MintGate {
        self.ew.hw.w.env.read(&g::mint_gate(mint).0)
    }

    /// The hook's extra accounts for a transfer, resolved as the interface does: the validation
    /// account, the gate program, the base metas, then each binding's.
    pub fn hook_accounts(&self, mint: &Pubkey, source: &Pubkey, destination: &Pubkey) -> Vec<AccountMeta> {
        let owner_of = |k: &Pubkey| {
            let a = self.ew.hw.w.env.account(k).expect("token account");
            Pubkey::try_from(&a.data[32..64]).unwrap()
        };
        let mut v = vec![
            AccountMeta::new_readonly(g::mint_gate(mint).0, false),
            AccountMeta::new_readonly(ids::ITEMS_ID, false),
            AccountMeta::new_readonly(g::GATE_ITEMS_SIGNER, false),
            AccountMeta::new(g::holder(mint, &owner_of(source)).0, false),
            AccountMeta::new(g::holder(mint, &owner_of(destination)).0, false),
        ];
        let gate: MintGate = self.mint_gate(mint);
        for b in &gate.bindings {
            v.push(AccountMeta::new_readonly(b.item, false));
            v.push(AccountMeta::new_readonly(b.proof, false));
            v.extend(b.extras.iter().map(|k| AccountMeta::new_readonly(*k, false)));
        }
        v.push(AccountMeta::new_readonly(GATE_ID, false));
        v.push(AccountMeta::new_readonly(g::extra_metas(mint).0, false));
        v
    }

    /// A Token-2022 transfer of a gated mint with the hook's accounts.
    pub fn transfer(&mut self, owner: &Keypair, mint: &Pubkey, source: &Pubkey, destination: &Pubkey, amount: u64) -> Tx {
        let extra = self.hook_accounts(mint, source, destination);
        let ix = transfer_checked_ix(source, mint, destination, &owner.pubkey(), amount, extra);
        self.send(owner, &[ix])
    }

    /// The gate vault's holding of an item, filled from `from` (the item's holder).
    pub fn deposit_item(&mut self, from: &Keypair, mint: &Pubkey, item_mint: &Pubkey) -> Pubkey {
        let vault = g::vault(mint).0;
        let h = from.insecure_clone();
        self.ew.hw.give_item(&h, item_mint, &vault);
        bordrless_token::client::holding_address(item_mint, &vault)
    }

    /// An item of `template` with `p`, held by a fresh author.
    pub fn item(&mut self, template: u16, p: hookwars_common::Params) -> (Keypair, Pubkey, Pubkey) {
        self.ew.hw.item(template, p, 0)
    }
}

/// `["gate-config"]`.
pub fn config() -> Pubkey {
    Pubkey::find_program_address(&[g::seeds::CONFIG], &GATE_ID).0
}

/// A gate error code.
pub fn gate_code(e: hookwars_gate::GateError) -> u32 {
    anchor_lang::error::ERROR_CODE_OFFSET + e as u32
}

/// The binding in `slot`, if any.
pub fn binding(gate: &MintGate, slot: u8) -> Option<&Binding> {
    gate.bindings.iter().find(|b| b.slot == slot)
}

/// The item's PDA from its mint.
pub fn item_of(item_mint: &Pubkey) -> Pubkey {
    pda::item(item_mint).0
}
