// Changed by Hookwars: new file (expansion, 10 section 11.3).
//! Rivalry (id 45, kind Relation, no callbacks). A config item naming one rival (its only target),
//! a start, a duration and a war budget. Spot only (R36): the budget is a ring-fenced share of the
//! token's own chest that war spends first against the rival; nothing ever moves between the two
//! chests because of the result. `hookwars_war` reads it; the item never runs on a transfer or a
//! swap.

use anchor_lang::prelude::Pubkey;
use hookwars_common::Params;

/// What a Rivalry item says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rivalry {
    /// Start, unix seconds (a `u32` field covers dates to 2106).
    pub starts_at: i64,
    /// End, unix seconds.
    pub ends_at: i64,
    /// War budget, bps of the token's own chest (ring-fenced, never paid to the rival).
    pub budget_bps: u16,
}

/// Reads a Rivalry item's params.
pub fn read(params: &Params) -> Rivalry {
    let starts_at = i64::from(params[0]);
    Rivalry {
        starts_at,
        ends_at: starts_at.saturating_add(i64::from(params[1])),
        budget_bps: params[2].min(10_000) as u16,
    }
}

/// Whether the rivalry with `rival` is live at `now` for an item aimed at `targets`.
pub fn live(params: &Params, targets: &[Pubkey], rival: &Pubkey, now: i64) -> bool {
    let r = read(params);
    targets.first() == Some(rival) && now >= r.starts_at && now < r.ends_at
}
