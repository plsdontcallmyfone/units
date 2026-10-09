// Changed by Hookwars: new file, the slot table's calls, answers and hook-data ranges; R20 Locked slot last.
//! Slots (docs/spec/01-token-slots.md): the accounts each called slot brings, the two calling
//! conventions (the Locked slot keeps the upstream one), the checks on every answer, and the
//! hook-data ranges with their epoch byte.

use anchor_lang::prelude::*;
use bordrless_hook::{
    discriminators, hook_signer_at, read_slot_answer, slot_flags, slot_kind, Allowed, Delta,
    HookReturn, Phase, SlotAllowed, SlotReturn, TokenHookArgs, TokenOp, TokenSlotArgs,
    TokenSlotOp, MAX_DELTAS, TOKEN_PREFIX_ACCOUNTS,
};

use crate::error::{slot_answer_error, TokenError};
use crate::hooks::HookCall;
use crate::state::{Mint, Slot};

/// Which operation calls the slots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotOp {
    /// A transfer (every subscribed slot).
    Transfer,
    /// A transfer out of a protocol vault (R16): the Locked slot only.
    ProtocolTransfer,
    /// A burn.
    Burn,
    /// A mint: the Locked slot only (R12).
    Mint,
}

/// Whether `slot` is called for `op`.
pub fn is_called(slot: &Slot, op: SlotOp) -> bool {
    if !slot.is_filled() || slot.kind == slot_kind::WAR {
        return false;
    }
    match op {
        SlotOp::Transfer => slot.flags & slot_flags::TRANSFER != 0,
        SlotOp::ProtocolTransfer => slot.is_locked() && slot.flags & slot_flags::TRANSFER != 0,
        SlotOp::Burn => slot.flags & slot_flags::BURN != 0,
        SlotOp::Mint => slot.is_locked() && slot.flags & slot_flags::MINT != 0,
    }
}

/// One called slot and its accounts.
pub struct SlotSlice<'a, 'info> {
    /// Index in the table.
    pub index: u8,
    /// The slot.
    pub slot: Slot,
    /// The call (program, signer, prefix, extras).
    pub call: HookCall<'a, 'info>,
}

/// Splits `remaining` into one slice per slot `op` calls: `[program, hook_signer, extras ...]`,
/// `extra_count` extras each. The program must be the slot's (`WrongHookProgram`), the signer this
/// program's `["hook-authority", program]` at the slot's bump (`BadHookSigner`), and the slices
/// must cover `remaining` exactly (`SlotAccountsMismatch`).
pub fn slices<'a, 'info>(
    mint: &Mint,
    op: SlotOp,
    prefix: [AccountInfo<'info>; 4],
    remaining: &'a [AccountInfo<'info>],
) -> Result<Vec<SlotSlice<'a, 'info>>> {
    let mut out = Vec::new();
    let mut at = 0usize;
    for (i, slot) in mint.active_slots().iter().enumerate() {
        if !is_called(slot, op) {
            continue;
        }
        let end = at + 2 + usize::from(slot.extra_count);
        require!(end <= remaining.len(), TokenError::SlotAccountsMismatch);
        let program = &remaining[at];
        let signer = &remaining[at + 1];
        require_keys_eq!(program.key(), slot.program, TokenError::WrongHookProgram);
        let expected = hook_signer_at(&crate::ID, &slot.program, slot.signer_bump)
            .ok_or(TokenError::BadHookSigner)?;
        require_keys_eq!(signer.key(), expected, TokenError::BadHookSigner);
        out.push(SlotSlice {
            index: i as u8,
            slot: *slot,
            call: HookCall {
                hook_program: program.clone(),
                hook_signer: signer.clone(),
                bump: slot.signer_bump,
                prefix: prefix.clone(),
                extras: &remaining[at + 2..end],
            },
        });
        at = end;
    }
    require!(at == remaining.len(), TokenError::SlotAccountsMismatch);
    Ok(out)
}

/// The bytes of `slot`'s range an item sees in `data`: zeros when the epoch byte is not the slot's
/// (stale or never stamped).
pub fn read_range(data: &[u8; 64], slot: &Slot) -> Vec<u8> {
    let len = usize::from(slot.data_len);
    if len == 0 {
        return Vec::new();
    }
    let off = usize::from(slot.data_offset);
    let range = &data[off..off + len];
    if range[0] == slot.data_epoch {
        range[1..].to_vec()
    } else {
        vec![0; len - 1]
    }
}

/// Writes an item's answered bytes into `slot`'s range of `data`: all zeros clear the range,
/// epoch byte included; anything else stamps the slot's epoch.
pub fn write_range(data: &mut [u8; 64], slot: &Slot, bytes: &[u8]) {
    let len = usize::from(slot.data_len);
    let off = usize::from(slot.data_offset);
    let range = &mut data[off..off + len];
    if bytes.iter().all(|b| *b == 0) {
        range.fill(0);
    } else {
        range[0] = slot.data_epoch;
        range[1..].copy_from_slice(bytes);
    }
}

/// Writes only the bytes of the Locked slot's range (R8) from a legacy answer.
pub fn write_locked(data: &mut [u8; 64], slot: &Slot, answer: &[u8; 64]) {
    let len = usize::from(slot.data_len);
    data[..len].copy_from_slice(&answer[..len]);
}

/// Whether a holding's data blocks its close on a slot mint: the Locked range non-zero, or an
/// item range at the slot's current epoch with any byte non-zero.
pub fn keeps_data(mint: &Mint, data: &[u8; 64]) -> bool {
    mint.active_slots().iter().any(|slot| {
        let len = usize::from(slot.data_len);
        if len == 0 {
            return false;
        }
        let off = usize::from(slot.data_offset);
        let range = &data[off..off + len];
        if slot.is_locked() {
            range.iter().any(|b| *b != 0)
        } else {
            range[0] == slot.data_epoch && range[1..].iter().any(|b| *b != 0)
        }
    })
}

/// The pre-state every called slot sees.
pub struct OpState {
    /// The operation (a protocol transfer is told `Transfer`).
    pub op: SlotOp,
    pub mint: Pubkey,
    pub source: Pubkey,
    pub destination: Pubkey,
    pub source_owner: Pubkey,
    pub destination_owner: Pubkey,
    pub authority: Pubkey,
    pub authority_is_delegate: bool,
    pub amount: u64,
    pub source_balance: u64,
    pub destination_balance: u64,
    pub decimals: u8,
    pub supply: u64,
    pub source_data: [u8; 64],
    pub destination_data: [u8; 64],
}

impl OpState {
    fn token_op(&self) -> TokenOp {
        match self.op {
            SlotOp::Transfer | SlotOp::ProtocolTransfer => TokenOp::Transfer,
            SlotOp::Burn => TokenOp::Burn,
            SlotOp::Mint => TokenOp::Mint,
        }
    }

    fn slot_op(&self) -> TokenSlotOp {
        match self.op {
            SlotOp::Burn => TokenSlotOp::Burn,
            _ => TokenSlotOp::Transfer,
        }
    }

    /// Legacy arguments (the Locked slot): the full 64 bytes. Hookwars R20: in a protocol transfer
    /// `authority` is [`crate::constants::PROTOCOL_TRANSFER_MARKER`], and in `Before` `delta` is
    /// the item slots' total cut on this operation (the Locked slot runs after them, R20).
    pub fn legacy_args(&self, phase: Phase, delta: u64) -> TokenHookArgs {
        let authority = if self.op == SlotOp::ProtocolTransfer {
            crate::constants::PROTOCOL_TRANSFER_MARKER
        } else {
            self.authority
        };
        TokenHookArgs {
            op: self.token_op(),
            phase,
            mint: self.mint,
            source: self.source,
            destination: self.destination,
            source_owner: self.source_owner,
            destination_owner: self.destination_owner,
            authority,
            authority_is_delegate: self.authority_is_delegate,
            amount: self.amount,
            delta,
            source_balance: self.source_balance,
            destination_balance: self.destination_balance,
            decimals: self.decimals,
            supply: self.supply,
            source_hook_data: self.source_data,
            destination_hook_data: self.destination_data,
        }
    }

    /// Slot arguments: only `slot`'s range.
    pub fn slot_args(
        &self,
        phase: Phase,
        index: u8,
        slot: &Slot,
        delta: u64,
        total_delta: u64,
    ) -> TokenSlotArgs {
        TokenSlotArgs {
            op: self.slot_op(),
            phase,
            slot: index,
            item: slot.item,
            mint: self.mint,
            source: self.source,
            destination: self.destination,
            source_owner: self.source_owner,
            destination_owner: self.destination_owner,
            authority: self.authority,
            authority_is_delegate: self.authority_is_delegate,
            amount: self.amount,
            delta,
            total_delta,
            source_balance: self.source_balance,
            destination_balance: self.destination_balance,
            decimals: self.decimals,
            supply: self.supply,
            source_data: read_range(&self.source_data, slot),
            destination_data: if self.op == SlotOp::Burn {
                vec![0; usize::from(slot.data_len).saturating_sub(1)]
            } else {
                read_range(&self.destination_data, slot)
            },
            payload: Vec::new(),
        }
    }
}

/// A slot's checked `before` answer.
pub struct SlotAnswer {
    /// Index of the slice in the slice list.
    pub slice: usize,
    /// Deltas (indices into that slice's prefix and extras).
    pub deltas: Vec<Delta>,
    /// The slot's cut.
    pub cut: u64,
    /// New source bytes: a full 64 (Locked) or the range (item).
    pub source_data: Option<Vec<u8>>,
    /// New destination bytes.
    pub destination_data: Option<Vec<u8>>,
}

fn before_discriminator(op: SlotOp) -> Option<[u8; 8]> {
    match op {
        SlotOp::Transfer | SlotOp::ProtocolTransfer => Some(discriminators::BEFORE_TRANSFER),
        SlotOp::Burn => Some(discriminators::BEFORE_BURN),
        SlotOp::Mint => Some(discriminators::BEFORE_MINT),
    }
}

fn after_discriminator(op: SlotOp) -> [u8; 8] {
    match op {
        SlotOp::Transfer | SlotOp::ProtocolTransfer => discriminators::AFTER_TRANSFER,
        SlotOp::Burn => discriminators::AFTER_BURN,
        SlotOp::Mint => discriminators::AFTER_MINT,
    }
}

fn before_flag(op: SlotOp) -> u16 {
    match op {
        SlotOp::Transfer | SlotOp::ProtocolTransfer => slot_flags::BEFORE_TRANSFER,
        SlotOp::Burn => slot_flags::BEFORE_BURN,
        SlotOp::Mint => slot_flags::BEFORE_MINT,
    }
}

fn after_flag(op: SlotOp) -> u16 {
    match op {
        SlotOp::Transfer | SlotOp::ProtocolTransfer => slot_flags::AFTER_TRANSFER,
        SlotOp::Burn => slot_flags::AFTER_BURN,
        SlotOp::Mint => slot_flags::AFTER_MINT,
    }
}

/// The largest cut `slot` may take of `amount`.
pub fn cut_bound(slot: &Slot, amount: u64) -> u64 {
    ((u128::from(amount) * u128::from(slot.bounds.max_cut_bps)) / 10_000) as u64
}

/// Runs every called slot's `before` callback on the same pre-state and checks each answer
/// (per slot, then across slots: at most `MAX_DELTAS` deltas, their sum at most the amount, no
/// target twice). Nothing is written here.
pub fn run_before(state: &OpState, slices: &[SlotSlice]) -> Result<Vec<SlotAnswer>> {
    let mut answers = Vec::new();
    let Some(disc) = before_discriminator(state.op) else {
        return Ok(answers);
    };
    let flag = before_flag(state.op);
    // Hookwars R20: item slots first, then the Locked slot, told the items' total cut as `delta`
    // so it can count what the destination really receives (the kit's eligible supply).
    let mut order: Vec<usize> = (0..slices.len())
        .filter(|n| !slices[*n].slot.is_locked())
        .collect();
    order.extend((0..slices.len()).filter(|n| slices[*n].slot.is_locked()));
    for n in order {
        let s = &slices[n];
        if s.slot.flags & flag == 0 {
            continue;
        }
        if s.slot.is_locked() {
            let allowed = Allowed::token(state.token_op(), Phase::Before, s.slot.flags);
            let items_cut = answers
                .iter()
                .try_fold(0u64, |t: u64, a: &SlotAnswer| t.checked_add(a.cut))
                .ok_or(TokenError::DeltaTooLarge)?;
            let args = state.legacy_args(Phase::Before, items_cut);
            if let Some((answer, sum)) = s.call.invoke_for_answer(disc, &args, allowed)? {
                let HookReturn {
                    deltas,
                    source_hook_data,
                    destination_hook_data,
                    ..
                } = answer;
                require!(
                    sum <= cut_bound(&s.slot, state.amount),
                    TokenError::SlotCutExceeded
                );
                answers.push(SlotAnswer {
                    slice: n,
                    deltas,
                    cut: sum,
                    source_data: source_hook_data.map(|d| d.to_vec()),
                    destination_data: destination_hook_data.map(|d| d.to_vec()),
                });
            }
        } else {
            let allowed = SlotAllowed::of(state.slot_op(), Phase::Before, s.slot.flags);
            let args = state.slot_args(Phase::Before, s.index, &s.slot, 0, 0);
            let mut data = Vec::with_capacity(8 + 400);
            data.extend_from_slice(&disc);
            args.serialize(&mut data)?;
            s.call.invoke_raw(data)?;
            let len = usize::from(s.slot.data_len).saturating_sub(1);
            if let Some((answer, cut)) = read_slot_answer(&s.slot.program, allowed, len)
                .map_err(slot_answer_error)?
            {
                let SlotReturn {
                    deltas,
                    source_data,
                    destination_data,
                } = answer;
                if let Some(d) = deltas.first() {
                    let i = usize::from(d.account);
                    require!(i >= TOKEN_PREFIX_ACCOUNTS, TokenError::WrongEquipVault);
                    let target = s
                        .call
                        .extras
                        .get(i - TOKEN_PREFIX_ACCOUNTS)
                        .ok_or(TokenError::WrongEquipVault)?;
                    require_keys_eq!(target.key(), s.slot.equip_vault, TokenError::WrongEquipVault);
                    require!(
                        cut <= cut_bound(&s.slot, state.amount),
                        TokenError::SlotCutExceeded
                    );
                }
                answers.push(SlotAnswer {
                    slice: n,
                    deltas,
                    cut,
                    source_data,
                    destination_data,
                });
            }
        }
    }
    // Across slots.
    let count: usize = answers.iter().map(|a| a.deltas.len()).sum();
    require!(count <= MAX_DELTAS, TokenError::TooManyDeltas);
    let total = answers
        .iter()
        .try_fold(0u64, |t, a| t.checked_add(a.cut))
        .ok_or(TokenError::DeltaTooLarge)?;
    require!(total <= state.amount, TokenError::DeltaTooLarge);
    let mut targets: Vec<Pubkey> = Vec::with_capacity(count);
    for a in &answers {
        for d in &a.deltas {
            let i = usize::from(d.account);
            require!(i >= TOKEN_PREFIX_ACCOUNTS, TokenError::InvalidDeltaAccount);
            let key = slices[a.slice]
                .call
                .extras
                .get(i - TOKEN_PREFIX_ACCOUNTS)
                .ok_or(TokenError::InvalidDeltaAccount)?
                .key();
            require!(!targets.contains(&key), TokenError::InvalidDeltaAccount);
            targets.push(key);
        }
    }
    Ok(answers)
}

/// Applies every answer's data to the pre-state bytes (each slot to its own range; the Locked
/// slot only inside its range). Answers the new source and destination bytes.
pub fn apply_data(
    state: &OpState,
    slices: &[SlotSlice],
    answers: &[SlotAnswer],
) -> ([u8; 64], [u8; 64]) {
    let mut source = state.source_data;
    let mut destination = state.destination_data;
    for a in answers {
        let slot = &slices[a.slice].slot;
        if slot.is_locked() {
            if let Some(d) = &a.source_data {
                let mut full = [0u8; 64];
                full.copy_from_slice(d);
                write_locked(&mut source, slot, &full);
            }
            if let Some(d) = &a.destination_data {
                let mut full = [0u8; 64];
                full.copy_from_slice(d);
                write_locked(&mut destination, slot, &full);
            }
        } else {
            if let Some(d) = &a.source_data {
                write_range(&mut source, slot, d);
            }
            if let Some(d) = &a.destination_data {
                write_range(&mut destination, slot, d);
            }
        }
    }
    (source, destination)
}

/// Runs every called slot's `after` callback on the post-state.
pub fn run_after(
    state: &OpState,
    slices: &[SlotSlice],
    answers: &[SlotAnswer],
    total: u64,
) -> Result<()> {
    let flag = after_flag(state.op);
    let disc = after_discriminator(state.op);
    for (n, s) in slices.iter().enumerate() {
        if s.slot.flags & flag == 0 {
            continue;
        }
        let own = answers
            .iter()
            .find(|a| a.slice == n)
            .map_or(0, |a| a.cut);
        if s.slot.is_locked() {
            s.call.invoke(disc, &state.legacy_args(Phase::After, total))?;
        } else {
            let args = state.slot_args(Phase::After, s.index, &s.slot, own, total);
            let mut data = Vec::with_capacity(8 + 400);
            data.extend_from_slice(&disc);
            args.serialize(&mut data)?;
            s.call.invoke_raw(data)?;
        }
    }
    Ok(())
}
