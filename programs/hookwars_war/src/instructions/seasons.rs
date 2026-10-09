//! Seasons (05 section 10): opening, king of the hill in O(1) per call, and the prize, a share of
//! protocol fees that already exist (R14).

use anchor_lang::prelude::*;
use bordrless_token::client as token_client;

use crate::common::*;
use crate::constants::*;
use crate::error::WarError;
use crate::events::*;
use crate::foreign::RaidLedger;
use crate::state::*;

// ---- open_season -----------------------------------------------------------------------------------

/// Accounts of `open_season`.
#[event_cpi]
#[derive(Accounts)]
pub struct OpenSeason<'info> {
    #[account(mut, seeds = [WAR_CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, WarConfig>>,
    #[account(mut, seeds = [SEASON_SEED, &(config.current_season + 1).to_le_bytes()], bump = next.bump)]
    pub next: Box<Account<'info, Season>>,
    /// The running season, when there is one (it must be finalized).
    pub previous: Option<Box<Account<'info, Season>>>,
    #[account(seeds = [LOOT_SEED, &(config.current_season + 1).to_le_bytes()], bump = loot_table.bump)]
    pub loot_table: Box<Account<'info, LootTable>>,
}

/// `open_season`: permissionless once the next season's start and both timelocks have passed and
/// the running season is finalized.
pub fn process_open_season(ctx: Context<OpenSeason>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let current = ctx.accounts.config.current_season;
    if current > 0 {
        let prev = ctx
            .accounts
            .previous
            .as_ref()
            .ok_or(WarError::MissingAccount)?;
        require_keys_eq!(prev.key(), Season::address(current).0, WarError::WrongAccount);
        require!(prev.finalized, WarError::SeasonNotFinalized);
    }
    let next = &mut ctx.accounts.next;
    require!(!next.opened, WarError::SeasonClosed);
    require!(
        now >= next.starts_at && now >= next.eta,
        WarError::TimelockNotPassed
    );
    require!(
        now >= ctx.accounts.loot_table.eta,
        WarError::LootTableNotReady
    );
    next.opened = true;
    let (number, starts_at, ends_at) = (next.number, next.starts_at, next.ends_at);
    ctx.accounts.config.current_season = number;
    emit_cpi!(SeasonOpened {
        season: number,
        starts_at,
        ends_at,
    });
    Ok(())
}

// ---- submit_candidate / finalize_season ------------------------------------------------------------

/// Accounts of `submit_candidate`.
#[event_cpi]
#[derive(Accounts)]
#[instruction(number: u32)]
pub struct SubmitCandidate<'info> {
    pub submitter: Signer<'info>,
    #[account(seeds = [WAR_CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, WarConfig>>,
    #[account(mut, seeds = [SEASON_SEED, &number.to_le_bytes()], bump = season.bump)]
    pub season: Box<Account<'info, Season>>,
    #[account(mut, seeds = [WAR_SEED, war_state.mint.as_ref()], bump = war_state.bump)]
    pub war_state: Box<Account<'info, WarState>>,
    /// CHECK: the candidate's raid ledger, when it has one (address-checked when passed).
    pub raid_ledger: Option<UncheckedAccount<'info>>,
}

/// `submit_candidate(season, mint)`: between `ends_at` and `ends_at + challenge_secs`. Becomes the
/// leader when there is none, or with a strictly higher score, or an equal one and a lower mint.
pub fn process_submit_candidate(ctx: Context<SubmitCandidate>, number: u32) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let params = ctx.accounts.config.params;
    let current = ctx.accounts.config.current_season;
    let season = &ctx.accounts.season;
    require!(season.opened && !season.finalized, WarError::SeasonNotOpen);
    require!(now >= season.ends_at, WarError::SeasonNotEnded);
    require!(
        now < season.ends_at.saturating_add(params.challenge_secs),
        WarError::ChallengeClosed
    );
    let state = &mut ctx.accounts.war_state;
    state.roll(current);
    if let Some(info) = ctx.accounts.raid_ledger.as_ref() {
        require_keys_eq!(info.key(), RaidLedger::address(&state.mint), WarError::WrongAccount);
        let ledger = RaidLedger::read(info)?;
        if state.season_id == number {
            state.season.raid_volume_won = state
                .season
                .raid_volume_won
                .max(ledger.season_volume(number));
        }
    }
    let counters = state.counters_of(number);
    let mint = state.mint;
    let score = ctx
        .accounts
        .season
        .score(&counters)
        .ok_or(WarError::MathOverflow)?;
    let season = &mut ctx.accounts.season;
    let beaten = match season.leader {
        None => None,
        Some(leader) => {
            let better = score > season.leader_score
                || (score == season.leader_score && mint < leader)
                || (leader == mint && score > season.leader_score);
            require!(better, WarError::NotHigherScore);
            Some(leader)
        }
    };
    season.leader = Some(mint);
    season.leader_score = score;
    let submitted_by = ctx.accounts.submitter.key();
    match beaten {
        None => emit_cpi!(CandidateSubmitted {
            season: number,
            mint,
            score,
            submitted_by,
        }),
        Some(b) => emit_cpi!(CandidateChallenged {
            season: number,
            mint,
            score,
            submitted_by,
            beaten: b,
        }),
    }
    Ok(())
}

/// Accounts of `finalize_season`.
#[event_cpi]
#[derive(Accounts)]
#[instruction(number: u32)]
pub struct FinalizeSeason<'info> {
    #[account(mut, seeds = [WAR_CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, WarConfig>>,
    #[account(mut, seeds = [SEASON_SEED, &number.to_le_bytes()], bump = season.bump)]
    pub season: Box<Account<'info, Season>>,
}

/// `finalize_season(season)`: after the challenge window; the window never extends.
pub fn process_finalize_season(ctx: Context<FinalizeSeason>, number: u32) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let challenge = ctx.accounts.config.params.challenge_secs;
    let season = &mut ctx.accounts.season;
    require!(season.opened && !season.finalized, WarError::SeasonNotOpen);
    require!(
        now >= season.ends_at.saturating_add(challenge),
        WarError::SeasonNotEnded
    );
    season.finalized = true;
    let (winner, score) = (season.leader, season.leader_score);
    let config = &mut ctx.accounts.config;
    config.last_winner = winner;
    config.last_winner_season = number;
    emit_cpi!(SeasonFinalized {
        season: number,
        winner,
        score,
    });
    Ok(())
}

// ---- split_protocol_fees ---------------------------------------------------------------------------

/// Accounts of `split_protocol_fees`. Remaining: the bridge's `wrap_sol` / `unwrap_sol` accounts
/// and the token program's.
#[event_cpi]
#[derive(Accounts)]
pub struct SplitProtocolFees<'info> {
    #[account(mut)]
    pub cranker: Signer<'info>,
    #[account(seeds = [WAR_CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, WarConfig>>,
    /// CHECK: the prize vault, the DEX's fee collector (seeds-checked).
    #[account(mut, seeds = [PRIZE_VAULT_SEED], bump = config.prize_vault_bump)]
    pub prize_vault: UncheckedAccount<'info>,
    /// CHECK: the prize vault's bridged-SOL holding (address-checked; may not exist).
    #[account(mut, address = token_client::holding_address(&BRIDGED_SOL_MINT, &prize_vault.key()))]
    pub prize_holding: UncheckedAccount<'info>,
    /// CHECK: the protocol treasury (address-checked).
    #[account(mut, address = config.protocol_treasury @ WarError::WrongAccount)]
    pub treasury: UncheckedAccount<'info>,
    /// CHECK: the last winner's chest, when there is a winner (address-checked).
    #[account(mut)]
    pub winner_chest: Option<UncheckedAccount<'info>>,
    /// The last winner's season (its `prize_paid` accumulates).
    #[account(mut)]
    pub winner_season: Option<Box<Account<'info, Season>>>,
    pub system_program: Program<'info, System>,
}

/// `split_protocol_fees`: of the vault's lamports above its rent minimum (after unwrapping any
/// bridged SOL the admin path paid into its holding), `season_prize_share_bps` to the last winner's
/// chest (wrapped into its holding), the rest to the treasury less a crank bounty.
pub fn process_split_protocol_fees<'info>(
    ctx: Context<'info, SplitProtocolFees<'info>>,
) -> Result<()> {
    let params = ctx.accounts.config.params;
    let all = available(ctx.accounts.to_account_infos(), ctx.remaining_accounts);
    let vault = ctx.accounts.prize_vault.key();
    let vault_seeds = SoloSeeds::new(PRIZE_VAULT_SEED, ctx.accounts.config.prize_vault_bump);
    if ctx.accounts.prize_holding.owner == &TOKEN_ID
        && token_client::read_holding(&ctx.accounts.prize_holding)?.amount > 0
    {
        invoke_built(
            &bordrless_bridge::client::unwrap_sol(vault, u64::MAX),
            &all,
            &[&vault_seeds.seeds()],
        )?;
    }
    let available_lamports = ctx
        .accounts
        .prize_vault
        .lamports()
        .saturating_sub(Rent::get()?.minimum_balance(0));
    require!(available_lamports > 0, WarError::NothingToDo);

    let winner = ctx.accounts.config.last_winner;
    let mut to_winner = 0u64;
    if let Some(w) = winner {
        let (chest, chest_bump) = chest_address(&w);
        let chest_info = ctx
            .accounts
            .winner_chest
            .as_ref()
            .ok_or(WarError::MissingAccount)?;
        require_keys_eq!(chest_info.key(), chest, WarError::WrongAccount);
        let season = ctx
            .accounts
            .winner_season
            .as_mut()
            .ok_or(WarError::MissingAccount)?;
        require_keys_eq!(
            season.key(),
            Season::address(ctx.accounts.config.last_winner_season).0,
            WarError::WrongAccount
        );
        to_winner = bps_of(available_lamports, u64::from(params.season_prize_share_bps));
        season.prize_paid = season.prize_paid.saturating_add(to_winner);
        if to_winner > 0 {
            transfer_lamports(
                &all,
                &vault_seeds.seeds(),
                vault,
                &chest_info.to_account_info(),
                to_winner,
            )?;
            let chest_seeds = KeyedSeeds::new(CHEST_SEED, w, chest_bump);
            invoke_built(
                &bordrless_bridge::client::wrap_sol(chest, to_winner),
                &all,
                &[&chest_seeds.seeds()],
            )?;
        }
    }
    let rest = available_lamports - to_winner;
    let bounty = bps_of(rest, u64::from(params.max_crank_bounty_bps));
    let to_treasury = rest - bounty;
    transfer_lamports(
        &all,
        &vault_seeds.seeds(),
        vault,
        &ctx.accounts.treasury.to_account_info(),
        to_treasury,
    )?;
    transfer_lamports(
        &all,
        &vault_seeds.seeds(),
        vault,
        &ctx.accounts.cranker.to_account_info(),
        bounty,
    )?;
    emit_cpi!(PrizePaid {
        season: ctx.accounts.config.last_winner_season,
        winner,
        to_winner,
        to_treasury,
        bounty,
        cranker: ctx.accounts.cranker.key(),
    });
    Ok(())
}
