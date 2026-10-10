// Changed by Hookwars: new file (pass 4b, 10 section 11.1); review 3 L-7: a named boss takes effect after the admin
// timelock.
//! Boss events: the season's `BossPool`, funded by `boss_share_bps` of the prize split
//! (`split_protocol_fees`), sealed from the boss token's `RaidLedger` after the season's end, and
//! paid to each source token's war chest by `claim_boss_share`, `to_share * source_volume /
//! total_volume`, once per source. Money only reaches war chests, which spend it under their
//! normal rules; no wallet is paid for an outcome.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::AccountMeta;
use bordrless_token::client as token_client;

use crate::common::*;
use crate::constants::*;
use crate::error::WarError;
use crate::events::*;
use crate::foreign::{Foreign, RaidLedger};
use crate::instructions::setup::{chest_balance, note_funding};
use crate::state::*;

/// Accounts of `init_boss_pool`.
#[event_cpi]
#[derive(Accounts)]
#[instruction(season: u32)]
pub struct InitBossPool<'info> {
    #[account(mut, address = config.admin @ WarError::NotAdmin)]
    pub admin: Signer<'info>,
    #[account(seeds = [WAR_CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, WarConfig>>,
    #[account(init, payer = admin, space = BossPool::LEN, seeds = [BOSS_SEED, &season.to_le_bytes()], bump)]
    pub pool: Box<Account<'info, BossPool>>,
    pub system_program: Program<'info, System>,
}

/// `init_boss_pool(season, boss_mint)`: the admin names the season's boss token (an ordinary
/// launch by the protocol, running the Boss item). The running season or a later one.
pub fn process_init_boss_pool(ctx: Context<InitBossPool>, season: u32, boss_mint: Pubkey) -> Result<()> {
    require!(season >= ctx.accounts.config.current_season.max(1), WarError::BossPoolState);
    let p = &mut ctx.accounts.pool;
    p.version = VERSION;
    p.bump = ctx.bumps.pool;
    p.season = season;
    p.boss_mint = boss_mint;
    p.effective_at = Clock::get()?
        .unix_timestamp
        .saturating_add(i64::from(ctx.accounts.config.params.admin_timelock_secs));
    emit_cpi!(BossPoolOpened { season, boss_mint });
    Ok(())
}

/// Accounts of `seal_boss_pool`.
#[event_cpi]
#[derive(Accounts)]
#[instruction(season: u32)]
pub struct SealBossPool<'info> {
    #[account(seeds = [SEASON_SEED, &season.to_le_bytes()], bump = season_account.bump)]
    pub season_account: Box<Account<'info, Season>>,
    #[account(mut, seeds = [BOSS_SEED, &season.to_le_bytes()], bump = pool.bump)]
    pub pool: Box<Account<'info, BossPool>>,
    /// CHECK: the boss token's `RaidLedger` under items (address-checked, decoded).
    #[account(address = crate::foreign::raid_ledger_address(&pool.boss_mint) @ WarError::WrongAccount)]
    pub boss_ledger: UncheckedAccount<'info>,
}

/// `seal_boss_pool(season)`: permissionless from the season's `ends_at`. Copies each recorded
/// source's volume (both windows of its ledger entry) and shares what the pool was funded with.
/// The Boss item names only the ledger (no war config), so its ledger keeps season 0; a ledger
/// rolled to another season is refused.
pub fn process_seal_boss_pool(ctx: Context<SealBossPool>, season: u32) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    require!(now >= ctx.accounts.season_account.ends_at, WarError::SeasonNotEnded);
    let ledger = RaidLedger::read(&ctx.accounts.boss_ledger)?;
    require!(ledger.season_id == season || ledger.season_id == 0, WarError::BossPoolState);
    let p = &mut ctx.accounts.pool;
    require!(!p.sealed, WarError::BossPoolState);
    let mut total = 0u64;
    let mut n = 0u8;
    for (i, e) in ledger.inbound.iter().enumerate() {
        if e.rival_mint == Pubkey::default() {
            continue;
        }
        let v = e.volume.saturating_add(e.prev_volume);
        p.sources[i] = BossSource {
            mint: e.rival_mint,
            volume: v,
            claimed: false,
        };
        total = total.saturating_add(v);
        n += 1;
    }
    p.total_volume = total;
    p.to_share = p.funded.saturating_sub(p.paid);
    p.sealed = true;
    emit_cpi!(BossPoolSealed {
        season,
        to_share: p.to_share,
        total_volume: total,
        sources: n,
    });
    Ok(())
}

/// Accounts of `claim_boss_share`. Remaining: the bridge's `wrap_sol` accounts for the chest.
#[event_cpi]
#[derive(Accounts)]
#[instruction(season: u32)]
pub struct ClaimBossShare<'info> {
    #[account(seeds = [WAR_CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, WarConfig>>,
    #[account(mut, seeds = [BOSS_SEED, &season.to_le_bytes()], bump = pool.bump)]
    pub pool: Box<Account<'info, BossPool>>,
    #[account(mut, seeds = [WAR_SEED, war_state.mint.as_ref()], bump = war_state.bump)]
    pub war_state: Box<Account<'info, WarState>>,
    /// CHECK: the source's chest (seeds-checked).
    #[account(mut, seeds = [CHEST_SEED, war_state.mint.as_ref()], bump = war_state.chest_bump)]
    pub war_chest: UncheckedAccount<'info>,
    /// CHECK: the chest's bridged-SOL holding (address-checked).
    #[account(mut, address = token_client::holding_address(&BRIDGED_SOL_MINT, &war_chest.key()))]
    pub chest_holding: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

/// `claim_boss_share(season)`: permissionless; pays the war state's token its sealed share into its
/// chest (wrapped), booked as `received_other` (solvency, not season funding). Pass 5 (M-7): the
/// source's volume is capped by its season funding.
pub fn process_claim_boss_share<'info>(ctx: Context<'info, ClaimBossShare<'info>>, season: u32) -> Result<()> {
    let mint = ctx.accounts.war_state.mint;
    let (i, amount, volume) = {
        let p = &ctx.accounts.pool;
        require!(p.sealed, WarError::BossPoolState);
        let i = p
            .sources
            .iter()
            .position(|s| s.mint == mint && s.volume > 0)
            .ok_or(WarError::NoBossShare)?;
        require!(!p.sources[i].claimed, WarError::NoBossShare);
        // Pass 5 (review 3 M-7): a source's volume counts only up to `raid_volume_per_funded`
        // times what its chest received in that season (the M-B cap of the score), so volume
        // washed through the boss by a token with an unfunded chest earns nothing. What a cap
        // withholds stays in the pool.
        let st = &ctx.accounts.war_state;
        let funded = if st.season_id == season {
            st.season.funded
        } else if st.season_id == season.saturating_add(1) {
            st.prev_season.funded
        } else {
            0
        };
        let cap = funded.saturating_mul(ctx.accounts.config.params.raid_volume_per_funded);
        let v = p.sources[i].volume.min(cap);
        let amount = (u128::from(p.to_share) * u128::from(v) / u128::from(p.total_volume.max(1))) as u64;
        (i, amount, v)
    };
    ctx.accounts.pool.sources[i].claimed = true;
    ctx.accounts.pool.paid = ctx.accounts.pool.paid.saturating_add(amount);
    let current = ctx.accounts.config.current_season;
    let before = chest_balance(&ctx.accounts.chest_holding)?;
    {
        let s = &mut ctx.accounts.war_state;
        s.roll(current);
        note_funding(s, before);
    }
    if amount > 0 {
        let pool_info = ctx.accounts.pool.to_account_info();
        pool_info.sub_lamports(amount)?;
        ctx.accounts.war_chest.add_lamports(amount)?;
        let all = available(ctx.accounts.to_account_infos(), ctx.remaining_accounts);
        let chest = ctx.accounts.war_chest.key();
        let seeds = KeyedSeeds::new(CHEST_SEED, mint, ctx.accounts.war_state.chest_bump);
        // The pool's direct debit must reach the runtime with the chest's credit: the CPI syncs
        // only the accounts it names, so the pool rides along as an extra writable account (the
        // bridge ignores it) and the caller's lamports stay balanced.
        let mut wrap = bordrless_bridge::client::wrap_sol(chest, amount);
        wrap.accounts.push(AccountMeta::new(ctx.accounts.pool.key(), false));
        invoke_built(&wrap, &all, &[&seeds.seeds()])?;
    }
    let after = chest_balance(&ctx.accounts.chest_holding)?;
    let s = &mut ctx.accounts.war_state;
    s.received_other = s.received_other.saturating_add(after.saturating_sub(before));
    s.last_seen_balance = after;
    emit_cpi!(BossShareClaimed {
        season,
        source_mint: mint,
        volume,
        amount,
    });
    Ok(())
}
