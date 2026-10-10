// Changed by Hookwars: new file (gating, docs/spec/18-gating.md part C): the external gate's ids,
// seeds and the reads the items program and the market make of it.
//! The external gate (`hookwars_gate`): Token-2022 mints of any protocol run units items as their
//! transfer hook while they hold a live licence (or own the item).

use anchor_lang::prelude::*;

/// `hookwars_gate`.
pub const GATE_ID: Pubkey = Pubkey::from_str_const("CKMGqUdYzVyTr4myZsXo8rEMeBA5QoqdABinbZfvh8Dj");
/// `["items-signer"]` under the gate: the only signer the items program's `gate_before` accepts.
pub const GATE_ITEMS_SIGNER: Pubkey = Pubkey::from_str_const("FXFoynetLBuu9WsePfn7gNapWXyW4o3USBfWHDWF42aY");

/// Seeds under the gate.
pub mod seeds {
    pub const CONFIG: &[u8] = b"gate-config";
    pub const MINT_GATE: &[u8] = b"gate-mint";
    pub const HOLDER: &[u8] = b"gate-holder";
    pub const VAULT: &[u8] = b"gate-vault";
    pub const ITEMS_SIGNER: &[u8] = b"items-signer";
    /// The SPL transfer-hook interface's validation account.
    pub const EXTRA_METAS: &[u8] = b"extra-account-metas";
}

/// `MintGate`'s discriminator (`sha256("account:MintGate")[..8]`).
pub const MINT_GATE_DISC: [u8; 8] = [155, 24, 147, 223, 89, 73, 35, 36];
/// `MintGate` layout: discriminator, version, bump, mint (10..42), authority (42..74), venue (74..106).
pub const MINT_GATE_MINT: usize = 10;
pub const MINT_GATE_VENUE: usize = 74;

/// `["gate-mint", mint]`.
pub fn mint_gate(mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::MINT_GATE, mint.as_ref()], &GATE_ID)
}

/// `["gate-holder", mint, owner]`.
pub fn holder(mint: &Pubkey, owner: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::HOLDER, mint.as_ref(), owner.as_ref()], &GATE_ID)
}

/// `["gate-vault", mint]`.
pub fn vault(mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::VAULT, mint.as_ref()], &GATE_ID)
}

/// `["extra-account-metas", mint]`.
pub fn extra_metas(mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::EXTRA_METAS, mint.as_ref()], &GATE_ID)
}

/// The venue (the AMM pool authority whose transfers count as buys and sells) the mint's
/// `MintGate` names, when `info` is that `MintGate`. Only the gate creates accounts it owns with
/// this discriminator, one per mint at its PDA, so owner, discriminator and mint identify it.
pub fn read_venue(info: &AccountInfo, mint: &Pubkey) -> Option<Pubkey> {
    if *info.owner != GATE_ID {
        return None;
    }
    let d = info.try_borrow_data().ok()?;
    if d.len() < MINT_GATE_VENUE + 32 || d[..8] != MINT_GATE_DISC || d[MINT_GATE_MINT..MINT_GATE_MINT + 32] != mint.as_ref()[..] {
        return None;
    }
    let v = Pubkey::try_from(&d[MINT_GATE_VENUE..MINT_GATE_VENUE + 32]).ok()?;
    (v != Pubkey::default()).then_some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_items_signer_is_the_gates_pda() {
        let (k, _) = Pubkey::find_program_address(&[seeds::ITEMS_SIGNER], &GATE_ID);
        assert_eq!(k, GATE_ITEMS_SIGNER);
    }
}
