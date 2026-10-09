// Changed by Hookwars: new file, slot-table instructions (create_slot_mint, set_slot_item,; R21 may_burn only on Pool slots. M3b: Pool slots may cut.
// set_vote_lock, touch); M2: Fee items may write data, Relation items may carry pool flags.
//! The slot table (docs/spec/01-token-slots.md sections 1, 4 and 5).

use anchor_lang::prelude::*;
use bordrless_hook::{
    discriminators, equip_owner, equip_rule, hook_signer, hook_signer_at, read_slot_answer,
    slot_authority, slot_flags, slot_kind, token_flags, Phase, SlotAllowed, TokenSlotArgs,
    TokenSlotOp, HOOK_DATA_LEN, MAX_DELTAS, MAX_HOOK_DATA,
};

use crate::client::read_holding;
use crate::constants::*;
use crate::error::{slot_answer_error, TokenError};
use crate::events::*;
use crate::hooks::HookCall;
use crate::instructions::mint::{check_metadata, CreateMint, CreateMintArgs};
use crate::slots::{read_range, write_range};
use crate::state::*;

/// One slot of a new slot table.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct SlotInit {
    /// `bordrless_hook::slot_kind`.
    pub kind: u8,
    /// `bordrless_hook::equip_rule`.
    pub equip_rule: u8,
    /// Bounds for life.
    pub bounds: SlotBounds,
    /// Item slots: range length including the epoch byte, or 0. Locked: the legacy program's
    /// range from byte 0 (the kit: 32), or 0.
    pub data_len: u8,
    /// Locked slot only: the legacy program.
    pub locked_program: Option<Pubkey>,
    /// Locked slot only: its upstream token flags.
    pub locked_flags: u16,
    /// Locked slot only: how many extra accounts it takes (its registry's length).
    pub locked_extra_count: u8,
}

fn kind_may_cut(kind: u8) -> bool {
    // Hookwars M3b: a Pool slot hosts Fee modules of composites (08 section 2.8), so it may cut on
    // the token side too.
    matches!(
        kind,
        slot_kind::FEE | slot_kind::REWARD | slot_kind::RELATION | slot_kind::POOL
    )
}

/// `create_slot_mint`: a mint with a slot table and no single hook.
pub fn process_create_slot_mint(
    ctx: Context<CreateMint>,
    args: CreateMintArgs,
    authority: Option<Pubkey>,
    inits: Vec<SlotInit>,
) -> Result<()> {
    require!(args.decimals <= MAX_DECIMALS, TokenError::InvalidDecimals);
    check_metadata(&args.name, &args.symbol, &args.uri)?;
    require!(
        args.hook_program.is_none() && args.hook_flags == 0 && args.hook_authority.is_none(),
        TokenError::MixedHookModes
    );
    require!(
        !inits.is_empty() && inits.len() <= MAX_SLOTS,
        TokenError::InvalidSlotTable
    );
    let mint_key = ctx.accounts.mint.key();
    if let Some(a) = authority {
        require_keys_eq!(
            a,
            slot_authority(&ARMORY_ID, &mint_key).0,
            TokenError::InvalidSlotAuthority
        );
    }

    let mut table = [Slot::default(); MAX_SLOTS];
    let mut locked_seen = false;
    let mut locked_cuts = false;
    let mut cutting_items = 0usize;
    let mut cut_sum = 0u32;
    let mut has_items = false;
    // The Locked range comes first, from byte 0.
    let mut offset = inits
        .iter()
        .find(|s| s.kind == slot_kind::LOCKED)
        .map_or(0usize, |s| usize::from(s.data_len));
    for (i, init) in inits.iter().enumerate() {
        require!(
            init.kind < slot_kind::COUNT && init.equip_rule < equip_rule::COUNT,
            TokenError::InvalidSlotTable
        );
        let b = init.bounds;
        // Hookwars R21: only a Pool slot may let its item burn part of a swap.
        require!(
            !b.may_burn || init.kind == slot_kind::POOL,
            TokenError::InvalidSlotTable
        );
        cut_sum += u32::from(b.max_cut_bps);
        let slot = &mut table[i];
        slot.kind = init.kind;
        slot.equip_rule = init.equip_rule;
        slot.bounds = b;
        slot.data_len = init.data_len;
        slot.data_epoch = 1;
        if init.kind == slot_kind::LOCKED {
            require!(!locked_seen, TokenError::InvalidSlotTable);
            locked_seen = true;
            let program = init.locked_program.ok_or(TokenError::InvalidSlotTable)?;
            require!(
                init.equip_rule == equip_rule::LOCKED
                    && program != crate::ID
                    && init.locked_flags & !token_flags::ALL == 0,
                TokenError::InvalidSlotTable
            );
            let writes = init.locked_flags & token_flags::WRITES_HOOK_DATA != 0;
            require!(
                (writes && init.data_len > 0 && b.may_write_data)
                    || (!writes && init.data_len == 0 && !b.may_write_data),
                TokenError::InvalidSlotTable
            );
            let returns = init.locked_flags & token_flags::TRANSFER_RETURNS_DELTA != 0;
            require!(
                !b.may_answer_touch && (b.max_cut_bps == 0 || returns),
                TokenError::InvalidSlotTable
            );
            locked_cuts = returns && b.max_cut_bps > 0;
            slot.data_offset = 0;
            slot.program = program;
            slot.flags = init.locked_flags;
            slot.signer_bump = Mint::hook_signer(&program).1;
            slot.extra_count = init.locked_extra_count;
        } else {
            has_items = true;
            require!(
                init.locked_program.is_none()
                    && init.locked_flags == 0
                    && init.locked_extra_count == 0,
                TokenError::InvalidSlotTable
            );
            require!(
                b.max_cut_bps == 0 || kind_may_cut(init.kind),
                TokenError::InvalidSlotTable
            );
            if init.kind == slot_kind::WAR {
                require!(
                    init.data_len == 0
                        && b.max_cut_bps == 0
                        && !b.may_write_data
                        && !b.may_answer_touch,
                    TokenError::InvalidSlotTable
                );
            }
            require!(
                (b.may_write_data && init.data_len >= 2) || (!b.may_write_data && init.data_len == 0),
                TokenError::InvalidSlotTable
            );
            require!(
                !b.may_answer_touch
                    || (b.may_write_data
                        && matches!(
                            init.kind,
                            slot_kind::REWARD
                                | slot_kind::DEFENSE
                                | slot_kind::RELATION
                                | slot_kind::POOL
                        )),
                TokenError::InvalidSlotTable
            );
            if init.data_len > 0 {
                slot.data_offset = offset as u8;
                offset += usize::from(init.data_len);
            }
            if b.max_cut_bps > 0 {
                cutting_items += 1;
                let owner = equip_owner(&ITEMS_ID, &mint_key, i as u8).0;
                slot.equip_vault = Holding::address(&mint_key, &owner).0;
            }
        }
    }
    require!(offset <= HOOK_DATA_LEN, TokenError::InvalidSlotTable);
    require!(cut_sum <= 10_000, TokenError::InvalidSlotTable);
    require!(
        cutting_items <= MAX_CUTTING_SLOTS
            && cutting_items + usize::from(locked_cuts) <= MAX_DELTAS,
        TokenError::TooManyCuttingSlots
    );
    if has_items {
        require!(authority.is_some(), TokenError::InvalidSlotAuthority);
    }

    let now = Clock::get()?.unix_timestamp;
    let mint = &mut ctx.accounts.mint;
    mint.version = VERSION;
    mint.decimals = args.decimals;
    mint.supply = 0;
    mint.max_supply = args.max_supply;
    mint.mint_authority = args.mint_authority;
    mint.freeze_authority = args.freeze_authority;
    mint.hook_authority = None;
    mint.metadata_authority = args.metadata_authority;
    mint.hook_program = None;
    mint.hook_flags = 0;
    mint.name = args.name.clone();
    mint.symbol = args.symbol.clone();
    mint.uri = args.uri.clone();
    mint.created_at = now;
    mint.creator = ctx.accounts.payer.key();
    mint.hook_signer_bump = 0;
    mint.reserved = [0; 31];
    mint.slot_authority = authority;
    mint.slot_count = inits.len() as u8;
    mint.slots = table;
    emit_cpi!(MintCreated {
        mint: mint.key(),
        creator: ctx.accounts.payer.key(),
        decimals: args.decimals,
        max_supply: args.max_supply,
        mint_authority: args.mint_authority,
        freeze_authority: args.freeze_authority,
        hook_authority: None,
        metadata_authority: args.metadata_authority,
        hook_program: None,
        hook_flags: 0,
        name: args.name,
        symbol: args.symbol,
        uri: args.uri,
        ts: now,
    });
    emit_cpi!(SlotsInitialized {
        mint: mint.key(),
        slot_authority: authority,
        slots: mint
            .active_slots()
            .iter()
            .map(|s| SlotInfo {
                kind: s.kind,
                equip_rule: s.equip_rule,
                max_cut_bps: s.bounds.max_cut_bps,
                may_refuse: s.bounds.may_refuse,
                may_write_data: s.bounds.may_write_data,
                may_answer_touch: s.bounds.may_answer_touch,
                may_burn: s.bounds.may_burn,
                data_offset: s.data_offset,
                data_len: s.data_len,
                equip_vault: s.equip_vault,
                locked_program: if s.is_locked() {
                    s.program
                } else {
                    Pubkey::default()
                },
            })
            .collect(),
    });
    Ok(())
}

/// Accounts of `set_slot_item`.
#[event_cpi]
#[derive(Accounts)]
pub struct SetSlotItem<'info> {
    /// The armory's `["slots", mint]` PDA, signing by CPI.
    pub slot_authority: Signer<'info>,
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    /// CHECK: the new item's program (executable), or this program's id to empty the slot.
    pub item_program: UncheckedAccount<'info>,
    /// CHECK: the slot's equip vault when the item may cut (checked in the handler).
    pub equip_vault: Option<UncheckedAccount<'info>>,
}

/// Token flags a slot of `kind` with `bounds` may carry.
fn allowed_flags(kind: u8, bounds: &SlotBounds) -> u16 {
    let base = match kind {
        // Hookwars M2: Half-Life is a Fee item that writes hook data (04 section 3.7).
        slot_kind::FEE => {
            slot_flags::TRANSFER | slot_flags::TRANSFER_RETURNS_DELTA | slot_flags::WRITES_HOOK_DATA
        }
        slot_kind::REWARD | slot_kind::RELATION => {
            slot_flags::TRANSFER
                | slot_flags::BURN
                | slot_flags::TRANSFER_RETURNS_DELTA
                | slot_flags::WRITES_HOOK_DATA
                | slot_flags::ANSWERS_TOUCH
        }
        slot_kind::DEFENSE => {
            slot_flags::TRANSFER
                | slot_flags::BURN
                | slot_flags::WRITES_HOOK_DATA
                | slot_flags::ANSWERS_TOUCH
        }
        // Hookwars M3b: a Pool slot hosts Fee modules of composites (08 section 2.8), so it may
        // answer a token-side cut too.
        slot_kind::POOL => {
            slot_flags::TRANSFER
                | slot_flags::TRANSFER_RETURNS_DELTA
                | slot_flags::WRITES_HOOK_DATA
                | slot_flags::ANSWERS_TOUCH
        }
        _ => 0,
    };
    let mut allowed = base;
    if bounds.max_cut_bps == 0 {
        allowed &= !slot_flags::TRANSFER_RETURNS_DELTA;
    }
    if !bounds.may_write_data {
        allowed &= !slot_flags::WRITES_HOOK_DATA;
    }
    if !bounds.may_answer_touch {
        allowed &= !slot_flags::ANSWERS_TOUCH;
    }
    allowed
}

/// `set_slot_item`: the armory puts an item in a slot (or empties it). The bounds are enforced at
/// every operation; whether the item's manifest fits them is the armory's to check.
pub fn process_set_slot_item(
    ctx: Context<SetSlotItem>,
    index: u8,
    item: Pubkey,
    flags: u16,
    pool_flags: u16,
    extra_count: u8,
) -> Result<()> {
    let mint = &mut ctx.accounts.mint;
    require!(
        mint.slot_authority == Some(ctx.accounts.slot_authority.key()),
        TokenError::InvalidSlotAuthority
    );
    require!(index < mint.slot_count, TokenError::SlotIndexOutOfRange);
    let slot = mint.slots[usize::from(index)];
    require!(!slot.is_locked(), TokenError::SlotLocked);
    let program = ctx.accounts.item_program.key();
    let emptying = item == Pubkey::default();
    if emptying {
        require!(
            flags == 0 && pool_flags == 0 && program == crate::ID && extra_count == 0,
            TokenError::InvalidSlotItem
        );
    } else {
        require!(
            ctx.accounts.item_program.executable
                && ![crate::ID, SWAP_ID, LAUNCH_ID, BRIDGE_ID, ARMORY_ID, WAR_ID]
                    .contains(&program),
            TokenError::InvalidSlotItem
        );
        require!(
            flags & !allowed_flags(slot.kind, &slot.bounds) == 0
                && (pool_flags == 0
                    || slot.kind == slot_kind::POOL
                    || slot.kind == slot_kind::RELATION),
            TokenError::SlotFlagsNotAllowed
        );
        if flags & slot_flags::TRANSFER_RETURNS_DELTA != 0 {
            let vault = ctx
                .accounts
                .equip_vault
                .as_ref()
                .ok_or(TokenError::WrongEquipVault)?;
            require_keys_eq!(vault.key(), slot.equip_vault, TokenError::WrongEquipVault);
            let holding =
                read_holding(&vault.to_account_info()).map_err(|_| TokenError::WrongEquipVault)?;
            require_keys_eq!(holding.mint, mint.key(), TokenError::WrongEquipVault);
        }
    }
    let epoch = if slot.data_epoch == u8::MAX {
        1
    } else {
        slot.data_epoch + 1
    };
    let s = &mut mint.slots[usize::from(index)];
    let old_item = s.item;
    s.item = item;
    s.flags = flags;
    s.pool_flags = pool_flags;
    s.extra_count = extra_count;
    s.data_epoch = epoch;
    if emptying {
        s.program = Pubkey::default();
        s.signer_bump = 0;
        s.launch_signer_bump = 0;
    } else {
        s.program = program;
        s.signer_bump = hook_signer(&crate::ID, &program).1;
        // Hookwars M2: Relation items (Treaty, Tribute) have a pool half too (04 section 3).
        s.launch_signer_bump = if s.kind == slot_kind::POOL || s.kind == slot_kind::RELATION {
            hook_signer(&LAUNCH_ID, &program).1
        } else {
            0
        };
    }
    emit_cpi!(SlotEquipped {
        mint: mint.key(),
        slot: index,
        old_item,
        new_item: item,
        program: if emptying { Pubkey::default() } else { program },
        flags,
        pool_flags,
        data_epoch: epoch,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// Accounts of `set_vote_lock`.
#[event_cpi]
#[derive(Accounts)]
pub struct SetVoteLock<'info> {
    /// The armory's `["slots", mint]` PDA, signing by CPI.
    pub slot_authority: Signer<'info>,
    pub mint: Account<'info, Mint>,
    #[account(mut, has_one = mint @ TokenError::MintMismatch)]
    pub holding: Account<'info, Holding>,
}

/// `set_vote_lock` (R11): locks tokens in place for a vote; `amount` 0 or a past `until` clears.
pub fn process_set_vote_lock(ctx: Context<SetVoteLock>, amount: u64, until: i64) -> Result<()> {
    let mint = &ctx.accounts.mint;
    require!(
        mint.slot_authority.is_some()
            && mint.slot_authority == Some(ctx.accounts.slot_authority.key()),
        TokenError::InvalidSlotAuthority
    );
    let now = Clock::get()?.unix_timestamp;
    let holding = &mut ctx.accounts.holding;
    require!(!holding.frozen, TokenError::Frozen);
    require!(amount <= holding.amount, TokenError::VoteLockExceedsBalance);
    if amount == 0 || until <= now {
        holding.vote_locked = 0;
        holding.vote_lock_until = 0;
    } else {
        holding.vote_locked = amount;
        holding.vote_lock_until = until;
    }
    emit_cpi!(VoteLockSet {
        mint: mint.key(),
        holding: holding.key(),
        owner: holding.owner,
        amount: holding.vote_locked,
        until: holding.vote_lock_until,
        ts: now,
    });
    Ok(())
}

/// Accounts of `touch`. Remaining accounts: the slot's extras.
#[event_cpi]
#[derive(Accounts)]
pub struct Touch<'info> {
    /// Any signer; the item decides whether to act for it.
    pub caller: Signer<'info>,
    pub mint: Account<'info, Mint>,
    #[account(mut, has_one = mint @ TokenError::MintMismatch)]
    pub holding: Account<'info, Holding>,
    /// CHECK: the slot's program (checked in the handler).
    pub item_program: UncheckedAccount<'info>,
    /// CHECK: this program's signer of the slot's callbacks (checked in the handler).
    pub hook_signer: UncheckedAccount<'info>,
}

/// `touch`: one slot's item may rewrite one holding's range, with no transfer.
pub fn process_touch<'info>(
    ctx: Context<'info, Touch<'info>>,
    index: u8,
    payload: Vec<u8>,
) -> Result<()> {
    let mint = &ctx.accounts.mint;
    require!(index < mint.slot_count, TokenError::SlotIndexOutOfRange);
    let slot = mint.slots[usize::from(index)];
    require!(slot.is_filled(), TokenError::SlotEmpty);
    require!(
        !slot.is_locked()
            && slot.kind != slot_kind::WAR
            && slot.flags & slot_flags::ANSWERS_TOUCH != 0,
        TokenError::TouchNotSupported
    );
    require!(payload.len() <= MAX_HOOK_DATA, TokenError::PayloadTooLong);
    require!(
        ctx.remaining_accounts.len() == usize::from(slot.extra_count),
        TokenError::SlotAccountsMismatch
    );
    require_keys_eq!(
        ctx.accounts.item_program.key(),
        slot.program,
        TokenError::WrongHookProgram
    );
    let expected =
        hook_signer_at(&crate::ID, &slot.program, slot.signer_bump).ok_or(TokenError::BadHookSigner)?;
    require_keys_eq!(
        ctx.accounts.hook_signer.key(),
        expected,
        TokenError::BadHookSigner
    );
    let holding = &mut ctx.accounts.holding;
    require!(!holding.frozen, TokenError::Frozen);
    let range = read_range(&holding.hook_data, &slot);
    let call = HookCall {
        hook_program: ctx.accounts.item_program.to_account_info(),
        hook_signer: ctx.accounts.hook_signer.to_account_info(),
        bump: slot.signer_bump,
        prefix: [
            mint.to_account_info(),
            holding.to_account_info(),
            holding.to_account_info(),
            ctx.accounts.caller.to_account_info(),
        ],
        extras: ctx.remaining_accounts,
    };
    let args = TokenSlotArgs {
        op: TokenSlotOp::Touch,
        phase: Phase::Before,
        slot: index,
        item: slot.item,
        mint: mint.key(),
        source: holding.key(),
        destination: holding.key(),
        source_owner: holding.owner,
        destination_owner: holding.owner,
        authority: ctx.accounts.caller.key(),
        authority_is_delegate: false,
        amount: 0,
        delta: 0,
        total_delta: 0,
        source_balance: holding.amount,
        destination_balance: holding.amount,
        decimals: mint.decimals,
        supply: mint.supply,
        source_data: range.clone(),
        destination_data: range,
        payload,
    };
    let mut data = Vec::with_capacity(8 + 400);
    data.extend_from_slice(&discriminators::ON_TOUCH);
    args.serialize(&mut data)?;
    call.invoke_raw(data)?;
    let allowed = SlotAllowed::of(TokenSlotOp::Touch, Phase::Before, slot.flags);
    let len = usize::from(slot.data_len).saturating_sub(1);
    if let Some((answer, _)) =
        read_slot_answer(&slot.program, allowed, len).map_err(slot_answer_error)?
    {
        if let Some(bytes) = answer.source_data {
            write_range(&mut holding.hook_data, &slot, &bytes);
            emit_cpi!(HookDataWritten {
                mint: mint.key(),
                holding: holding.key(),
                owner: holding.owner,
                data: holding.hook_data,
            });
        }
    }
    Ok(())
}
