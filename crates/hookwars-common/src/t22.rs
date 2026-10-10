// Changed by Hookwars: new file (gating, docs/spec/18-gating.md part C): the Token-2022 bytes the
// gate and the market read.
//! Token-2022 reads: base mint and account fields, the `TransferHook` mint extension and the
//! `TransferHookAccount` account extension. Layouts are the SPL Token-2022 program's.

use anchor_lang::prelude::*;

/// SPL Token-2022.
pub const TOKEN_2022_ID: Pubkey = Pubkey::from_str_const("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

/// Base account length (shared by `Mint` padding and `Account`).
pub const BASE_ACCOUNT_LEN: usize = 165;
/// Base mint length.
pub const BASE_MINT_LEN: usize = 82;
/// Account type byte after the base: 1 mint, 2 account.
pub const ACCOUNT_TYPE_MINT: u8 = 1;
pub const ACCOUNT_TYPE_ACCOUNT: u8 = 2;
/// Extension types.
pub const EXT_TRANSFER_HOOK: u16 = 14;
pub const EXT_TRANSFER_HOOK_ACCOUNT: u16 = 15;

/// The value of extension `ty` in an extended mint or account, if present.
pub fn extension(data: &[u8], account_type: u8, ty: u16) -> Option<&[u8]> {
    if data.len() <= BASE_ACCOUNT_LEN || data[BASE_ACCOUNT_LEN] != account_type {
        return None;
    }
    let mut at = BASE_ACCOUNT_LEN + 1;
    while at + 4 <= data.len() {
        let t = u16::from_le_bytes([data[at], data[at + 1]]);
        let len = usize::from(u16::from_le_bytes([data[at + 2], data[at + 3]]));
        let start = at + 4;
        let end = start.checked_add(len)?;
        if end > data.len() {
            return None;
        }
        if t == 0 {
            return None;
        }
        if t == ty {
            return Some(&data[start..end]);
        }
        at = end;
    }
    None
}

/// A mint's transfer-hook authority and program, when the mint is a Token-2022 mint with the
/// `TransferHook` extension.
pub fn transfer_hook(info: &AccountInfo) -> Option<(Option<Pubkey>, Option<Pubkey>)> {
    if *info.owner != TOKEN_2022_ID {
        return None;
    }
    let d = info.try_borrow_data().ok()?;
    let v = extension(&d, ACCOUNT_TYPE_MINT, EXT_TRANSFER_HOOK)?;
    if v.len() < 64 {
        return None;
    }
    let nz = |b: &[u8]| {
        let k = Pubkey::try_from(b).ok()?;
        (k != Pubkey::default()).then_some(k)
    };
    Some((nz(&v[..32]), nz(&v[32..64])))
}

/// Whether `mint` is a Token-2022 mint whose transfer hook program is `program`.
pub fn hooked_by(info: &AccountInfo, program: &Pubkey) -> bool {
    matches!(transfer_hook(info), Some((_, Some(p))) if p == *program)
}

/// A Token-2022 mint's supply and decimals.
pub fn mint_supply(info: &AccountInfo) -> Option<(u64, u8)> {
    if *info.owner != TOKEN_2022_ID {
        return None;
    }
    let d = info.try_borrow_data().ok()?;
    if d.len() < BASE_MINT_LEN {
        return None;
    }
    Some((u64::from_le_bytes(d[36..44].try_into().ok()?), d[44]))
}

/// A Token-2022 token account's base fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokenAccount {
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub amount: u64,
    /// The `TransferHookAccount` extension's `transferring` flag (set by Token-2022 only while it
    /// calls the hook during a transfer).
    pub transferring: bool,
}

/// Reads a Token-2022 token account.
pub fn token_account(info: &AccountInfo) -> Option<TokenAccount> {
    if *info.owner != TOKEN_2022_ID {
        return None;
    }
    let d = info.try_borrow_data().ok()?;
    if d.len() < BASE_ACCOUNT_LEN {
        return None;
    }
    let transferring = extension(&d, ACCOUNT_TYPE_ACCOUNT, EXT_TRANSFER_HOOK_ACCOUNT)
        .and_then(|v| v.first().copied())
        .unwrap_or(0)
        != 0;
    Some(TokenAccount {
        mint: Pubkey::try_from(&d[..32]).ok()?,
        owner: Pubkey::try_from(&d[32..64]).ok()?,
        amount: u64::from_le_bytes(d[64..72].try_into().ok()?),
        transferring,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions_are_found_by_type() {
        let mut d = vec![0u8; BASE_ACCOUNT_LEN];
        d.push(ACCOUNT_TYPE_ACCOUNT);
        d.extend_from_slice(&7u16.to_le_bytes());
        d.extend_from_slice(&0u16.to_le_bytes());
        d.extend_from_slice(&EXT_TRANSFER_HOOK_ACCOUNT.to_le_bytes());
        d.extend_from_slice(&1u16.to_le_bytes());
        d.push(1);
        assert_eq!(extension(&d, ACCOUNT_TYPE_ACCOUNT, EXT_TRANSFER_HOOK_ACCOUNT), Some(&[1u8][..]));
        assert_eq!(extension(&d, ACCOUNT_TYPE_MINT, EXT_TRANSFER_HOOK_ACCOUNT), None);
        assert_eq!(extension(&d, ACCOUNT_TYPE_ACCOUNT, EXT_TRANSFER_HOOK), None);
    }
}
