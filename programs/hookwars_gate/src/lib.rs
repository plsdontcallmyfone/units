// Changed by Hookwars: new file (gating, docs/spec/18-gating.md part C): the external gate.
//! `hookwars_gate`: the SPL Token-2022 transfer-hook interface for units items.
//!
//! Any Token-2022 mint, from any protocol, whose transfer hook program is this program registers
//! here (signed by its transfer-hook authority) and binds up to [`MAX_BINDINGS`] units items. A
//! binding is live while the mint holds a live licence for the item (the market's `License` at
//! `["license", item, mint]`) or while the gate's vault for the mint holds the item. On every
//! transfer Token-2022 calls [`hookwars_gate::execute`]; for each live binding the gate calls the
//! items program's restricted `gate_before`, which runs the item's token-side modules: refusals,
//! holder stamps (kept here in a `HolderState` per wallet, since Token-2022 accounts carry no hook
//! data) and price reads. It never takes a cut: items that may cut are refused at bind. A binding
//! whose proof lapsed is inert (it allows every transfer).

#![allow(unexpected_cfgs)]
#![allow(clippy::result_large_err)]

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::{get_return_data, invoke, invoke_signed};
use anchor_lang::solana_program::system_instruction;
use anchor_lang::InstructionData;
use bordrless_hook::{Phase, SlotReturn, TokenSlotArgs, TokenSlotOp};
use hookwars_common::gate::{self as g, seeds};
use hookwars_common::{access as acc, ids, t22, template_id, token_flags};

declare_id!("CKMGqUdYzVyTr4myZsXo8rEMeBA5QoqdABinbZfvh8Dj");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "units gate",
    project_url: "https://github.com/plsdontcallmyfone/units",
    contacts: "link:https://github.com/plsdontcallmyfone/units/security/advisories/new",
    policy: "https://github.com/plsdontcallmyfone/units/blob/main/SECURITY.md",
    source_code: "https://github.com/plsdontcallmyfone/units"
}

/// Items one mint may bind.
pub const MAX_BINDINGS: usize = 4;
/// Targets one binding may name.
pub const MAX_TARGETS: usize = 4;
/// Extra accounts one binding may carry (the composite's module list, module extras, `Wear`).
pub const MAX_BINDING_EXTRAS: usize = 12;
/// Extra accounts `Execute` may resolve for one mint (a transfer must still fit a transaction).
pub const MAX_EXTRA_METAS: usize = 30;
/// Bytes of holder memory per wallet.
pub const HOLDER_BYTES: usize = 64;
/// The accounts `Execute` always resolves before the bindings' own: `MintGate`, the items
/// program, the gate's items signer, the source and the destination wallets' `HolderState`.
pub const BASE_METAS: usize = 5;
/// `sha256("spl-transfer-hook-interface:execute")[..8]`: the interface's `Execute`, and the TLV
/// type of the extra-account-metas list.
pub const EXECUTE_DISC: [u8; 8] = [105, 37, 101, 197, 75, 251, 102, 26];
const VERSION: u8 = 1;

/// Binding kinds.
pub mod kind {
    /// The proof is the market's `License` for the item and the mint.
    pub const LICENCE: u8 = 0;
    /// The proof is the gate vault's holding of the item mint.
    pub const OWNED: u8 = 1;
}

fn now() -> Result<i64> {
    Ok(Clock::get()?.unix_timestamp)
}

#[cfg(not(feature = "no-entrypoint"))]
fn upgrade_authority(program_data: &AccountInfo) -> Result<Option<Pubkey>> {
    require_keys_eq!(*program_data.key, hookwars_common::programdata_address(&crate::ID), GateError::NotUpgradeAuthority);
    require_keys_eq!(*program_data.owner, ids::BPF_LOADER_UPGRADEABLE_ID, GateError::NotUpgradeAuthority);
    let data = program_data.try_borrow_data()?;
    require!(data.len() >= 45 && data[..4] == [3, 0, 0, 0], GateError::NotUpgradeAuthority);
    if data[12] == 0 {
        return Ok(None);
    }
    Ok(Some(Pubkey::new_from_array(data[13..45].try_into().unwrap())))
}

#[cfg(feature = "no-entrypoint")]
fn upgrade_authority(_program_data: &AccountInfo) -> Result<Option<Pubkey>> {
    err!(GateError::NotUpgradeAuthority)
}

// ---------------------------------------------------------------------------------------------
// State.

#[account]
#[derive(InitSpace, Debug)]
pub struct GateConfig {
    pub version: u8,
    pub bump: u8,
    pub admin: Pubkey,
    pub pending_admin: Option<Pubkey>,
    /// Stops new registrations and binds (never blocks transfers).
    pub paused: bool,
    /// The default `strict` for new registrations.
    pub require_holder_state_default: bool,
    pub mints: u64,
    pub reserved: [u8; 32],
}

/// One bound item.
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Debug, Default, PartialEq, Eq)]
pub struct Binding {
    pub slot: u8,
    pub item: Pubkey,
    pub item_mint: Pubkey,
    pub template_id: u16,
    pub kind: u8,
    /// The `License` (licence kind) or the vault's holding of the item mint (owned kind).
    pub proof: Pubkey,
    pub role: u8,
    #[max_len(4)]
    pub targets: Vec<Pubkey>,
    /// The binding's range of holder memory.
    pub data_offset: u8,
    pub data_bytes: u8,
    /// Holder bytes written under another generation read as zeros.
    pub generation: u32,
    /// The items program's further accounts: the composite's module list, module extras, `Wear`.
    #[max_len(12)]
    pub extras: Vec<Pubkey>,
    pub bound_at: i64,
}

/// `["gate-mint", mint]`. The first fields' offsets are fixed (`hookwars_common::gate`): the items
/// program reads `venue` from them.
#[account]
#[derive(InitSpace, Debug)]
pub struct MintGate {
    pub version: u8,
    pub bump: u8,
    pub mint: Pubkey,
    /// Binds, unbinds, sets the venue (the mint's transfer-hook authority at registration).
    pub authority: Pubkey,
    /// The AMM pool authority whose transfers count as buys and sells (default = none).
    pub venue: Pubkey,
    /// Refuse transfers to or from wallets without a `HolderState` while a binding keeps memory.
    pub strict: bool,
    /// Counts binds; each binding keeps the value it was bound under.
    pub generation: u32,
    #[max_len(4)]
    pub bindings: Vec<Binding>,
    pub registered_at: i64,
    pub reserved: [u8; 32],
}

/// `["gate-holder", mint, owner]`: one wallet's holder memory for one mint.
#[account]
#[derive(InitSpace, Debug)]
pub struct HolderState {
    pub version: u8,
    pub bump: u8,
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub data: [u8; 64],
    /// Per slot, the binding generation the slot's range was last written under.
    pub generations: [u32; 4],
}

// ---------------------------------------------------------------------------------------------
// Errors and events.

#[error_code]
pub enum GateError {
    #[msg("the signer is not this program's upgrade authority")]
    NotUpgradeAuthority,
    #[msg("only the gate admin may do this")]
    NotAdmin,
    #[msg("the gate is paused for new registrations and binds")]
    Paused,
    #[msg("the mint is not a Token-2022 mint whose transfer hook is this program")]
    NotHooked,
    #[msg("the signer is not the mint's transfer-hook authority")]
    NotHookAuthority,
    #[msg("only the mint's gate authority may do this")]
    NotGateAuthority,
    #[msg("a wrong account")]
    WrongAccount,
    #[msg("the slot is taken or out of range")]
    BadSlot,
    #[msg("this item cannot run on an external token")]
    ItemNotGateable,
    #[msg("the proof is not live")]
    ProofNotLive,
    #[msg("the binding's holder memory does not fit")]
    NoRoom,
    #[msg("too many accounts for one transfer")]
    TooManyAccounts,
    #[msg("the transfer is not a Token-2022 transfer of this mint")]
    NotTransferring,
    #[msg("a wallet in this transfer has no holder state and the mint is strict")]
    HolderStateMissing,
    #[msg("the binding is still live")]
    StillLive,
    #[msg("the item is bound")]
    ItemBound,
    #[msg("arithmetic overflow")]
    Overflow,
}

#[event]
pub struct MintRegistered {
    pub mint: Pubkey,
    pub authority: Pubkey,
    pub venue: Pubkey,
    pub strict: bool,
    pub ts: i64,
}

#[event]
pub struct VenueSet {
    pub mint: Pubkey,
    pub venue: Pubkey,
    pub strict: bool,
}

#[event]
pub struct GateAuthoritySet {
    pub mint: Pubkey,
    pub authority: Pubkey,
}

#[event]
pub struct ItemBound {
    pub mint: Pubkey,
    pub slot: u8,
    pub item: Pubkey,
    pub template_id: u16,
    pub kind: u8,
    pub proof: Pubkey,
    pub generation: u32,
    pub ts: i64,
}

#[event]
pub struct ItemUnbound {
    pub mint: Pubkey,
    pub slot: u8,
    pub item: Pubkey,
    pub lapsed: bool,
    pub ts: i64,
}

#[event]
pub struct ItemWithdrawn {
    pub mint: Pubkey,
    pub item_mint: Pubkey,
    pub to: Pubkey,
}

#[event]
pub struct GateConfigSet {
    pub admin: Pubkey,
    pub paused: bool,
    pub require_holder_state_default: bool,
}

// ---------------------------------------------------------------------------------------------
// The extra-account-metas list (SPL transfer-hook interface).

/// One `ExtraAccountMeta` (35 bytes): discriminator, address config, signer, writable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Meta(pub [u8; 35]);

impl Meta {
    /// A fixed address.
    pub fn literal(key: &Pubkey, writable: bool) -> Self {
        let mut m = [0u8; 35];
        m[1..33].copy_from_slice(key.as_ref());
        m[34] = u8::from(writable);
        Self(m)
    }

    /// A PDA of this program from packed seeds (at most 32 bytes).
    fn pda(packed: &[u8], writable: bool) -> Self {
        let mut m = [0u8; 35];
        m[0] = 1;
        m[1..1 + packed.len()].copy_from_slice(packed);
        m[34] = u8::from(writable);
        Self(m)
    }

    /// `["gate-mint", <mint>]`.
    pub fn mint_gate() -> Self {
        let mut p = vec![1, seeds::MINT_GATE.len() as u8];
        p.extend_from_slice(seeds::MINT_GATE);
        p.extend_from_slice(&[3, 1]);
        Self::pda(&p, false)
    }

    /// `["gate-holder", <mint>, <owner of the token account at index>]`.
    pub fn holder(token_account_index: u8) -> Self {
        let mut p = vec![1, seeds::HOLDER.len() as u8];
        p.extend_from_slice(seeds::HOLDER);
        p.extend_from_slice(&[3, 1, 4, token_account_index, 32, 32]);
        Self::pda(&p, true)
    }
}

/// The metas `Execute` resolves for a mint with these bindings, in order.
pub fn metas_of(bindings: &[Binding]) -> Vec<Meta> {
    let mut v = vec![
        Meta::mint_gate(),
        Meta::literal(&ids::ITEMS_ID, false),
        Meta::literal(&g::GATE_ITEMS_SIGNER, false),
        Meta::holder(0),
        Meta::holder(2),
    ];
    for b in bindings {
        v.push(Meta::literal(&b.item, false));
        v.push(Meta::literal(&b.proof, false));
        v.extend(b.extras.iter().map(|k| Meta::literal(k, false)));
    }
    v
}

/// The TLV bytes of the extra-account-metas account.
pub fn metas_bytes(metas: &[Meta]) -> Vec<u8> {
    let mut d = EXECUTE_DISC.to_vec();
    let len = 4 + 35 * metas.len();
    d.extend_from_slice(&(len as u32).to_le_bytes());
    d.extend_from_slice(&(metas.len() as u32).to_le_bytes());
    for m in metas {
        d.extend_from_slice(&m.0);
    }
    d
}

/// Writes the list, resizing the account (the payer tops up rent).
fn write_metas<'info>(
    info: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    system: &AccountInfo<'info>,
    bindings: &[Binding],
) -> Result<()> {
    let metas = metas_of(bindings);
    require!(metas.len() <= MAX_EXTRA_METAS, GateError::TooManyAccounts);
    let bytes = metas_bytes(&metas);
    let need = Rent::get()?.minimum_balance(bytes.len());
    let have = info.lamports();
    if need > have {
        invoke(
            &system_instruction::transfer(payer.key, info.key, need - have),
            &[payer.clone(), info.clone(), system.clone()],
        )?;
    }
    info.resize(bytes.len())?;
    info.try_borrow_mut_data()?.copy_from_slice(&bytes);
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Proofs.

fn proof_live(b: &Binding, mint: &Pubkey, proof: &AccountInfo, ts: i64) -> bool {
    if *proof.key != b.proof {
        return false;
    }
    match b.kind {
        kind::LICENCE => acc::read_license(proof, &b.item, mint).is_some_and(|l| l.live_at(ts)),
        kind::OWNED => bordrless_token::client::read_holding(proof)
            .map(|h| h.mint == b.item_mint && h.owner == g::vault(mint).0 && h.amount == 1)
            .unwrap_or(false),
        _ => false,
    }
}

fn expected_proof(k: u8, item: &Pubkey, item_mint: &Pubkey, mint: &Pubkey) -> Result<Pubkey> {
    match k {
        kind::LICENCE => Ok(acc::license_address(item, mint)),
        kind::OWNED => Ok(bordrless_token::client::holding_address(item_mint, &g::vault(mint).0)),
        _ => err!(GateError::WrongAccount),
    }
}

/// First-fit range of `len` bytes among the other bindings' ranges.
fn allocate(bindings: &[Binding], len: u8) -> Option<u8> {
    if len == 0 {
        return Some(0);
    }
    let mut used: Vec<(u16, u16)> = bindings
        .iter()
        .filter(|b| b.data_bytes > 0)
        .map(|b| (u16::from(b.data_offset), u16::from(b.data_offset) + u16::from(b.data_bytes)))
        .collect();
    used.sort();
    let mut at: u16 = 0;
    for (s, e) in used {
        if at + u16::from(len) <= s {
            break;
        }
        at = at.max(e);
    }
    (at + u16::from(len) <= HOLDER_BYTES as u16).then_some(at as u8)
}

// ---------------------------------------------------------------------------------------------
// The program.

#[program]
pub mod hookwars_gate {
    use super::*;

    /// The upgrade authority opens the config once.
    pub fn init_config(ctx: Context<InitConfig>, admin: Pubkey, require_holder_state_default: bool) -> Result<()> {
        let up = upgrade_authority(&ctx.accounts.program_data)?;
        require!(up == Some(ctx.accounts.authority.key()), GateError::NotUpgradeAuthority);
        let c = &mut ctx.accounts.config;
        c.version = VERSION;
        c.bump = ctx.bumps.config;
        c.admin = admin;
        c.pending_admin = None;
        c.paused = false;
        c.require_holder_state_default = require_holder_state_default;
        c.mints = 0;
        c.reserved = [0; 32];
        emit!(GateConfigSet { admin, paused: false, require_holder_state_default });
        Ok(())
    }

    /// The admin pauses or resumes registrations and binds, and sets the default `strict`.
    /// Transfers are never blocked by the pause.
    pub fn set_config(ctx: Context<AdminOnly>, paused: bool, require_holder_state_default: bool) -> Result<()> {
        let c = &mut ctx.accounts.config;
        c.paused = paused;
        c.require_holder_state_default = require_holder_state_default;
        emit!(GateConfigSet { admin: c.admin, paused, require_holder_state_default });
        Ok(())
    }

    /// The admin proposes a successor; it accepts.
    pub fn propose_admin(ctx: Context<AdminOnly>, admin: Pubkey) -> Result<()> {
        ctx.accounts.config.pending_admin = Some(admin);
        Ok(())
    }

    pub fn accept_admin(ctx: Context<AcceptAdmin>) -> Result<()> {
        let c = &mut ctx.accounts.config;
        require!(c.pending_admin == Some(ctx.accounts.new_admin.key()), GateError::NotAdmin);
        c.admin = ctx.accounts.new_admin.key();
        c.pending_admin = None;
        emit!(GateConfigSet { admin: c.admin, paused: c.paused, require_holder_state_default: c.require_holder_state_default });
        Ok(())
    }

    /// A Token-2022 mint whose transfer hook is this program registers, signed by its
    /// transfer-hook authority. `strict = None` takes the config's default.
    pub fn register_mint(ctx: Context<RegisterMint>, venue: Pubkey, strict: Option<bool>) -> Result<()> {
        require!(!ctx.accounts.config.paused, GateError::Paused);
        let (authority, program) = t22::transfer_hook(&ctx.accounts.mint).ok_or(GateError::NotHooked)?;
        require!(program == Some(crate::ID), GateError::NotHooked);
        require!(authority == Some(ctx.accounts.authority.key()), GateError::NotHookAuthority);
        let mint = ctx.accounts.mint.key();
        let (metas_key, metas_bump) = g::extra_metas(&mint);
        require_keys_eq!(ctx.accounts.extra_metas.key(), metas_key, GateError::WrongAccount);
        let ts = now()?;
        let strict = strict.unwrap_or(ctx.accounts.config.require_holder_state_default);
        let gate = &mut ctx.accounts.mint_gate;
        gate.version = VERSION;
        gate.bump = ctx.bumps.mint_gate;
        gate.mint = mint;
        gate.authority = ctx.accounts.authority.key();
        gate.venue = venue;
        gate.strict = strict;
        gate.generation = 0;
        gate.bindings = Vec::new();
        gate.registered_at = ts;
        gate.reserved = [0; 32];
        // The validation account: created here (owned by this program), then written.
        let bytes = metas_bytes(&metas_of(&[]));
        let info = ctx.accounts.extra_metas.to_account_info();
        let payer = ctx.accounts.payer.to_account_info();
        let system = ctx.accounts.system_program.to_account_info();
        let bump = [metas_bump];
        let signer: &[&[u8]] = &[seeds::EXTRA_METAS, mint.as_ref(), &bump];
        create_pda(&payer, &info, &system, bytes.len(), signer)?;
        info.try_borrow_mut_data()?.copy_from_slice(&bytes);
        ctx.accounts.config.mints = ctx.accounts.config.mints.saturating_add(1);
        emit!(MintRegistered { mint, authority: ctx.accounts.authority.key(), venue, strict, ts });
        Ok(())
    }

    /// The mint's gate authority sets the venue and `strict`.
    pub fn set_venue(ctx: Context<GateAuthority>, venue: Pubkey, strict: bool) -> Result<()> {
        let gate = &mut ctx.accounts.mint_gate;
        gate.venue = venue;
        gate.strict = strict;
        emit!(VenueSet { mint: gate.mint, venue, strict });
        Ok(())
    }

    /// The mint's gate authority hands the gate to another wallet.
    pub fn set_gate_authority(ctx: Context<GateAuthority>, authority: Pubkey) -> Result<()> {
        let gate = &mut ctx.accounts.mint_gate;
        gate.authority = authority;
        emit!(GateAuthoritySet { mint: gate.mint, authority });
        Ok(())
    }

    /// The mint's gate authority binds an item to a slot. Remaining accounts: the items program's
    /// further accounts for the item (the composite's module list first, then each module's
    /// extras, then the item's `Wear` when it wears), stored and resolved on every transfer.
    pub fn bind<'info>(
        ctx: Context<'info, Bind<'info>>,
        slot: u8,
        binding_kind: u8,
        targets: Vec<Pubkey>,
        role: u8,
    ) -> Result<()> {
        require!(!ctx.accounts.config.paused, GateError::Paused);
        require!(usize::from(slot) < MAX_BINDINGS, GateError::BadSlot);
        require!(targets.len() <= MAX_TARGETS, GateError::ItemNotGateable);
        let mint = ctx.accounts.mint_gate.mint;
        require!(ctx.accounts.mint_gate.bindings.iter().all(|b| b.slot != slot), GateError::BadSlot);
        // The item: owned by the armory, at its PDA.
        let item_info = &ctx.accounts.item;
        require_keys_eq!(*item_info.owner, ids::ARMORY_ID, GateError::WrongAccount);
        let it = hookwars_armory::state::Item::try_deserialize(&mut &item_info.try_borrow_data()?[..])
            .map_err(|_| error!(GateError::WrongAccount))?;
        require_keys_eq!(item_info.key(), hookwars_common::pda::item(&it.item_mint).0, GateError::WrongAccount);
        require!(ctx.accounts.mint_gate.bindings.iter().all(|b| b.item != item_info.key()), GateError::ItemBound);
        // 18 section 3.3: no cuts, a token-side callback, the items program's own code, not exclusive.
        let m = it.manifest;
        require!(
            !m.token_cuts()
                && m.token_flags & token_flags::BEFORE_TRANSFER != 0
                && !acc::is_external(it.template_id)
                && !it.exclusive,
            GateError::ItemNotGateable
        );
        // The proof: at its address and live now.
        let proof = expected_proof(binding_kind, &item_info.key(), &it.item_mint, &mint)?;
        let ts = now()?;
        let mut b = Binding {
            slot,
            item: item_info.key(),
            item_mint: it.item_mint,
            template_id: it.template_id,
            kind: binding_kind,
            proof,
            role,
            targets,
            data_offset: 0,
            data_bytes: m.data_bytes,
            generation: 0,
            extras: ctx.remaining_accounts.iter().map(|a| a.key()).collect(),
            bound_at: ts,
        };
        require!(proof_live(&b, &mint, &ctx.accounts.proof, ts), GateError::ProofNotLive);
        // The items program's further accounts: as many as its modules read.
        require!(b.extras.len() <= MAX_BINDING_EXTRAS, GateError::TooManyAccounts);
        let expected = if it.template_id == template_id::COMPOSITE {
            let list = ctx.remaining_accounts.first().ok_or(GateError::WrongAccount)?;
            let c = hookwars_items::equip::read_composite(list, &item_info.key())?;
            for md in &c.modules {
                let mm = hookwars_common::manifest(md.template_id, &md.params, md.target_count).unwrap_or_default();
                require!(!mm.token_cuts(), GateError::ItemNotGateable);
                require!(usize::from(md.target_start) + usize::from(md.target_count) <= b.targets.len(), GateError::ItemNotGateable);
            }
            1 + c
                .modules
                .iter()
                .map(|md| hookwars_items::templates::extra_count(md.template_id, usize::from(md.target_count)))
                .sum::<usize>()
        } else {
            hookwars_items::templates::extra_count(it.template_id, b.targets.len())
        } + usize::from(it.has_wear);
        require!(b.extras.len() == expected, GateError::WrongAccount);
        let gate = &mut ctx.accounts.mint_gate;
        b.data_offset = allocate(&gate.bindings, b.data_bytes).ok_or(GateError::NoRoom)?;
        gate.generation = gate.generation.checked_add(1).ok_or(GateError::Overflow)?;
        b.generation = gate.generation;
        let (item, template, generation) = (b.item, b.template_id, b.generation);
        gate.bindings.push(b);
        gate.bindings.sort_by_key(|x| x.slot);
        write_metas(
            &ctx.accounts.extra_metas.to_account_info(),
            &ctx.accounts.payer.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            &gate.bindings,
        )?;
        emit!(ItemBound { mint, slot, item, template_id: template, kind: binding_kind, proof, generation, ts });
        Ok(())
    }

    /// Frees a slot: the mint's gate authority at any time, anyone once the binding's proof
    /// lapsed (so a lapsed binding never keeps its accounts in every transfer).
    pub fn unbind(ctx: Context<Unbind>, slot: u8) -> Result<()> {
        let ts = now()?;
        let gate = &mut ctx.accounts.mint_gate;
        let mint = gate.mint;
        let at = gate.bindings.iter().position(|b| b.slot == slot).ok_or(GateError::BadSlot)?;
        let b = gate.bindings[at].clone();
        let lapsed = !proof_live(&b, &mint, &ctx.accounts.proof, ts);
        if ctx.accounts.signer.key() != gate.authority {
            require_keys_eq!(ctx.accounts.proof.key(), b.proof, GateError::WrongAccount);
            require!(lapsed, GateError::StillLive);
        }
        gate.bindings.remove(at);
        write_metas(
            &ctx.accounts.extra_metas.to_account_info(),
            &ctx.accounts.signer.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            &gate.bindings,
        )?;
        emit!(ItemUnbound { mint, slot, item: b.item, lapsed, ts });
        Ok(())
    }

    /// The mint's gate authority takes an item out of the gate's vault (only while unbound). The
    /// vault is the owner of the item holding the owned binding kind proves.
    pub fn withdraw_item(ctx: Context<WithdrawItem>) -> Result<()> {
        let gate = &ctx.accounts.mint_gate;
        let mint = gate.mint;
        let item_mint = ctx.accounts.item_mint.key();
        require!(gate.bindings.iter().all(|b| b.item_mint != item_mint), GateError::ItemBound);
        let (vault, bump) = g::vault(&mint);
        require_keys_eq!(ctx.accounts.vault.key(), vault, GateError::WrongAccount);
        require_keys_eq!(
            ctx.accounts.vault_holding.key(),
            bordrless_token::client::holding_address(&item_mint, &vault),
            GateError::WrongAccount
        );
        let ix = bordrless_token::client::transfer(
            vault,
            ctx.accounts.vault_holding.key(),
            ctx.accounts.destination.key(),
            item_mint,
            None,
            vec![],
            1,
        );
        let b = [bump];
        let s: &[&[u8]] = &[seeds::VAULT, mint.as_ref(), &b];
        invoke_signed(
            &ix,
            &[
                ctx.accounts.vault.to_account_info(),
                ctx.accounts.vault_holding.to_account_info(),
                ctx.accounts.destination.to_account_info(),
                ctx.accounts.item_mint.to_account_info(),
                ctx.accounts.token_event_authority.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
            ],
            &[s],
        )?;
        emit!(ItemWithdrawn { mint, item_mint, to: ctx.accounts.destination.key() });
        Ok(())
    }

    /// Anyone opens a wallet's holder memory for a registered mint (the payer pays rent).
    pub fn open_holder(ctx: Context<OpenHolder>, owner: Pubkey) -> Result<()> {
        let h = &mut ctx.accounts.holder;
        h.version = VERSION;
        h.bump = ctx.bumps.holder;
        h.mint = ctx.accounts.mint_gate.mint;
        h.owner = owner;
        h.data = [0; 64];
        h.generations = [0; 4];
        Ok(())
    }

    /// The SPL transfer-hook interface's `Execute`, called by Token-2022 during a transfer.
    /// Accounts: source, mint, destination, owner, the extra-account-metas list, then the metas.
    #[instruction(discriminator = &EXECUTE_DISC)]
    pub fn execute<'info>(ctx: Context<'info, ExecuteHook<'info>>, amount: u64) -> Result<()> {
        process_execute(ctx, amount)
    }
}

/// Creates a PDA owned by this program, also when someone sent lamports to its address first
/// (a plain `create_account` would then fail and block the registration).
fn create_pda<'info>(
    payer: &AccountInfo<'info>,
    info: &AccountInfo<'info>,
    system: &AccountInfo<'info>,
    len: usize,
    signer: &[&[u8]],
) -> Result<()> {
    let rent = Rent::get()?.minimum_balance(len);
    if info.lamports() == 0 {
        invoke_signed(
            &system_instruction::create_account(payer.key, info.key, rent, len as u64, &crate::ID),
            &[payer.clone(), info.clone(), system.clone()],
            &[signer],
        )?;
        return Ok(());
    }
    require_keys_eq!(*info.owner, anchor_lang::solana_program::system_program::ID, GateError::WrongAccount);
    let top = rent.saturating_sub(info.lamports());
    if top > 0 {
        invoke(&system_instruction::transfer(payer.key, info.key, top), &[payer.clone(), info.clone(), system.clone()])?;
    }
    invoke_signed(&system_instruction::allocate(info.key, len as u64), &[info.clone(), system.clone()], &[signer])?;
    invoke_signed(&system_instruction::assign(info.key, &crate::ID), &[info.clone(), system.clone()], &[signer])?;
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Execute.

fn read_holder(info: &AccountInfo, mint: &Pubkey, owner: &Pubkey) -> Result<Option<HolderState>> {
    require_keys_eq!(*info.key, g::holder(mint, owner).0, GateError::WrongAccount);
    if *info.owner != crate::ID || info.data_is_empty() {
        return Ok(None);
    }
    Ok(Some(HolderState::try_deserialize(&mut &info.try_borrow_data()?[..])?))
}

fn range(h: &Option<HolderState>, b: &Binding) -> Vec<u8> {
    let (s, e) = (usize::from(b.data_offset), usize::from(b.data_offset) + usize::from(b.data_bytes));
    match h {
        Some(h) if h.generations[usize::from(b.slot)] == b.generation => h.data[s..e].to_vec(),
        _ => vec![0; e - s],
    }
}

fn put_range(h: &mut HolderState, b: &Binding, bytes: &[u8]) {
    let (s, e) = (usize::from(b.data_offset), usize::from(b.data_offset) + usize::from(b.data_bytes));
    let n = (e - s).min(bytes.len());
    h.data[s..s + n].copy_from_slice(&bytes[..n]);
    h.generations[usize::from(b.slot)] = b.generation;
}

fn save_holder(info: &AccountInfo, h: &HolderState) -> Result<()> {
    let mut data = info.try_borrow_mut_data()?;
    let mut out: &mut [u8] = &mut data[..];
    h.try_serialize(&mut out)
}

fn process_execute<'info>(ctx: Context<'info, ExecuteHook<'info>>, amount: u64) -> Result<()> {
    let a = &ctx.accounts;
    let mint = a.mint.key();
    // 18 section 3.4 step 1: only a real Token-2022 transfer of this mint reaches the bindings.
    let src = t22::token_account(&a.source).ok_or(GateError::NotTransferring)?;
    let dst = t22::token_account(&a.destination).ok_or(GateError::NotTransferring)?;
    require!(src.transferring && src.mint == mint && dst.mint == mint, GateError::NotTransferring);
    require!(t22::hooked_by(&a.mint, &crate::ID), GateError::NotHooked);
    require_keys_eq!(a.extra_metas.key(), g::extra_metas(&mint).0, GateError::WrongAccount);
    let rem = ctx.remaining_accounts;
    require!(rem.len() >= BASE_METAS, GateError::WrongAccount);
    let gate_info = &rem[0];
    require_keys_eq!(*gate_info.key, g::mint_gate(&mint).0, GateError::WrongAccount);
    require_keys_eq!(*gate_info.owner, crate::ID, GateError::WrongAccount);
    let gate = MintGate::try_deserialize(&mut &gate_info.try_borrow_data()?[..])?;
    if gate.bindings.is_empty() {
        return Ok(());
    }
    require_keys_eq!(*rem[1].key, ids::ITEMS_ID, GateError::WrongAccount);
    require_keys_eq!(*rem[2].key, g::GATE_ITEMS_SIGNER, GateError::WrongAccount);
    let (src_h_info, dst_h_info) = (&rem[3], &rem[4]);
    let mut src_h = read_holder(src_h_info, &mint, &src.owner)?;
    let mut dst_h = read_holder(dst_h_info, &mint, &dst.owner)?;
    let one_holder = src_h_info.key == dst_h_info.key;
    let ts = now()?;
    let (supply, decimals) = t22::mint_supply(&a.mint).ok_or(GateError::NotHooked)?;
    let same = a.source.key() == a.destination.key();
    // Balances before the transfer (Token-2022 calls the hook after moving the amount).
    let source_before = if same { src.amount } else { src.amount.checked_add(amount).ok_or(GateError::Overflow)? };
    let destination_before = if same { dst.amount } else { dst.amount.saturating_sub(amount) };
    let (_, signer_bump) = Pubkey::find_program_address(&[seeds::ITEMS_SIGNER], &crate::ID);
    let mut at = BASE_METAS;
    for b in &gate.bindings {
        let n = 2 + b.extras.len();
        require!(at + n <= rem.len(), GateError::WrongAccount);
        let (item_info, proof_info) = (&rem[at], &rem[at + 1]);
        let extras = &rem[at + 2..at + n];
        at += n;
        require_keys_eq!(*item_info.key, b.item, GateError::WrongAccount);
        require!(extras.iter().zip(b.extras.iter()).all(|(i, k)| i.key == k), GateError::WrongAccount);
        if !proof_live(b, &mint, proof_info, ts) {
            // A lapsed binding is inert: it never blocks a transfer.
            continue;
        }
        if b.data_bytes > 0 && gate.strict {
            let venue = gate.venue != Pubkey::default();
            require!(src_h.is_some() || (venue && src.owner == gate.venue), GateError::HolderStateMissing);
            require!(dst_h.is_some() || (venue && dst.owner == gate.venue), GateError::HolderStateMissing);
        }
        let args = hookwars_items::GateRunArgs {
            token: TokenSlotArgs {
                op: TokenSlotOp::Transfer,
                phase: Phase::Before,
                slot: b.slot,
                item: b.item,
                mint,
                source: a.source.key(),
                destination: a.destination.key(),
                source_owner: src.owner,
                destination_owner: dst.owner,
                authority: a.owner.key(),
                authority_is_delegate: a.owner.key() != src.owner,
                amount,
                delta: 0,
                total_delta: 0,
                source_balance: source_before,
                destination_balance: destination_before,
                decimals,
                supply,
                source_data: range(&src_h, b),
                destination_data: range(&dst_h, b),
                payload: Vec::new(),
            },
            targets: b.targets.clone(),
            role: b.role,
        };
        let mut metas = vec![
            AccountMeta::new_readonly(g::GATE_ITEMS_SIGNER, true),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(b.item, false),
        ];
        metas.extend(b.extras.iter().map(|k| AccountMeta::new_readonly(*k, false)));
        let ix = Instruction {
            program_id: ids::ITEMS_ID,
            accounts: metas,
            data: hookwars_items::instruction::GateBefore { args }.data(),
        };
        let mut infos = vec![rem[2].clone(), a.mint.to_account_info(), item_info.clone()];
        infos.extend(extras.iter().cloned());
        infos.push(rem[1].clone());
        let bump = [signer_bump];
        invoke_signed(&ix, &infos, &[&[seeds::ITEMS_SIGNER, &bump]])?;
        let Some((program, data)) = get_return_data() else { continue };
        if program != ids::ITEMS_ID {
            continue;
        }
        let answer = SlotReturn::deserialize(&mut &data[..]).map_err(|_| error!(GateError::WrongAccount))?;
        if let (Some(bytes), Some(h)) = (answer.source_data.as_ref(), src_h.as_mut()) {
            put_range(h, b, bytes);
        }
        if one_holder {
            // One wallet on both sides: one account, the destination's answer last.
            if let (Some(bytes), Some(h)) = (answer.destination_data.as_ref(), src_h.as_mut()) {
                put_range(h, b, bytes);
            }
            dst_h = src_h.clone();
        } else if let (Some(bytes), Some(h)) = (answer.destination_data.as_ref(), dst_h.as_mut()) {
            put_range(h, b, bytes);
        }
    }
    if let Some(h) = &src_h {
        save_holder(src_h_info, h)?;
    }
    if let Some(h) = &dst_h {
        if !one_holder {
            save_holder(dst_h_info, h)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Accounts.

#[derive(Accounts)]
pub struct InitConfig<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(init, payer = authority, space = 8 + GateConfig::INIT_SPACE, seeds = [seeds::CONFIG], bump)]
    pub config: Box<Account<'info, GateConfig>>,
    /// CHECK: this program's ProgramData, parsed in the handler.
    pub program_data: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AdminOnly<'info> {
    pub admin: Signer<'info>,
    #[account(mut, seeds = [seeds::CONFIG], bump = config.bump, has_one = admin @ GateError::NotAdmin)]
    pub config: Box<Account<'info, GateConfig>>,
}

#[derive(Accounts)]
pub struct AcceptAdmin<'info> {
    pub new_admin: Signer<'info>,
    #[account(mut, seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, GateConfig>>,
}

#[derive(Accounts)]
pub struct RegisterMint<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// The mint's transfer-hook authority.
    pub authority: Signer<'info>,
    #[account(mut, seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, GateConfig>>,
    /// CHECK: the Token-2022 mint (its `TransferHook` extension is read in the handler).
    pub mint: UncheckedAccount<'info>,
    #[account(init, payer = payer, space = 8 + MintGate::INIT_SPACE,
        seeds = [seeds::MINT_GATE, mint.key().as_ref()], bump)]
    pub mint_gate: Box<Account<'info, MintGate>>,
    /// CHECK: `["extra-account-metas", mint]`, created in the handler.
    #[account(mut)]
    pub extra_metas: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct GateAuthority<'info> {
    pub authority: Signer<'info>,
    #[account(mut, seeds = [seeds::MINT_GATE, mint_gate.mint.as_ref()], bump = mint_gate.bump,
        has_one = authority @ GateError::NotGateAuthority)]
    pub mint_gate: Box<Account<'info, MintGate>>,
}

#[derive(Accounts)]
pub struct Bind<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub authority: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, GateConfig>>,
    #[account(mut, seeds = [seeds::MINT_GATE, mint_gate.mint.as_ref()], bump = mint_gate.bump,
        has_one = authority @ GateError::NotGateAuthority)]
    pub mint_gate: Box<Account<'info, MintGate>>,
    /// CHECK: `["extra-account-metas", mint]`.
    #[account(mut, seeds = [seeds::EXTRA_METAS, mint_gate.mint.as_ref()], bump, owner = crate::ID)]
    pub extra_metas: UncheckedAccount<'info>,
    /// CHECK: the armory `Item` (checked in the handler).
    pub item: UncheckedAccount<'info>,
    /// CHECK: the binding's proof (checked in the handler).
    pub proof: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Unbind<'info> {
    /// The mint's gate authority, or anyone once the binding lapsed (pays any rent difference).
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(mut, seeds = [seeds::MINT_GATE, mint_gate.mint.as_ref()], bump = mint_gate.bump)]
    pub mint_gate: Box<Account<'info, MintGate>>,
    /// CHECK: `["extra-account-metas", mint]`.
    #[account(mut, seeds = [seeds::EXTRA_METAS, mint_gate.mint.as_ref()], bump, owner = crate::ID)]
    pub extra_metas: UncheckedAccount<'info>,
    /// CHECK: the binding's proof (checked in the handler).
    pub proof: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WithdrawItem<'info> {
    pub authority: Signer<'info>,
    #[account(seeds = [seeds::MINT_GATE, mint_gate.mint.as_ref()], bump = mint_gate.bump,
        has_one = authority @ GateError::NotGateAuthority)]
    pub mint_gate: Box<Account<'info, MintGate>>,
    /// CHECK: `["gate-vault", mint]` (checked in the handler).
    pub vault: UncheckedAccount<'info>,
    /// CHECK: the item mint (a units token).
    pub item_mint: UncheckedAccount<'info>,
    /// CHECK: the vault's holding of the item mint (checked in the handler).
    #[account(mut)]
    pub vault_holding: UncheckedAccount<'info>,
    /// CHECK: the receiving holding (the token program checks it).
    #[account(mut)]
    pub destination: UncheckedAccount<'info>,
    /// CHECK: the token program.
    #[account(address = bordrless_token::ID)]
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: its event authority.
    #[account(address = bordrless_token::client::event_authority())]
    pub token_event_authority: UncheckedAccount<'info>,
}

#[derive(Accounts)]
#[instruction(owner: Pubkey)]
pub struct OpenHolder<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(seeds = [seeds::MINT_GATE, mint_gate.mint.as_ref()], bump = mint_gate.bump)]
    pub mint_gate: Box<Account<'info, MintGate>>,
    #[account(init, payer = payer, space = 8 + HolderState::INIT_SPACE,
        seeds = [seeds::HOLDER, mint_gate.mint.as_ref(), owner.as_ref()], bump)]
    pub holder: Box<Account<'info, HolderState>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ExecuteHook<'info> {
    /// CHECK: the source token account (read in the handler).
    pub source: UncheckedAccount<'info>,
    /// CHECK: the Token-2022 mint.
    pub mint: UncheckedAccount<'info>,
    /// CHECK: the destination token account.
    pub destination: UncheckedAccount<'info>,
    /// CHECK: the source's owner or delegate.
    pub owner: UncheckedAccount<'info>,
    /// CHECK: `["extra-account-metas", mint]` (checked in the handler).
    pub extra_metas: UncheckedAccount<'info>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_are_first_fit_and_never_overlap() {
        let b = |slot: u8, off: u8, len: u8| Binding { slot, data_offset: off, data_bytes: len, ..Default::default() };
        assert_eq!(allocate(&[], 5), Some(0));
        assert_eq!(allocate(&[b(0, 0, 5)], 5), Some(5));
        assert_eq!(allocate(&[b(0, 0, 5), b(1, 10, 5)], 5), Some(5));
        assert_eq!(allocate(&[b(0, 0, 5), b(1, 10, 5)], 6), Some(15));
        assert_eq!(allocate(&[b(0, 0, 60)], 5), None);
        assert_eq!(allocate(&[b(0, 0, 60)], 0), Some(0));
    }

    #[test]
    fn the_metas_list_is_the_interfaces_tlv() {
        let gate = MintGate {
            version: 1,
            bump: 1,
            mint: Pubkey::new_unique(),
            authority: Pubkey::new_unique(),
            venue: Pubkey::default(),
            strict: true,
            generation: 0,
            bindings: vec![],
            registered_at: 0,
            reserved: [0; 32],
        };
        let d = metas_bytes(&metas_of(&gate.bindings));
        assert_eq!(d[..8], EXECUTE_DISC);
        assert_eq!(u32::from_le_bytes(d[8..12].try_into().unwrap()) as usize, 4 + 35 * BASE_METAS);
        assert_eq!(u32::from_le_bytes(d[12..16].try_into().unwrap()) as usize, BASE_METAS);
        assert_eq!(d.len(), 16 + 35 * BASE_METAS);
        // The holder metas: PDA of this program, seeds literal, mint, the token account's owner.
        let h = &d[16 + 35 * 3..16 + 35 * 4];
        assert_eq!(h[0], 1);
        assert_eq!(h[34], 1);
    }

    #[test]
    fn the_mint_gate_head_offsets_match_the_shared_reader() {
        let mint = Pubkey::new_unique();
        let venue = Pubkey::new_unique();
        let gate = MintGate {
            version: 1,
            bump: 2,
            mint,
            authority: Pubkey::new_unique(),
            venue,
            strict: false,
            generation: 0,
            bindings: vec![],
            registered_at: 0,
            reserved: [0; 32],
        };
        let mut d = Vec::new();
        gate.try_serialize(&mut d).unwrap();
        assert_eq!(d[..8], g::MINT_GATE_DISC);
        assert_eq!(d[g::MINT_GATE_MINT..g::MINT_GATE_MINT + 32], mint.to_bytes());
        assert_eq!(d[g::MINT_GATE_VENUE..g::MINT_GATE_VENUE + 32], venue.to_bytes());
    }
}
