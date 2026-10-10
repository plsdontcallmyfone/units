//! `hooklab.json`: what a template says about itself, and the static checks on it. Every bound the
//! property suite enforces comes from here; the lab never assumes a number the author did not
//! declare (the protocol's own limits come from `bordrless_hook` and `hookwars_common`).

use std::str::FromStr;

use anchor_lang::prelude::Pubkey;
use bordrless_hook::{slot_flags, HOOK_DATA_LEN};
use hookwars_common::{kind, PARAM_FIELDS};
use serde::{Deserialize, Serialize};

/// The manifest schema this lab reads.
pub const SCHEMA: u32 = 1;
/// Longest template name the armory stores (02 section 3.1).
pub const MAX_NAME: usize = 32;

/// Parameter schema.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Params {
    pub field_count: u8,
    pub names: Vec<String>,
    pub field_min: Vec<u32>,
    pub field_max: Vec<u32>,
}

/// What `register_template` gets besides the schema.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Registry {
    pub open_authoring: bool,
    pub loot_enabled: bool,
    pub forge_enabled: bool,
    pub max_level: u8,
    pub loot_royalty_bps: u16,
    pub max_targets: u8,
}

/// `hooklab.json`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: u32,
    pub name: String,
    pub program_id: String,
    pub kind: String,
    pub callbacks: Vec<String>,
    #[serde(default)]
    pub pool_callbacks: Vec<String>,
    pub max_cut_transfer_bps: u16,
    pub may_refuse: bool,
    #[serde(default)]
    pub may_burn: bool,
    pub data_bytes: u8,
    #[serde(default)]
    pub answers_touch: bool,
    pub max_cu_per_call: u64,
    pub extras: Vec<String>,
    pub params: Params,
    pub registry: Registry,
}

/// The callbacks an item slot may run, and the flag bit of each.
pub const CALLBACKS: [(&str, u16); 5] = [
    ("before_transfer", slot_flags::BEFORE_TRANSFER),
    ("after_transfer", slot_flags::AFTER_TRANSFER),
    ("before_burn", slot_flags::BEFORE_BURN),
    ("after_burn", slot_flags::AFTER_BURN),
    ("on_touch", slot_flags::ANSWERS_TOUCH),
];

/// The extras a v1 template may name, in the order it lists them.
pub const EXTRAS: [&str; 2] = ["item", "equip_vault"];

impl Manifest {
    /// Parses `hooklab.json`.
    pub fn parse(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|e| format!("hooklab.json: {e}"))
    }

    /// The template's program id.
    pub fn program(&self) -> Result<Pubkey, String> {
        Pubkey::from_str(&self.program_id).map_err(|_| "program_id is not a public key".to_string())
    }

    /// The kind byte (`hookwars_common::kind`).
    pub fn kind_byte(&self) -> Option<u8> {
        Some(match self.kind.as_str() {
            "fee" => kind::FEE,
            "reward" => kind::REWARD,
            "defense" => kind::DEFENSE,
            "relation" => kind::RELATION,
            "pool" => kind::POOL,
            "war" => kind::WAR,
            _ => return None,
        })
    }

    /// The slot flags an equip of this template sets.
    pub fn flags(&self) -> u16 {
        let mut f = 0;
        for (name, bit) in CALLBACKS {
            if self.callbacks.iter().any(|c| c == name) {
                f |= bit;
            }
        }
        if self.max_cut_transfer_bps > 0 {
            f |= slot_flags::TRANSFER_RETURNS_DELTA;
        }
        if self.data_bytes > 0 {
            f |= slot_flags::WRITES_HOOK_DATA;
        }
        f
    }

    /// Whether the template cuts transfers into its equip vault.
    pub fn cuts(&self) -> bool {
        self.max_cut_transfer_bps > 0
    }

    /// The slot's `data_len` (the template's bytes plus the token program's epoch byte).
    pub fn data_len(&self) -> u8 {
        if self.data_bytes == 0 {
            0
        } else {
            self.data_bytes + 1
        }
    }

    /// The 11-wide field bounds `register_template` takes.
    pub fn field_bounds(&self) -> ([u32; PARAM_FIELDS], [u32; PARAM_FIELDS]) {
        let mut lo = [0u32; PARAM_FIELDS];
        let mut hi = [0u32; PARAM_FIELDS];
        for i in 0..usize::from(self.params.field_count).min(PARAM_FIELDS) {
            lo[i] = self.params.field_min.get(i).copied().unwrap_or(0);
            hi[i] = self.params.field_max.get(i).copied().unwrap_or(0);
        }
        (lo, hi)
    }

    /// Static checks. Empty means the manifest is consistent with itself and the protocol.
    pub fn check(&self) -> Vec<String> {
        let mut e = Vec::new();
        if self.schema != SCHEMA {
            e.push(format!("schema {} (this lab reads {SCHEMA})", self.schema));
        }
        if self.name.is_empty() || self.name.len() > MAX_NAME {
            e.push(format!("name must be 1 to {MAX_NAME} bytes"));
        }
        if self.name.chars().any(|c| c.is_control()) {
            e.push("name has control characters".into());
        }
        match self.program() {
            Ok(p) => {
                if protocol_ids().contains(&p) {
                    e.push("program_id is a protocol program".into());
                }
            }
            Err(m) => e.push(m),
        }
        match self.kind.as_str() {
            "pool" => e.push("kind pool: the pool-side ABI suite is not in this lab yet (v1 checks token-side templates)".into()),
            "war" => e.push("kind war: war items are never called; nothing for this lab to run".into()),
            _ if self.kind_byte().is_none() => e.push(format!("unknown kind {}", self.kind)),
            _ => {}
        }
        if self.callbacks.is_empty() {
            e.push("no callbacks".into());
        }
        for c in &self.callbacks {
            if !CALLBACKS.iter().any(|(n, _)| n == c) {
                e.push(format!("unknown callback {c}"));
            }
        }
        if !self.pool_callbacks.is_empty() {
            e.push("pool_callbacks: not in this lab yet".into());
        }
        if self.max_cut_transfer_bps > 10_000 {
            e.push("max_cut_transfer_bps over 10,000".into());
        }
        if self.cuts() && !self.callbacks.iter().any(|c| c == "before_transfer") {
            e.push("a cut needs before_transfer (only it may answer deltas)".into());
        }
        if usize::from(self.data_len()) > HOOK_DATA_LEN {
            e.push(format!("data_bytes {} does not fit a holding's {HOOK_DATA_LEN} bytes", self.data_bytes));
        }
        if self.answers_touch {
            if self.data_bytes == 0 {
                e.push("answers_touch needs a data range".into());
            }
            if !matches!(self.kind.as_str(), "reward" | "defense" | "relation" | "pool") {
                e.push("answers_touch: only reward, defense, relation and pool slots answer touch".into());
            }
            if !self.callbacks.iter().any(|c| c == "on_touch") {
                e.push("answers_touch without on_touch in callbacks".into());
            }
        } else if self.callbacks.iter().any(|c| c == "on_touch") {
            e.push("on_touch in callbacks but answers_touch is false".into());
        }
        if self.max_cu_per_call == 0 {
            e.push("max_cu_per_call must be declared".into());
        }
        for x in &self.extras {
            if !EXTRAS.contains(&x.as_str()) {
                e.push(format!("unknown extra {x}"));
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        for x in &self.extras {
            if !seen.insert(x) {
                e.push(format!("extra {x} listed twice"));
            }
        }
        if self.cuts() != self.extras.iter().any(|x| x == "equip_vault") {
            e.push("equip_vault is an extra exactly when the template cuts".into());
        }
        let p = &self.params;
        let n = usize::from(p.field_count);
        if n > PARAM_FIELDS {
            e.push(format!("field_count over {PARAM_FIELDS}"));
        }
        if p.names.len() != n || p.field_min.len() != n || p.field_max.len() != n {
            e.push("params: names, field_min and field_max must each have field_count entries".into());
        }
        for (i, (lo, hi)) in p.field_min.iter().zip(&p.field_max).enumerate() {
            if lo > hi {
                e.push(format!("params field {i}: min over max"));
            }
        }
        let r = &self.registry;
        if r.max_level == 0 {
            e.push("registry.max_level must be at least 1".into());
        }
        if r.loot_royalty_bps > 10_000 {
            e.push("registry.loot_royalty_bps over 10,000".into());
        }
        if r.max_targets != 0 && self.kind != "relation" {
            e.push("registry.max_targets is for relation templates".into());
        }
        e
    }
}

/// Program ids a template may never be (the armory's own list, 02 section 3.2).
pub fn protocol_ids() -> Vec<Pubkey> {
    use hookwars_common::ids;
    vec![
        ids::TOKEN_ID,
        ids::SWAP_ID,
        ids::LAUNCH_ID,
        ids::KIT_ID,
        ids::BRIDGE_ID,
        ids::COMPANION_ID,
        ids::ARMORY_ID,
        ids::WAR_ID,
        ids::ITEMS_ID,
        anchor_lang::system_program::ID,
        Pubkey::default(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example() -> Manifest {
        Manifest::parse(include_str!("../../../examples/hook-template/hooklab.json")).unwrap()
    }

    #[test]
    fn the_example_manifest_is_clean() {
        let m = example();
        assert_eq!(m.check(), Vec::<String>::new());
        assert_eq!(m.data_len(), 6);
        assert!(m.cuts());
        assert_eq!(
            m.flags(),
            slot_flags::BEFORE_TRANSFER | slot_flags::TRANSFER_RETURNS_DELTA | slot_flags::WRITES_HOOK_DATA
        );
    }

    #[test]
    fn inconsistent_manifests_are_named() {
        let mut m = example();
        m.extras = vec!["item".into()];
        assert!(m.check().iter().any(|e| e.contains("equip_vault")));
        let mut m = example();
        m.data_bytes = 64;
        assert!(m.check().iter().any(|e| e.contains("does not fit")));
        let mut m = example();
        m.kind = "pool".into();
        assert!(m.check().iter().any(|e| e.contains("pool-side")));
        let mut m = example();
        m.program_id = bordrless_token::ID.to_string();
        assert!(m.check().iter().any(|e| e.contains("protocol program")));
        let mut m = example();
        m.params.field_min = vec![600];
        assert!(m.check().iter().any(|e| e.contains("min over max")));
        assert!(Manifest::parse(r#"{"schema":1,"bogus":true}"#).is_err());
    }
}
