// Changed by Hookwars: slot table in the mint, vote lock in the holding.
//! Accounts of the token standard.

use anchor_lang::prelude::*;

use crate::constants::*;

/// A mint. Address: any signer at creation (a keypair, or a PDA of the creating program).
#[account]
#[derive(InitSpace, Debug)]
pub struct Mint {
    /// Layout version.
    pub version: u8,
    /// Decimals.
    pub decimals: u8,
    /// Current supply.
    pub supply: u64,
    /// Largest supply ever allowed; 0 means unlimited.
    pub max_supply: u64,
    /// May mint; `None` once revoked.
    pub mint_authority: Option<Pubkey>,
    /// May freeze and thaw holdings; `None` once revoked.
    pub freeze_authority: Option<Pubkey>,
    /// May change the hook; `None` locks the hook for ever.
    pub hook_authority: Option<Pubkey>,
    /// May change the metadata; `None` fixes it for ever.
    pub metadata_authority: Option<Pubkey>,
    /// The hook program, if any.
    pub hook_program: Option<Pubkey>,
    /// Which callbacks run (`bordrless_hook::token_flags`).
    pub hook_flags: u16,
    /// Name.
    #[max_len(32)]
    pub name: String,
    /// Symbol.
    #[max_len(10)]
    pub symbol: String,
    /// Metadata URI.
    #[max_len(200)]
    pub uri: String,
    /// Creation time.
    pub created_at: i64,
    /// Who paid for the creation.
    pub creator: Pubkey,
    /// Bump of `["hook-authority", hook_program]`, this program's signer of every callback to the
    /// hook (one per hook program, so a hook can tell its own callbacks from a signer another hook
    /// passed on); 0 without a hook. Set with the hook.
    pub hook_signer_bump: u8,
    /// Reserved.
    pub reserved: [u8; 31],
    /// Hookwars: the armory's `["slots", mint]` PDA, which alone may change the slots and vote
    /// locks; `None` when the table never changes.
    pub slot_authority: Option<Pubkey>,
    /// Hookwars: entries of `slots` in use (0: no slot table, the upstream single hook applies).
    pub slot_count: u8,
    /// Hookwars: the slot table, in call order; entries at `slot_count..` are zero.
    pub slots: [Slot; MAX_SLOTS],
}

/// Bounds a slot keeps for life.
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SlotBounds {
    /// Most an item in this slot may cut from one transfer, in bps of the amount.
    pub max_cut_bps: u16,
    /// Whether an item may refuse operations (declared; not enforceable at run time).
    pub may_refuse: bool,
    /// Whether an item may write this slot's hook-data range.
    pub may_write_data: bool,
    /// Whether an item may answer `touch`.
    pub may_answer_touch: bool,
}

/// One slot of a mint (docs/spec/01-token-slots.md section 1.1).
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Slot {
    /// `bordrless_hook::slot_kind`.
    pub kind: u8,
    /// `bordrless_hook::equip_rule` (stored, never acted on here).
    pub equip_rule: u8,
    /// Fixed bounds.
    pub bounds: SlotBounds,
    /// First byte of this slot's range in `Holding.hook_data`.
    pub data_offset: u8,
    /// Length of the range (item slots: including the epoch byte; 0 without data).
    pub data_len: u8,
    /// The equipped item (default when empty or Locked).
    pub item: Pubkey,
    /// The program that runs (default when empty).
    pub program: Pubkey,
    /// Token callbacks and answers (`bordrless_hook::slot_flags`).
    pub flags: u16,
    /// Pool slots only: the launchpad's pool flags (stored, never read here).
    pub pool_flags: u16,
    /// The only holding a cut of this slot may credit (default when the slot never cuts).
    pub equip_vault: Pubkey,
    /// Bump of `["hook-authority", program]` under this program.
    pub signer_bump: u8,
    /// Pool slots only: bump of the launchpad's `["hook-authority", program]`.
    pub launch_signer_bump: u8,
    /// Tags this slot's data in every holding; changes on every equip; starts at 1.
    pub data_epoch: u8,
    /// Extra accounts the item takes after its program and signer (fixed when equipped; the
    /// Locked slot's when created). Hookwars M1: replaces the `slot_accounts` argument (M1 notes).
    pub extra_count: u8,
}

impl Slot {
    /// Whether an item (or the Locked program) is in the slot.
    pub fn is_filled(&self) -> bool {
        self.program != Pubkey::default()
    }

    /// Whether this is the Locked slot.
    pub fn is_locked(&self) -> bool {
        self.kind == bordrless_hook::slot_kind::LOCKED
    }
}

impl Mint {
    /// Account size.
    pub const LEN: usize = DISCRIMINATOR_LEN + Self::INIT_SPACE;

    /// The hook, if one is set.
    pub fn hook(&self) -> Option<(Pubkey, u16)> {
        self.hook_program.map(|p| (p, self.hook_flags))
    }

    /// This program's signer of the callbacks to `hook_program`, `["hook-authority",
    /// hook_program]`, and its bump (a search: for creating a mint or setting its hook).
    pub fn hook_signer(hook_program: &Pubkey) -> (Pubkey, u8) {
        bordrless_hook::hook_signer(&crate::ID, hook_program)
    }

    /// Whether the mint runs a slot table.
    pub fn uses_slots(&self) -> bool {
        self.slot_count > 0
    }

    /// The slots in use.
    pub fn active_slots(&self) -> &[Slot] {
        &self.slots[..usize::from(self.slot_count).min(MAX_SLOTS)]
    }

    /// The Locked slot, if any.
    pub fn locked_slot(&self) -> Option<(u8, &Slot)> {
        self.active_slots()
            .iter()
            .enumerate()
            .find(|(_, s)| s.is_locked())
            .map(|(i, s)| (i as u8, s))
    }

    /// Whether the mint's hook keeps hook data in its holdings (a hook with `WRITES_HOOK_DATA`).
    pub fn hook_writes_data(&self) -> bool {
        self.hook_program.is_some()
            && self.hook_flags & bordrless_hook::token_flags::WRITES_HOOK_DATA != 0
    }
}

/// A holding: the token account of `owner` for `mint`, at `["holding", mint, owner]`.
#[account]
#[derive(InitSpace, Debug)]
pub struct Holding {
    /// Layout version.
    pub version: u8,
    /// Bump of the PDA.
    pub bump: u8,
    /// The mint.
    pub mint: Pubkey,
    /// The owner.
    pub owner: Pubkey,
    /// Balance.
    pub amount: u64,
    /// Delegate, if any.
    pub delegate: Option<Pubkey>,
    /// What the delegate may still move.
    pub delegated_amount: u64,
    /// Frozen holdings cannot send, receive or burn.
    pub frozen: bool,
    /// State the mint's hook keeps for this holder (`bordrless_hook::HOOK_DATA_LEN` bytes). Only
    /// the mint's hook program changes it: by answering a `before_*` callback, or through
    /// `write_hook_data`, and only while the mint has `WRITES_HOOK_DATA`. Zero until then.
    pub hook_data: [u8; 64],
    /// Hookwars (R11): tokens the armory locked for a vote (was `reserved[0..8]`).
    pub vote_locked: u64,
    /// Hookwars (R11): unix time the lock ends; 0 with no lock (was `reserved[8..16]`).
    pub vote_lock_until: i64,
}

impl Holding {
    /// The amount a vote lock holds at `now` (0 without a live lock).
    pub fn locked_at(&self, now: i64) -> u64 {
        if now < self.vote_lock_until {
            self.vote_locked
        } else {
            0
        }
    }

    /// Account size.
    pub const LEN: usize = DISCRIMINATOR_LEN + Self::INIT_SPACE;

    /// Whether the hook data is all zero (nothing kept for this holder).
    pub fn hook_data_is_empty(&self) -> bool {
        self.hook_data.iter().all(|b| *b == 0)
    }

    /// The holding address of `owner` for `mint`.
    pub fn address(mint: &Pubkey, owner: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[HOLDING_SEED, mint.as_ref(), owner.as_ref()], &crate::ID)
    }
}

/// Which authority `set_authority` replaces.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorityKind {
    /// `mint_authority`.
    Mint,
    /// `freeze_authority`.
    Freeze,
    /// `hook_authority`.
    Hook,
    /// `metadata_authority`.
    Metadata,
}
