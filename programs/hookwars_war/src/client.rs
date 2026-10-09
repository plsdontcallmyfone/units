//! Instruction builders for clients and the LiteSVM suites. The steps that call other programs take
//! a `slice` (the token slice the client resolved, as the token program checks it) and `inner`:
//! the instructions the step will build on chain, whose accounts must be present (see
//! [`accounts_of`]).

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use bordrless_token::client as token_client;

use crate::constants::*;
use crate::instructions::{ConfigArgs, SeasonArgs, SliceArgs};
use crate::state::*;

pub fn event_authority() -> Pubkey {
    crate::EVENT_AUTHORITY_AND_BUMP.0
}

pub fn config_address() -> Pubkey {
    WarConfig::address().0
}

pub fn program_data_address() -> Pubkey {
    Pubkey::find_program_address(&[crate::ID.as_ref()], &BPF_LOADER_UPGRADEABLE_ID).0
}

/// Every account of `ixs` (and their programs), as remaining accounts: none signs, writability as
/// the instruction has it. Duplicates are harmless (a transaction lists each key once).
pub fn accounts_of(ixs: &[Instruction]) -> Vec<AccountMeta> {
    let mut v = Vec::new();
    for ix in ixs {
        for m in &ix.accounts {
            v.push(AccountMeta {
                pubkey: m.pubkey,
                is_signer: false,
                is_writable: m.is_writable,
            });
        }
        v.push(AccountMeta::new_readonly(ix.program_id, false));
    }
    v
}

fn ix(accounts: Vec<AccountMeta>, data: Vec<u8>) -> Instruction {
    Instruction {
        program_id: crate::ID,
        accounts,
        data,
    }
}

fn opt(key: Option<Pubkey>) -> Option<Pubkey> {
    key
}

pub fn init_config(authority: Pubkey, args: ConfigArgs) -> Instruction {
    ix(
        crate::accounts::InitConfig {
            authority,
            config: config_address(),
            program_data: program_data_address(),
            system_program: system_program::ID,
            event_authority: event_authority(),
            program: crate::ID,
        }
        .to_account_metas(None),
        crate::instruction::InitConfig { args }.data(),
    )
}

pub fn propose_config(admin: Pubkey, args: ConfigArgs) -> Instruction {
    ix(
        crate::accounts::ProposeConfig {
            admin,
            config: config_address(),
            event_authority: event_authority(),
            program: crate::ID,
        }
        .to_account_metas(None),
        crate::instruction::ProposeConfig { args }.data(),
    )
}

pub fn apply_config() -> Instruction {
    ix(
        crate::accounts::ApplyConfig {
            config: config_address(),
            event_authority: event_authority(),
            program: crate::ID,
        }
        .to_account_metas(None),
        crate::instruction::ApplyConfig {}.data(),
    )
}

pub fn cancel_pending(admin: Pubkey, what: u8, season: u32) -> Instruction {
    let target = match what {
        1 => Season::address(season).0,
        2 => LootTable::address(season).0,
        _ => config_address(),
    };
    ix(
        crate::accounts::CancelPending {
            admin,
            config: config_address(),
            target,
            event_authority: event_authority(),
            program: crate::ID,
        }
        .to_account_metas(None),
        crate::instruction::CancelPending { what, season }.data(),
    )
}

pub fn propose_season(admin: Pubkey, args: SeasonArgs) -> Instruction {
    ix(
        crate::accounts::ProposeSeason {
            admin,
            config: config_address(),
            season: Season::address(args.number).0,
            system_program: system_program::ID,
            event_authority: event_authority(),
            program: crate::ID,
        }
        .to_account_metas(None),
        crate::instruction::ProposeSeason { args }.data(),
    )
}

/// `propose_loot_table`: the templates of the entries follow, in entry order.
pub fn propose_loot_table(admin: Pubkey, season: u32, entries: Vec<LootEntry>, templates: &[Pubkey]) -> Instruction {
    let mut accounts = crate::accounts::ProposeLootTable {
        admin,
        config: config_address(),
        loot_table: LootTable::address(season).0,
        system_program: system_program::ID,
        event_authority: event_authority(),
        program: crate::ID,
    }
    .to_account_metas(None);
    accounts.extend(templates.iter().map(|t| AccountMeta::new_readonly(*t, false)));
    ix(
        accounts,
        crate::instruction::ProposeLootTable { season, entries }.data(),
    )
}

pub fn init_war(payer: Pubkey, mint: Pubkey) -> Instruction {
    let chest = chest_address(&mint).0;
    let inbox = inbox_address(&mint).0;
    ix(
        crate::accounts::InitWar {
            payer,
            config: config_address(),
            mint,
            launch: bordrless_launch::client::launch_address(&mint),
            war_state: WarState::address(&mint).0,
            war_chest: chest,
            treaty_inbox: inbox,
            bridged_sol_mint: BRIDGED_SOL_MINT,
            chest_holding: token_client::holding_address(&BRIDGED_SOL_MINT, &chest),
            inbox_holding: token_client::holding_address(&BRIDGED_SOL_MINT, &inbox),
            token_program: TOKEN_ID,
            token_event_authority: token_client::event_authority(),
            system_program: system_program::ID,
            event_authority: event_authority(),
            program: crate::ID,
        }
        .to_account_metas(None),
        crate::instruction::InitWar {}.data(),
    )
}

pub fn record_funding(mint: Pubkey) -> Instruction {
    let chest = chest_address(&mint).0;
    ix(
        crate::accounts::RecordFunding {
            war_state: WarState::address(&mint).0,
            chest_holding: token_client::holding_address(&BRIDGED_SOL_MINT, &chest),
            event_authority: event_authority(),
            program: crate::ID,
        }
        .to_account_metas(None),
        crate::instruction::RecordFunding {}.data(),
    )
}

/// Keys of a step's War orders: the item the War slot names and its template.
#[derive(Clone, Copy, Debug)]
pub struct Orders {
    pub item: Pubkey,
    pub template: Pubkey,
}

/// `siege`: `slice` is the rival mint's delivery slice; `inner` the instructions the step builds
/// (the launch swap, the holding creation, the bridge unwrap).
#[allow(clippy::too_many_arguments)]
pub fn siege(
    cranker: Pubkey,
    mint: Pubkey,
    orders: Orders,
    rival_mint: Pubkey,
    rival_pool: Pubkey,
    rival_has_war: bool,
    rival_kit_config: Option<Pubkey>,
    slice: Vec<AccountMeta>,
    inner: &[Instruction],
) -> Instruction {
    let mut accounts = crate::accounts::Siege {
        cranker,
        config: config_address(),
        war_state: WarState::address(&mint).0,
        war_chest: chest_address(&mint).0,
        mint,
        orders_item: orders.item,
        orders_template: orders.template,
        raid_ledger: crate::foreign::raid_ledger_address(&mint),
        rival_mint,
        rival_launch: bordrless_launch::client::launch_address(&rival_mint),
        rival_pool,
        rival_war_state: rival_has_war.then(|| WarState::address(&rival_mint).0),
        rival_kit_config: opt(rival_kit_config),
        system_program: system_program::ID,
        event_authority: event_authority(),
        program: crate::ID,
    }
    .to_account_metas(None);
    let first = slice.len() as u8;
    accounts.extend(slice);
    accounts.extend(accounts_of(inner));
    ix(
        accounts,
        crate::instruction::Siege {
            args: SliceArgs { first, second: 0 },
        }
        .data(),
    )
}

/// `counter_strike`: `buy_slice` for the delivery, `burn_slice` for the burn.
#[allow(clippy::too_many_arguments)]
pub fn counter_strike(
    cranker: Pubkey,
    mint: Pubkey,
    orders: Orders,
    pool: Pubkey,
    kit_config: Option<Pubkey>,
    buy_slice: Vec<AccountMeta>,
    burn_slice: Vec<AccountMeta>,
    inner: &[Instruction],
) -> Instruction {
    let mut accounts = crate::accounts::CounterStrike {
        cranker,
        config: config_address(),
        war_state: WarState::address(&mint).0,
        war_chest: chest_address(&mint).0,
        mint,
        orders_item: orders.item,
        orders_template: orders.template,
        launch: bordrless_launch::client::launch_address(&mint),
        pool,
        kit_config,
        system_program: system_program::ID,
        event_authority: event_authority(),
        program: crate::ID,
    }
    .to_account_metas(None);
    let args = SliceArgs {
        first: buy_slice.len() as u8,
        second: burn_slice.len() as u8,
    };
    accounts.extend(buy_slice);
    accounts.extend(burn_slice);
    accounts.extend(accounts_of(inner));
    ix(accounts, crate::instruction::CounterStrike { args }.data())
}

#[allow(clippy::too_many_arguments)]
pub fn raze(
    cranker: Pubkey,
    mint: Pubkey,
    orders: Orders,
    rival_mint: Pubkey,
    rival_pool: Pubkey,
    slice: Vec<AccountMeta>,
    inner: &[Instruction],
) -> Instruction {
    let mut accounts = crate::accounts::Raze {
        cranker,
        config: config_address(),
        war_state: WarState::address(&mint).0,
        war_chest: chest_address(&mint).0,
        mint,
        orders_item: orders.item,
        orders_template: orders.template,
        rival_mint,
        rival_launch: bordrless_launch::client::launch_address(&rival_mint),
        rival_pool,
        system_program: system_program::ID,
        event_authority: event_authority(),
        program: crate::ID,
    }
    .to_account_metas(None);
    let first = slice.len() as u8;
    accounts.extend(slice);
    accounts.extend(accounts_of(inner));
    ix(
        accounts,
        crate::instruction::Raze {
            args: SliceArgs { first, second: 0 },
        }
        .data(),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn return_captured(
    cranker: Pubkey,
    mint: Pubkey,
    rival_mint: Pubkey,
    treaty_item: Pubkey,
    treaty_template: Pubkey,
    slice: Vec<AccountMeta>,
    inner: &[Instruction],
) -> Instruction {
    let mut accounts = crate::accounts::ReturnCaptured {
        cranker,
        config: config_address(),
        war_state: WarState::address(&mint).0,
        war_chest: chest_address(&mint).0,
        mint,
        rival_mint,
        treaty_item,
        treaty_template,
        rival_war_chest: chest_address(&rival_mint).0,
        system_program: system_program::ID,
        event_authority: event_authority(),
        program: crate::ID,
    }
    .to_account_metas(None);
    let first = slice.len() as u8;
    accounts.extend(slice);
    accounts.extend(accounts_of(inner));
    ix(
        accounts,
        crate::instruction::ReturnCaptured {
            args: SliceArgs { first, second: 0 },
        }
        .data(),
    )
}

pub fn share_treaty_inflow(cranker: Pubkey, mint: Pubkey, inner: &[Instruction]) -> Instruction {
    let inbox = inbox_address(&mint).0;
    let mut accounts = crate::accounts::ShareTreatyInflow {
        cranker,
        config: config_address(),
        war_state: WarState::address(&mint).0,
        treaty_inbox: inbox,
        inbox_holding: token_client::holding_address(&BRIDGED_SOL_MINT, &inbox),
        system_program: system_program::ID,
        event_authority: event_authority(),
        program: crate::ID,
    }
    .to_account_metas(None);
    accounts.extend(accounts_of(inner));
    ix(accounts, crate::instruction::ShareTreatyInflow {}.data())
}

/// `accrue_treaty_time`: `pairs` of (treaty item, partner mint).
pub fn accrue_treaty_time(mint: Pubkey, season: u32, pairs: &[(Pubkey, Pubkey)]) -> Instruction {
    let mut accounts = crate::accounts::AccrueTreatyTime {
        config: config_address(),
        war_state: WarState::address(&mint).0,
        mint,
        season: Season::address(season).0,
        event_authority: event_authority(),
        program: crate::ID,
    }
    .to_account_metas(None);
    for (item, partner) in pairs {
        accounts.push(AccountMeta::new_readonly(*item, false));
        accounts.push(AccountMeta::new_readonly(*partner, false));
    }
    ix(accounts, crate::instruction::AccrueTreatyTime {}.data())
}

/// `claim_bounty`: `raid_extras` are the Raid slot's extras; `inner` the bridge unwrap.
pub fn claim_bounty(
    owner: Pubkey,
    mint: Pubkey,
    orders: Orders,
    raid_slot: u8,
    raid_extras: Vec<AccountMeta>,
    inner: &[Instruction],
) -> Instruction {
    let chest = chest_address(&mint).0;
    let mut accounts = crate::accounts::ClaimBounty {
        owner,
        config: config_address(),
        war_state: WarState::address(&mint).0,
        war_chest: chest,
        chest_holding: token_client::holding_address(&BRIDGED_SOL_MINT, &chest),
        mint,
        orders_item: orders.item,
        orders_template: orders.template,
        holding: token_client::holding_address(&mint, &owner),
        war_signer: war_signer_address().0,
        items_program: ITEMS_ID,
        items_hook_signer: token_client::hook_signer(&ITEMS_ID),
        system_program: system_program::ID,
        event_authority: event_authority(),
        program: crate::ID,
    }
    .to_account_metas(None);
    accounts.extend(raid_extras);
    accounts.push(AccountMeta::new_readonly(TOKEN_ID, false));
    accounts.push(AccountMeta::new_readonly(token_client::event_authority(), false));
    accounts.extend(accounts_of(inner));
    ix(accounts, crate::instruction::ClaimBounty { raid_slot }.data())
}

#[allow(clippy::too_many_arguments)]
pub fn roll(
    owner: Pubkey,
    mint: Pubkey,
    nonce: u64,
    raid_slot: u8,
    oracle_program: Pubkey,
    oracle_account: Pubkey,
    raid_extras: Vec<AccountMeta>,
) -> Instruction {
    let holding = token_client::holding_address(&mint, &owner);
    let mut accounts = crate::accounts::Roll {
        owner,
        config: config_address(),
        mint,
        holding,
        roll_request: RollRequest::address(&holding, nonce).0,
        war_signer: war_signer_address().0,
        items_program: ITEMS_ID,
        items_hook_signer: token_client::hook_signer(&ITEMS_ID),
        oracle_program,
        oracle_account,
        system_program: system_program::ID,
        event_authority: event_authority(),
        program: crate::ID,
    }
    .to_account_metas(None);
    accounts.extend(raid_extras);
    accounts.push(AccountMeta::new_readonly(TOKEN_ID, false));
    accounts.push(AccountMeta::new_readonly(token_client::event_authority(), false));
    ix(accounts, crate::instruction::Roll { nonce, raid_slot }.data())
}

/// `reveal`: `armory_extra` are `mint_loot`'s further accounts.
pub fn reveal(
    revealer: Pubkey,
    owner: Pubkey,
    holding: Pubkey,
    nonce: u64,
    season: u32,
    oracle_account: Pubkey,
    armory_extra: Vec<AccountMeta>,
) -> Instruction {
    let mut accounts = crate::accounts::Reveal {
        revealer,
        config: config_address(),
        owner,
        roll_request: RollRequest::address(&holding, nonce).0,
        loot_table: LootTable::address(season).0,
        oracle_account,
        loot_signer: loot_signer_address().0,
        armory_program: ARMORY_ID,
        system_program: system_program::ID,
        event_authority: event_authority(),
        program: crate::ID,
    }
    .to_account_metas(None);
    accounts.extend(armory_extra);
    ix(accounts, crate::instruction::Reveal {}.data())
}

pub fn cancel_roll(owner: Pubkey, holding: Pubkey, nonce: u64) -> Instruction {
    ix(
        crate::accounts::CancelRoll {
            owner,
            config: config_address(),
            roll_request: RollRequest::address(&holding, nonce).0,
            event_authority: event_authority(),
            program: crate::ID,
        }
        .to_account_metas(None),
        crate::instruction::CancelRoll {}.data(),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn claim_quest(
    owner: Pubkey,
    mint: Pubkey,
    season: u32,
    quest_id: u8,
    period: u32,
    raid_slot: u8,
    raid_extras: Vec<AccountMeta>,
) -> Instruction {
    let mark_mint = if quest_id == quest::FORGE {
        Pubkey::default()
    } else {
        mint
    };
    let mut accounts = crate::accounts::ClaimQuest {
        owner,
        config: config_address(),
        season: Season::address(season).0,
        mint,
        holding: token_client::holding_address(&mint, &owner),
        quest_mark: QuestMark::address(season, &mark_mint, &owner).0,
        forge_counter: (quest_id == quest::FORGE).then(|| crate::foreign::forge_counter_address(&owner)),
        war_signer: war_signer_address().0,
        items_program: ITEMS_ID,
        items_hook_signer: token_client::hook_signer(&ITEMS_ID),
        system_program: system_program::ID,
        event_authority: event_authority(),
        program: crate::ID,
    }
    .to_account_metas(None);
    accounts.extend(raid_extras);
    accounts.push(AccountMeta::new_readonly(TOKEN_ID, false));
    accounts.push(AccountMeta::new_readonly(token_client::event_authority(), false));
    ix(
        accounts,
        crate::instruction::ClaimQuest {
            quest_id,
            period,
            raid_slot,
        }
        .data(),
    )
}

pub fn close_quest_mark(owner: Pubkey, season: u32, mark: Pubkey) -> Instruction {
    ix(
        crate::accounts::CloseQuestMark {
            owner,
            season: Season::address(season).0,
            quest_mark: mark,
        }
        .to_account_metas(None),
        crate::instruction::CloseQuestMark {}.data(),
    )
}

/// `open_season`: `current` is the config's current season.
pub fn open_season(current: u32) -> Instruction {
    ix(
        crate::accounts::OpenSeason {
            config: config_address(),
            next: Season::address(current + 1).0,
            previous: (current > 0).then(|| Season::address(current).0),
            loot_table: LootTable::address(current + 1).0,
            event_authority: event_authority(),
            program: crate::ID,
        }
        .to_account_metas(None),
        crate::instruction::OpenSeason {}.data(),
    )
}

pub fn submit_candidate(submitter: Pubkey, number: u32, mint: Pubkey, with_ledger: bool) -> Instruction {
    ix(
        crate::accounts::SubmitCandidate {
            submitter,
            config: config_address(),
            season: Season::address(number).0,
            war_state: WarState::address(&mint).0,
            raid_ledger: with_ledger.then(|| crate::foreign::raid_ledger_address(&mint)),
            event_authority: event_authority(),
            program: crate::ID,
        }
        .to_account_metas(None),
        crate::instruction::SubmitCandidate { number }.data(),
    )
}

pub fn finalize_season(number: u32) -> Instruction {
    ix(
        crate::accounts::FinalizeSeason {
            config: config_address(),
            season: Season::address(number).0,
            event_authority: event_authority(),
            program: crate::ID,
        }
        .to_account_metas(None),
        crate::instruction::FinalizeSeason { number }.data(),
    )
}

/// `split_protocol_fees`: `winner` is the config's last winner and its season, when set.
pub fn split_protocol_fees(
    cranker: Pubkey,
    treasury: Pubkey,
    winner: Option<(Pubkey, u32)>,
    inner: &[Instruction],
) -> Instruction {
    let vault = prize_vault_address().0;
    let mut accounts = crate::accounts::SplitProtocolFees {
        cranker,
        config: config_address(),
        prize_vault: vault,
        prize_holding: token_client::holding_address(&BRIDGED_SOL_MINT, &vault),
        treasury,
        winner_chest: winner.map(|(m, _)| chest_address(&m).0),
        winner_season: winner.map(|(_, s)| Season::address(s).0),
        system_program: system_program::ID,
        event_authority: event_authority(),
        program: crate::ID,
    }
    .to_account_metas(None);
    accounts.extend(accounts_of(inner));
    ix(accounts, crate::instruction::SplitProtocolFees {}.data())
}
