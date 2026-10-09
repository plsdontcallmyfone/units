// Changed by Hookwars: new program.
//! `hookwars_war`: each token's war chest, spent only through the instructions here
//! (docs/spec/05-war.md). Every spend is permissionless, checks on chain that it is due, is capped
//! per call and per interval, and pays its sender a crank bounty. Spot only: a siege is a spot buy,
//! a counter-strike a spot buy and a burn, a raze a spot sell; bounties and loot pay for raid
//! activity already recorded; the season prize is a share of protocol fees that already exist.
//!
//! Layout: [`state`], [`instructions`], [`common`] (calling other programs, the War orders, the
//! Raid range), [`foreign`] (accounts of programs not in this branch, decoded from the spec),
//! [`oracle`] (the randomness adapter interface), [`events`], [`error`], [`client`].

#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

pub mod client;
pub mod common;
pub mod constants;
pub mod error;
pub mod events;
pub mod foreign;
pub mod instructions;
pub mod oracle;
pub mod state;

pub use instructions::*;
use state::{LootEntry, WarParams};

declare_id!("5vJnBvr33jpsfYxMY2pvNf6tF9tkj8eaZ6goFtByUWA2");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "Hookwars war",
    project_url: "https://github.com/BordrlessDex/bordrless-programs",
    contacts: "link:https://github.com/BordrlessDex/bordrless-programs/security/advisories/new",
    policy: "https://github.com/BordrlessDex/bordrless-programs/blob/main/SECURITY.md",
    source_code: "https://github.com/BordrlessDex/bordrless-programs"
}

#[program]
pub mod hookwars_war {
    use super::*;

    // ---- config and timelock (section 11)

    /// Creates the config, once, by this program's upgrade authority.
    pub fn init_config(ctx: Context<InitConfig>, args: ConfigArgs) -> Result<()> {
        instructions::admin::process_init_config(ctx, args)
    }

    /// The admin proposes a config change, applied after `admin_timelock_secs`.
    pub fn propose_config(ctx: Context<ProposeConfig>, args: ConfigArgs) -> Result<()> {
        instructions::admin::process_propose_config(ctx, args)
    }

    /// Anyone applies a proposed config change after its `eta`.
    pub fn apply_config(ctx: Context<ApplyConfig>) -> Result<()> {
        instructions::admin::process_apply_config(ctx)
    }

    /// The admin cancels a pending config change, season or loot table before it applies.
    pub fn cancel_pending(ctx: Context<CancelPending>, what: u8, season: u32) -> Result<()> {
        instructions::admin::process_cancel_pending(ctx, what, season)
    }

    /// The admin proposes a season (weights and dates), behind the timelock.
    pub fn propose_season(ctx: Context<ProposeSeason>, args: SeasonArgs) -> Result<()> {
        instructions::admin::process_propose_season(ctx, args)
    }

    /// The admin proposes a season's loot table, behind the timelock.
    pub fn propose_loot_table<'info>(
        ctx: Context<'info, ProposeLootTable<'info>>,
        season: u32,
        entries: Vec<LootEntry>,
    ) -> Result<()> {
        instructions::admin::process_propose_loot_table(ctx, season, entries)
    }

    // ---- setup and funding (sections 4, 5)

    /// A token's war state, chest and treaty inbox.
    pub fn init_war(ctx: Context<InitWar>) -> Result<()> {
        instructions::setup::process_init_war(ctx)
    }

    /// Counts what reached the chest since the last write.
    pub fn record_funding(ctx: Context<RecordFunding>) -> Result<()> {
        instructions::setup::process_record_funding(ctx)
    }

    // ---- attack and defense (section 6)

    /// The chest spot-buys a rival that has been raided into this token enough.
    pub fn siege<'info>(ctx: Context<'info, Siege<'info>>, args: SliceArgs) -> Result<()> {
        instructions::attack::process_siege(ctx, args)
    }

    /// After a fall, the chest buys back its own token and burns it.
    pub fn counter_strike<'info>(
        ctx: Context<'info, CounterStrike<'info>>,
        args: SliceArgs,
    ) -> Result<()> {
        instructions::attack::process_counter_strike(ctx, args)
    }

    /// Sells captured rival tokens back into the rival's pool, slowly.
    pub fn raze<'info>(ctx: Context<'info, Raze<'info>>, args: SliceArgs) -> Result<()> {
        instructions::attack::process_raze(ctx, args)
    }

    /// Peace: captured rival tokens to the rival's chest, under a treaty both equip.
    pub fn return_captured<'info>(
        ctx: Context<'info, ReturnCaptured<'info>>,
        args: SliceArgs,
    ) -> Result<()> {
        instructions::attack::process_return_captured(ctx, args)
    }

    /// Streams what treaties paid this token to its holders through the kit.
    pub fn share_treaty_inflow<'info>(ctx: Context<'info, ShareTreatyInflow<'info>>) -> Result<()> {
        instructions::treaty::process_share_treaty_inflow(ctx)
    }

    /// Adds the season time treaties held to the score's counter.
    pub fn accrue_treaty_time<'info>(ctx: Context<'info, AccrueTreatyTime<'info>>) -> Result<()> {
        instructions::treaty::process_accrue_treaty_time(ctx)
    }

    // ---- bounties, loot and quests (sections 7 to 9)

    /// A holder turns raid points into SOL from the chest.
    pub fn claim_bounty<'info>(
        ctx: Context<'info, ClaimBounty<'info>>,
        raid_slot: u8,
    ) -> Result<()> {
        instructions::bounty::process_claim_bounty(ctx, raid_slot)
    }

    /// Spends a loot ticket and asks the randomness adapter.
    pub fn roll<'info>(ctx: Context<'info, Roll<'info>>, nonce: u64, raid_slot: u8) -> Result<()> {
        instructions::loot::process_roll(ctx, nonce, raid_slot)
    }

    /// Reads the randomness and asks the armory to mint the item.
    pub fn reveal<'info>(ctx: Context<'info, Reveal<'info>>) -> Result<()> {
        instructions::loot::process_reveal(ctx)
    }

    /// The owner closes an expired roll (no ticket refund).
    pub fn cancel_roll(ctx: Context<CancelRoll>) -> Result<()> {
        instructions::loot::process_cancel_roll(ctx)
    }

    /// Claims a quest's loot ticket, once per period.
    pub fn claim_quest<'info>(
        ctx: Context<'info, ClaimQuest<'info>>,
        quest_id: u8,
        period: u32,
        raid_slot: u8,
    ) -> Result<()> {
        instructions::quests::process_claim_quest(ctx, quest_id, period, raid_slot)
    }

    /// Returns a quest mark's rent once its season is finalized.
    pub fn close_quest_mark(ctx: Context<CloseQuestMark>) -> Result<()> {
        instructions::quests::process_close_quest_mark(ctx)
    }

    // ---- seasons (section 10)

    /// Opens the next season.
    pub fn open_season(ctx: Context<OpenSeason>) -> Result<()> {
        instructions::seasons::process_open_season(ctx)
    }

    /// Submits a token as the season's leader, or challenges the leader.
    pub fn submit_candidate(ctx: Context<SubmitCandidate>, number: u32) -> Result<()> {
        instructions::seasons::process_submit_candidate(ctx, number)
    }

    /// Finalizes a season after its challenge window.
    pub fn finalize_season(ctx: Context<FinalizeSeason>, number: u32) -> Result<()> {
        instructions::seasons::process_finalize_season(ctx, number)
    }

    /// Splits the prize vault between the last winner's chest and the treasury.
    pub fn split_protocol_fees<'info>(
        ctx: Context<'info, SplitProtocolFees<'info>>,
    ) -> Result<()> {
        instructions::seasons::process_split_protocol_fees(ctx)
    }
}

/// Re-exported for clients: the parameters type.
pub type Params = WarParams;
