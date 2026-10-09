// Changed by Hookwars: new file (expansion, 10 section 8).
//! Coalition (id 43, kind Relation, no callbacks). A config item: equipping it names the coalition
//! a token belongs to and caps how much of its own war chest it may contribute. `hookwars_war`
//! reads it (`form_coalition`, `contribute`); the item never runs on a transfer or a swap.

use hookwars_common::Params;

/// What a Coalition item says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Coalition {
    /// The coalition id it joins (`["coalition", id]` under war).
    pub coalition_id: u32,
    /// The most of this token's chest it may contribute per contribution interval, in bps.
    pub max_contribution_bps: u16,
}

/// Reads a Coalition item's params (fields 0 and 1; `hookwars_common::validate` already refused
/// zero ids and contributions above 10,000 bps).
pub fn read(params: &Params) -> Coalition {
    Coalition {
        coalition_id: params[0],
        max_contribution_bps: params[1].min(10_000) as u16,
    }
}
