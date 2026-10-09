//! Quests whose condition this program checks, each granting one loot ticket (05 section 9).
//! "Once per period" lives in a `QuestMark` PDA, never in hook data.

use anchor_lang::prelude::*;
use anchor_lang::system_program;
use bordrless_token::client as token_client;
use bordrless_token::state::Mint;

use crate::common::*;
use crate::constants::*;
use crate::error::WarError;
use crate::events::*;
use crate::foreign::ForgeCounter;
use crate::instructions::admin::read_own;
use crate::state::*;

/// Accounts of `claim_quest`. Remaining: the Raid slot's extras.
#[event_cpi]
#[derive(Accounts)]
pub struct ClaimQuest<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(seeds = [WAR_CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, WarConfig>>,
    /// CHECK: the current `Season` (address-checked).
    pub season: UncheckedAccount<'info>,
    pub mint: Box<Account<'info, Mint>>,
    /// CHECK: the owner's holding of the mint (address-checked).
    #[account(mut, address = token_client::holding_address(&mint.key(), &owner.key()))]
    pub holding: UncheckedAccount<'info>,
    /// CHECK: the `QuestMark` (address-checked; created here at the first claim).
    #[account(mut)]
    pub quest_mark: UncheckedAccount<'info>,
    /// CHECK: the armory's `ForgeCounter` of the owner (`Forge` only; checked when read).
    pub forge_counter: Option<UncheckedAccount<'info>>,
    /// CHECK: `["war-signer"]`.
    #[account(seeds = [WAR_SIGNER_SEED], bump)]
    pub war_signer: UncheckedAccount<'info>,
    /// CHECK: the items program.
    #[account(address = ITEMS_ID)]
    pub items_program: UncheckedAccount<'info>,
    /// CHECK: the token program's signer of the items program.
    #[account(address = token_client::hook_signer(&ITEMS_ID))]
    pub items_hook_signer: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

/// The current quest period of `season` at `now` (the first is 1).
pub fn period_at(season: &Season, now: i64, period_secs: i64) -> u32 {
    let elapsed = now.saturating_sub(season.starts_at).max(0);
    u32::try_from(elapsed / period_secs.max(1))
        .unwrap_or(u32::MAX - 1)
        .saturating_add(1)
}

/// Creates the `QuestMark` at `info` when it does not exist, the owner paying.
fn ensure_mark<'info>(
    info: &AccountInfo<'info>,
    owner: &AccountInfo<'info>,
    system: &AccountInfo<'info>,
    season: u32,
    mark_mint: Pubkey,
) -> Result<QuestMark> {
    let (address, bump) = QuestMark::address(season, &mark_mint, owner.key);
    require_keys_eq!(*info.key, address, WarError::WrongAccount);
    if info.owner == &crate::ID {
        return read_own::<QuestMark>(info);
    }
    let rent = Rent::get()?.minimum_balance(QuestMark::LEN);
    let season_le = season.to_le_bytes();
    let bump_seed = [bump];
    let seeds: [&[u8]; 5] = [
        QUEST_SEED,
        &season_le,
        mark_mint.as_ref(),
        owner.key.as_ref(),
        &bump_seed,
    ];
    system_program::create_account(
        CpiContext::new_with_signer(
            *system.key,
            system_program::CreateAccount {
                from: owner.clone(),
                to: info.clone(),
            },
            &[&seeds],
        ),
        rent,
        QuestMark::LEN as u64,
        &crate::ID,
    )?;
    Ok(QuestMark {
        version: VERSION,
        bump,
        season,
        mint: mark_mint,
        owner: *owner.key,
        last_period: 0,
        last_forge_count: 0,
    })
}

fn write_mark(info: &AccountInfo, mark: &QuestMark) -> Result<()> {
    let mut data = info.try_borrow_mut_data()?;
    let mut cursor: &mut [u8] = &mut data;
    mark.try_serialize(&mut cursor)
}

/// `claim_quest(quest_id, period, raid_slot)`.
pub fn process_claim_quest<'info>(
    ctx: Context<'info, ClaimQuest<'info>>,
    quest_id: u8,
    period: u32,
    raid_slot_index: u8,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let params = ctx.accounts.config.params;
    let current = ctx.accounts.config.current_season;
    require!(current > 0, WarError::SeasonNotOpen);
    require_keys_eq!(
        ctx.accounts.season.key(),
        Season::address(current).0,
        WarError::WrongAccount
    );
    let season = read_own::<Season>(&ctx.accounts.season)?;
    require!(
        period == period_at(&season, now, params.quest_period_secs),
        WarError::QuestConditionUnmet
    );
    let mint_key = ctx.accounts.mint.key();
    let mark_mint = match quest_id {
        quest::RAID => mint_key,
        quest::FORGE => Pubkey::default(),
        _ => return err!(WarError::InvalidParams),
    };
    let mut mark = ensure_mark(
        &ctx.accounts.quest_mark,
        &ctx.accounts.owner.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        current,
        mark_mint,
    )?;
    require!(period > mark.last_period, WarError::QuestAlreadyClaimed);

    let slot = raid_slot(&ctx.accounts.mint, raid_slot_index)?;
    let all = available(ctx.accounts.to_account_infos(), ctx.remaining_accounts);
    let extras = slice_metas(ctx.remaining_accounts, usize::from(slot.extra_count))?;
    let touch = |payload: WarTouch| {
        touch_raid(
            &all,
            ctx.accounts.war_signer.key(),
            ctx.bumps.war_signer,
            mint_key,
            ctx.accounts.holding.key(),
            raid_slot_index,
            payload,
            extras.clone(),
        )
    };
    let before = read_raid(&ctx.accounts.holding, &slot)?;
    if quest_id == quest::RAID {
        let need = params.quest_raid_points;
        require!(
            need > 0 && before.points_in(current) >= need,
            WarError::QuestConditionUnmet
        );
        touch(WarTouch::SpendRaidPoints { amount: need })?;
        let spent = read_raid(&ctx.accounts.holding, &slot)?;
        require!(
            spent.points_in(current) == before.points_in(current) - need,
            WarError::TouchMismatch
        );
    } else {
        let info = ctx
            .accounts
            .forge_counter
            .as_ref()
            .ok_or(WarError::MissingAccount)?;
        require_keys_eq!(
            info.key(),
            ForgeCounter::address(&ctx.accounts.owner.key()),
            WarError::WrongAccount
        );
        let counter = ForgeCounter::read(info)?;
        require!(
            counter.count > mark.last_forge_count,
            WarError::QuestConditionUnmet
        );
        mark.last_forge_count = counter.count;
    }
    let tickets = read_raid(&ctx.accounts.holding, &slot)?.tickets;
    touch(WarTouch::AddTicket { amount: 1 })?;
    let after = read_raid(&ctx.accounts.holding, &slot)?;
    require!(
        after.tickets == tickets.saturating_add(1),
        WarError::TouchMismatch
    );
    mark.last_period = period;
    write_mark(&ctx.accounts.quest_mark, &mark)?;
    emit_cpi!(QuestClaimed {
        quest_id,
        mint: mint_key,
        owner: ctx.accounts.owner.key(),
        season: current,
        period,
    });
    Ok(())
}

/// Accounts of `close_quest_mark`.
#[derive(Accounts)]
pub struct CloseQuestMark<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(seeds = [SEASON_SEED, &quest_mark.season.to_le_bytes()], bump = season.bump,
              constraint = season.finalized @ WarError::SeasonNotFinalized)]
    pub season: Box<Account<'info, Season>>,
    #[account(mut, close = owner, has_one = owner @ WarError::WrongAccount)]
    pub quest_mark: Box<Account<'info, QuestMark>>,
}

/// `close_quest_mark`: the rent back to the owner once the mark's season is finalized.
pub fn process_close_quest_mark(_ctx: Context<CloseQuestMark>) -> Result<()> {
    Ok(())
}
