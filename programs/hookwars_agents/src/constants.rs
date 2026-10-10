// Changed by Hookwars: new file (09).
//! Seeds, structural caps and the fixed addresses the agents program checks against.

use anchor_lang::prelude::*;

/// Layout version of every account.
pub const VERSION: u8 = 1;

pub const CONFIG_SEED: &[u8] = b"agents-config";
pub const PASSPORT_SEED: &[u8] = b"passport";
pub const AGENT_KEY_SEED: &[u8] = b"agent-key";
pub const OPERATOR_SEED: &[u8] = b"operator";
pub const LINK_SEED: &[u8] = b"link";
pub const ATTEST_SEED: &[u8] = b"attest";
pub const ENDORSE_SEED: &[u8] = b"endorse";
pub const POLICY_SEED: &[u8] = b"policy";
pub const VAULT_SEED: &[u8] = b"agent-vault";
pub const BOND_SEED: &[u8] = b"bond";
/// One marker per bonded proposal, so a proposal is bonded at most once (09 8.2).
pub const BOND_MARK_SEED: &[u8] = b"bond-mark";
pub const BADGE_MINT_SEED: &[u8] = b"badge-mint";
pub const SIGNER_SEED: &[u8] = b"agents-signer";
pub const ARMORY_CALLER_SEED: &[u8] = b"armory-caller";
/// `["agents-caller"]` under the armory, war or items: the only signers `record` accepts.
pub const AGENTS_CALLER_SEED: &[u8] = b"agents-caller";

/// Structural caps on string lengths. They equal the token program's metadata bounds (a badge
/// carries the name and avatar URI as its metadata), and the config's params narrow them.
pub const NAME_CAP: usize = 32;
pub const URI_CAP: usize = 200;
pub const HANDLE_CAP: usize = 32;
/// Structural caps on list sizes (`VERIFIERS_MAX`, `POLICY_TARGETS_MAX`, `POLICY_MAX_TRACKED`: to
/// measure; provisional build values, like `MAX_MODULES`).
pub const VERIFIERS_CAP: usize = 8;
pub const TARGETS_CAP: usize = 8;
pub const TRACKED_CAP: usize = 4;

/// The badge symbol.
pub const BADGE_SYMBOL: &str = "AGENT";

/// `kinds` bits (09 2.2).
pub mod kind {
    pub const AUTHOR: u8 = 1;
    pub const DIPLOMAT: u8 = 1 << 1;
    pub const CRANKER: u8 = 1 << 2;
    pub const RAIDER: u8 = 1 << 3;
    pub const ALL: u8 = AUTHOR | DIPLOMAT | CRANKER | RAIDER;
}

/// Passport status.
pub mod status {
    pub const ACTIVE: u8 = 0;
    pub const PAUSED: u8 = 1;
    pub const RETIRED: u8 = 2;
}

/// Proof levels (09 section 5).
pub mod proof {
    pub const DECLARED: u8 = 0;
    pub const LINKED: u8 = 1;
    pub const ATTESTED: u8 = 2;
}

/// Link platforms (09 2.5).
pub mod platform {
    pub const X: u8 = 0;
    pub const TELEGRAM: u8 = 1;
    pub const GITHUB: u8 = 2;
    pub const FARCASTER: u8 = 3;
    pub const WEB: u8 = 4;
    pub const MAX: u8 = WEB;
}

/// TEE kinds (09 2.6): Intel TDX 0, AMD SEV-SNP 1, Intel SGX 2, AWS Nitro 3.
pub const TEE_KIND_MAX: u8 = 3;

/// `record` kinds (09 6.1 and 6.3).
pub mod record_kind {
    pub const ITEMS_AUTHORED: u8 = 0;
    pub const ITEMS_EQUIPPED: u8 = 1;
    pub const ITEMS_FORGED: u8 = 2;
    /// `value`: the amount claimed when the cut mint is bridged SOL, else 0.
    pub const ROYALTY_CLAIM: u8 = 3;
    /// `value`: the value the crank's bounty was computed on.
    pub const CRANK: u8 = 4;
    /// `value`: the bounty paid.
    pub const BOUNTY: u8 = 5;
    pub const LOOT_REVEAL: u8 = 6;
    pub const MAX: u8 = LOOT_REVEAL;
}

/// Bond status (09 2.9).
pub mod bond_status {
    pub const POSTED: u8 = 0;
    pub const RATIFIED: u8 = 1;
    pub const REJECTED: u8 = 2;
    pub const RETURNED: u8 = 3;
    pub const HELD: u8 = 4;
    pub const BROKEN: u8 = 5;
}

/// The ed25519 signature verification program.
pub const ED25519_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("Ed25519SigVerify111111111111111111111111111");
/// The instructions sysvar.
pub const INSTRUCTIONS_SYSVAR_ID: Pubkey =
    Pubkey::from_str_const("Sysvar1nstructions1111111111111111111111111");
/// The upgradeable loader.
pub const BPF_LOADER_UPGRADEABLE_ID: Pubkey =
    Pubkey::from_str_const("BPFLoaderUpgradeab1e11111111111111111111111");
/// Loaders a policy vault may never call.
pub const LOADERS: [Pubkey; 4] = [
    BPF_LOADER_UPGRADEABLE_ID,
    Pubkey::from_str_const("BPFLoader2111111111111111111111111111111111"),
    Pubkey::from_str_const("BPFLoader1111111111111111111111111111111111"),
    Pubkey::from_str_const("LoaderV411111111111111111111111111111111111"),
];

/// `["agents-caller"]` under the armory, war and items (checked against their derivation in the
/// unit tests).
pub const RECORDERS: [Pubkey; 3] = [
    Pubkey::from_str_const("GBAdLKHQNAKDWwW5S3k9qrEHiAeVWtZA8VC11NqwTVGU"),
    Pubkey::from_str_const("Bva1RRSNtZAsFi4piXTYhsXYghYzF7LpufoReiDqZSBx"),
    Pubkey::from_str_const("BzwMrjS1qBHvFAZk7JDoj8xBNjvPsJPDfAVRbiY3kFS4"),
];

/// The statement an agent key signs to link a social account (09 5.2). `nonce` is the passport's
/// `link_nonce` at the time of the link (review 3 L-5: a statement is good for one link only).
pub fn link_statement(passport: &Pubkey, platform: u8, handle: &str, nonce: u32) -> String {
    format!("units agent link v2\npassport: {passport}\nplatform: {platform}\nhandle: {handle}\nnonce: {nonce}")
}

/// `sha256(agent_key || passport || nonce)` zero-padded to 64 bytes (09 5.3).
pub fn report_data(agent_key: &Pubkey, passport: &Pubkey, nonce: &[u8; 32]) -> [u8; 64] {
    let h = solana_sha256_hasher::hashv(&[agent_key.as_ref(), passport.as_ref(), nonce]);
    let mut out = [0u8; 64];
    out[..32].copy_from_slice(h.as_ref());
    out
}

/// `sha256(bytes)`.
pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    let h = solana_sha256_hasher::hashv(&[bytes]);
    let mut out = [0u8; 32];
    out.copy_from_slice(h.as_ref());
    out
}

/// PDAs.
pub mod pda {
    use super::*;

    pub fn config() -> (Pubkey, u8) {
        Pubkey::find_program_address(&[CONFIG_SEED], &crate::ID)
    }
    pub fn passport(operator: &Pubkey, index: u32) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[PASSPORT_SEED, operator.as_ref(), &index.to_le_bytes()], &crate::ID)
    }
    pub fn agent_key(key: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[AGENT_KEY_SEED, key.as_ref()], &crate::ID)
    }
    pub fn operator(operator: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[OPERATOR_SEED, operator.as_ref()], &crate::ID)
    }
    pub fn link(passport: &Pubkey, platform: u8) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[LINK_SEED, passport.as_ref(), &[platform]], &crate::ID)
    }
    pub fn attestation(passport: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[ATTEST_SEED, passport.as_ref()], &crate::ID)
    }
    pub fn endorsement(attestation: &Pubkey, verifier: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[ENDORSE_SEED, attestation.as_ref(), verifier.as_ref()], &crate::ID)
    }
    pub fn policy(passport: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[POLICY_SEED, passport.as_ref()], &crate::ID)
    }
    pub fn vault(passport: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[VAULT_SEED, passport.as_ref()], &crate::ID)
    }
    pub fn bond(passport: &Pubkey, proposal_a: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[BOND_SEED, passport.as_ref(), proposal_a.as_ref()], &crate::ID)
    }
    pub fn bond_mark(proposal: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[BOND_MARK_SEED, proposal.as_ref()], &crate::ID)
    }
    pub fn badge_mint(passport: &Pubkey, generation: u8) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[BADGE_MINT_SEED, passport.as_ref(), &[generation]], &crate::ID)
    }
    pub fn signer() -> (Pubkey, u8) {
        Pubkey::find_program_address(&[SIGNER_SEED], &crate::ID)
    }
    /// `["armory-caller", mint]` under this program: signs the armory's `equip_launch` for badges.
    pub fn armory_caller(mint: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[ARMORY_CALLER_SEED, mint.as_ref()], &crate::ID)
    }
    /// `["treaty-inbox", mint]` under the war program.
    pub fn treaty_inbox(mint: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(&[b"treaty-inbox", mint.as_ref()], &hookwars_common::ids::WAR_ID).0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hookwars_common::ids;

    #[test]
    fn fixed_addresses_match_their_derivations() {
        assert_eq!(pda::signer().0, ids::AGENTS_SIGNER);
        assert_eq!(ids::AGENTS_ID, crate::ID);
        for (i, program) in [ids::ARMORY_ID, ids::WAR_ID, ids::ITEMS_ID].iter().enumerate() {
            assert_eq!(
                Pubkey::find_program_address(&[AGENTS_CALLER_SEED], program).0,
                RECORDERS[i]
            );
        }
    }

    #[test]
    fn report_data_is_the_hash_zero_padded() {
        let (k, p, n) = (Pubkey::new_unique(), Pubkey::new_unique(), [7u8; 32]);
        let r = report_data(&k, &p, &n);
        assert_eq!(r[32..], [0u8; 32]);
        assert_ne!(r[..32], [0u8; 32]);
        assert_ne!(report_data(&p, &k, &n), r);
    }
}
