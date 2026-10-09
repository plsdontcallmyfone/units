// Changed by Hookwars: new file (M2), the armory; M3b: composites (create_composite, the module
// list passed to init_equip), the Performance reader on the pool's ring, settle_bounty_bps.
//! `hookwars_armory` (docs/spec/02-armory.md): templates, items as supply-1 tokens on the token
//! standard, royalties and their claims, equip rules (holder vote with notice, performance revert,
//! locked), equipping at launch (signed by the launchpad), loot minting (signed by the war program),
//! and forging. It is the only signer the token program accepts for a mint's slot changes and vote
//! locks (`["slots", mint]`), and it calls the items program for every template rule.

#![allow(unexpected_cfgs)]
#![allow(clippy::result_large_err)]

use anchor_lang::prelude::*;
use bordrless_token::state::Mint;
use hookwars_common::{
    ids, pda, seeds, shape, EquipConfig, Params, PerformanceRule, PARAM_FIELDS,
};

pub mod cpi;
pub mod error;
pub mod events;
pub mod state;

use cpi::*;
use error::ArmoryError;
use events::*;
use state::*;

declare_id!("7xnkyX5B7Ywz86Cu2ofM5T2z2UPmy1qhpgrAuK9Kk5MU");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "Hookwars armory",
    project_url: "https://github.com/BordrlessDex/bordrless-programs",
    contacts: "link:https://github.com/BordrlessDex/bordrless-programs/security/advisories/new",
    policy: "https://github.com/BordrlessDex/bordrless-programs/blob/main/SECURITY.md",
    source_code: "https://github.com/BordrlessDex/bordrless-programs"
}

/// Arguments of `register_template` (02 section 3.1).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct RegisterTemplateArgs {
    /// The template id: `ArmoryConfig.templates + 1` (04 reserves 0 for "none").
    pub id: u16,
    pub code_hash: [u8; 32],
    pub kind: u8,
    pub field_count: u8,
    pub field_min: [u32; PARAM_FIELDS],
    pub field_max: [u32; PARAM_FIELDS],
    pub open_authoring: bool,
    pub loot_enabled: bool,
    pub forge_enabled: bool,
    pub max_level: u8,
    pub loot_royalty_bps: u16,
    pub max_targets: u8,
    pub name: String,
}

/// One slot at launch (02 section 7.1; M2: one slot per call).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct LaunchEquip {
    pub slot: u8,
    pub item: Option<Pubkey>,
    pub config: EquipConfig,
    pub notice_secs: u32,
    pub rule: Option<PerformanceRule>,
}

fn check_params(p: &ArmoryParams) -> Result<()> {
    require!(
        p.max_royalty_bps <= 10_000
            && p.vote_quorum_bps <= 10_000
            && p.forge_gain_bps <= 10_000
            && p.max_pool_item_cut_bps <= 10_000
            && p.max_pool_item_discount_bps <= 10_000
            && p.settle_bounty_bps <= 10_000
            && p.vote_period_secs > 0
            && p.min_twap_secs > 0
            && p.min_notice_secs <= p.max_notice_secs,
        ArmoryError::InvalidParams
    );
    Ok(())
}

/// The upgrade authority of this program, from its ProgramData (as upstream configs are created).
fn upgrade_authority(program_data: &AccountInfo) -> Result<Option<Pubkey>> {
    require_keys_eq!(
        *program_data.key,
        hookwars_common::programdata_address(&crate::ID),
        ArmoryError::NotUpgradeAuthority
    );
    require_keys_eq!(
        *program_data.owner,
        ids::BPF_LOADER_UPGRADEABLE_ID,
        ArmoryError::NotUpgradeAuthority
    );
    let data = program_data.try_borrow_data()?;
    require!(
        data.len() >= 45 && data[..4] == [3, 0, 0, 0],
        ArmoryError::NotUpgradeAuthority
    );
    if data[12] == 0 {
        return Ok(None);
    }
    Ok(Some(Pubkey::new_from_array(data[13..45].try_into().unwrap())))
}

fn now() -> Result<i64> {
    Ok(Clock::get()?.unix_timestamp)
}

/// Instructions.
#[program]
pub mod hookwars_armory {
    use super::*;

    /// Creates the config; the program's upgrade authority signs.
    pub fn init(ctx: Context<Init>, admin: Pubkey, params: ArmoryParams) -> Result<()> {
        let up = upgrade_authority(&ctx.accounts.program_data)?;
        require!(
            up == Some(ctx.accounts.authority.key()),
            ArmoryError::NotUpgradeAuthority
        );
        check_params(&params)?;
        let c = &mut ctx.accounts.config;
        c.version = VERSION;
        c.bump = ctx.bumps.config;
        c.admin = admin;
        c.pending_admin = None;
        c.pending_admin_at = 0;
        c.items_minted = 0;
        c.templates = 0;
        c.params = params;
        c.reserved = [0; 62];
        Ok(())
    }

    /// The admin proposes new params; they apply after `admin_timelock_secs` (D-9).
    pub fn propose_params(ctx: Context<ProposeParams>, params: ArmoryParams) -> Result<()> {
        check_params(&params)?;
        let ready_at = now()? + i64::from(ctx.accounts.config.params.admin_timelock_secs);
        let p = &mut ctx.accounts.pending;
        p.bump = ctx.bumps.pending;
        p.params = params;
        p.ready_at = ready_at;
        emit_cpi!(ParamsProposed { params, ready_at });
        Ok(())
    }

    /// Applies proposed params once the timelock has passed; anyone may send it.
    pub fn apply_params(ctx: Context<ApplyParams>) -> Result<()> {
        let t = now()?;
        require!(t >= ctx.accounts.pending.ready_at, ArmoryError::Timelock);
        let params = ctx.accounts.pending.params;
        ctx.accounts.config.params = params;
        emit_cpi!(ParamsApplied { params, ts: t });
        Ok(())
    }

    /// The admin proposes a new admin, who may accept after the timelock.
    pub fn propose_admin(ctx: Context<AdminOnly>, admin: Pubkey) -> Result<()> {
        let c = &mut ctx.accounts.config;
        let ready_at = now()? + i64::from(c.params.admin_timelock_secs);
        c.pending_admin = Some(admin);
        c.pending_admin_at = ready_at;
        emit_cpi!(AdminProposed { admin, ready_at });
        Ok(())
    }

    /// The proposed admin accepts.
    pub fn accept_admin(ctx: Context<AcceptAdmin>) -> Result<()> {
        let t = now()?;
        let c = &mut ctx.accounts.config;
        require!(
            c.pending_admin == Some(ctx.accounts.new_admin.key()),
            ArmoryError::NotPendingAdmin
        );
        require!(t >= c.pending_admin_at, ArmoryError::Timelock);
        c.admin = ctx.accounts.new_admin.key();
        c.pending_admin = None;
        emit_cpi!(AdminAccepted {
            admin: c.admin,
            ts: t
        });
        Ok(())
    }

    /// Registers a template (02 section 3.1).
    pub fn register_template(
        ctx: Context<RegisterTemplate>,
        args: RegisterTemplateArgs,
    ) -> Result<()> {
        process_register_template(ctx, args)
    }

    /// Retires a template (02 section 3.2).
    pub fn retire_template(ctx: Context<RetireTemplate>, template_id: u16) -> Result<()> {
        let t = &mut ctx.accounts.template;
        require!(t.id == template_id, ArmoryError::WrongAccount);
        t.status = template_status::RETIRED;
        emit_cpi!(TemplateRetired {
            template_id,
            ts: now()?
        });
        Ok(())
    }

    /// Anyone authors an item from an open template (02 section 4.2).
    pub fn create_item(
        ctx: Context<CreateItem>,
        template_id: u16,
        params: [u32; PARAM_FIELDS],
        royalty_bps: u16,
    ) -> Result<()> {
        process_create_item(ctx, template_id, params, royalty_bps)
    }

    /// Hookwars M3b: anyone authors a composite item (08 section 2) from open templates. The
    /// remaining accounts are each module's `Template`, in module order.
    pub fn create_composite<'info>(
        ctx: Context<'info, CreateComposite<'info>>,
        modules: Vec<hookwars_common::composite::Module>,
        royalty_bps: u16,
    ) -> Result<()> {
        process_create_composite(ctx, modules, royalty_bps)
    }

    /// The war program mints a loot item (02 section 4.3).
    pub fn mint_loot(
        ctx: Context<MintLoot>,
        template_id: u16,
        params: [u32; PARAM_FIELDS],
    ) -> Result<()> {
        process_mint_loot(ctx, template_id, params)
    }

    /// The item's holder claims royalty from one of its royalty holdings (02 section 5.2).
    pub fn claim_royalty<'info>(
        ctx: Context<'info, ClaimRoyalty<'info>>,
        amount: u64,
    ) -> Result<()> {
        process_claim_royalty(ctx, amount)
    }

    /// The launchpad equips one slot of a fresh mint (02 section 7.1).
    pub fn equip_launch(ctx: Context<EquipLaunch>, entry: LaunchEquip) -> Result<()> {
        process_equip_launch(ctx, entry)
    }

    /// Proposes an item (or emptying) for a slot (02 section 6.2).
    pub fn propose(
        ctx: Context<Propose>,
        slot: u8,
        item: Option<Pubkey>,
        equip_config: EquipConfig,
    ) -> Result<()> {
        process_propose(ctx, slot, item, equip_config)
    }

    /// Votes by locking tokens in place (02 section 6.3).
    pub fn vote(ctx: Context<Vote>, support: bool, amount: u64) -> Result<()> {
        process_vote(ctx, support, amount)
    }

    /// Resolves a proposal once voting ends (02 section 6.4).
    pub fn finalize(ctx: Context<Finalize>) -> Result<()> {
        process_finalize(ctx)
    }

    /// Applies a passed proposal after its notice (02 section 6.4).
    pub fn execute(ctx: Context<Execute>) -> Result<()> {
        process_execute(ctx)
    }

    /// Fails a passed proposal that no longer fits (02 section 6.4).
    pub fn fail_stale(ctx: Context<FailStale>) -> Result<()> {
        process_fail_stale(ctx)
    }

    /// The proposer cancels an open proposal with no votes (02 section 6.5).
    pub fn cancel(ctx: Context<Cancel>) -> Result<()> {
        let t = now()?;
        let p = &mut ctx.accounts.proposal;
        require!(p.status == proposal_status::OPEN, ArmoryError::NotExecutable);
        require!(
            p.votes_for == 0 && p.votes_against == 0,
            ArmoryError::VotesExist
        );
        p.status = proposal_status::CANCELLED;
        ctx.accounts.slot_state.open_proposal = None;
        emit_cpi!(ProposalResolved {
            proposal: p.key(),
            status: p.status,
            votes_for: 0,
            votes_against: 0,
            eligible: 0,
            ts: t
        });
        Ok(())
    }

    /// Closes a final proposal once its vote locks are closed; rent to the proposer (02 6.6).
    pub fn close_proposal(ctx: Context<CloseProposal>) -> Result<()> {
        let p = &ctx.accounts.proposal;
        require!(
            matches!(
                p.status,
                proposal_status::FAILED | proposal_status::EXECUTED | proposal_status::CANCELLED
            ),
            ArmoryError::ProposalNotFinal
        );
        require!(p.voters_open == 0, ArmoryError::VotesOpen);
        Ok(())
    }

    /// Closes a vote lock of a final proposal; rent to the voter (02 section 6.6).
    pub fn close_vote(ctx: Context<CloseVote>) -> Result<()> {
        let p = &mut ctx.accounts.proposal;
        require!(
            matches!(
                p.status,
                proposal_status::FAILED | proposal_status::EXECUTED | proposal_status::CANCELLED
            ),
            ArmoryError::ProposalNotFinal
        );
        p.voters_open = p.voters_open.saturating_sub(1);
        emit_cpi!(VoteUnlocked {
            proposal: p.key(),
            voter: ctx.accounts.vote_lock.voter,
            amount: ctx.accounts.vote_lock.amount
        });
        Ok(())
    }

    /// Checks a `Performance` slot's condition and reverts it when due (02 section 6.7).
    pub fn check_performance(ctx: Context<CheckPerformance>) -> Result<()> {
        process_check_performance(ctx)
    }

    /// Forges two items of one template into one (02 section 9).
    pub fn forge(ctx: Context<Forge>) -> Result<()> {
        process_forge(ctx)
    }
}

// ------------------------------------------------------------------------------ contexts

#[derive(Accounts)]
pub struct Init<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(init, payer = authority, space = 8 + ArmoryConfig::INIT_SPACE, seeds = [seeds::CONFIG], bump)]
    pub config: Box<Account<'info, ArmoryConfig>>,
    /// CHECK: this program's ProgramData, parsed in the handler.
    pub program_data: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct ProposeParams<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump, has_one = admin @ ArmoryError::NotAdmin)]
    pub config: Box<Account<'info, ArmoryConfig>>,
    #[account(init_if_needed, payer = admin, space = 8 + PendingParams::INIT_SPACE, seeds = [seeds::PENDING], bump)]
    pub pending: Box<Account<'info, PendingParams>>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct ApplyParams<'info> {
    /// CHECK: receives the pending account's rent (the admin who proposed).
    #[account(mut, address = config.admin @ ArmoryError::NotAdmin)]
    pub admin: UncheckedAccount<'info>,
    #[account(mut, seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, ArmoryConfig>>,
    #[account(mut, seeds = [seeds::PENDING], bump = pending.bump, close = admin)]
    pub pending: Box<Account<'info, PendingParams>>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct AdminOnly<'info> {
    pub admin: Signer<'info>,
    #[account(mut, seeds = [seeds::CONFIG], bump = config.bump, has_one = admin @ ArmoryError::NotAdmin)]
    pub config: Box<Account<'info, ArmoryConfig>>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct AcceptAdmin<'info> {
    pub new_admin: Signer<'info>,
    #[account(mut, seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, ArmoryConfig>>,
}

#[event_cpi]
#[derive(Accounts)]
#[instruction(args: RegisterTemplateArgs)]
pub struct RegisterTemplate<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(mut, seeds = [seeds::CONFIG], bump = config.bump, has_one = admin @ ArmoryError::NotAdmin)]
    pub config: Box<Account<'info, ArmoryConfig>>,
    #[account(init, payer = admin, space = 8 + Template::INIT_SPACE,
        seeds = [seeds::TEMPLATE, &args.id.to_le_bytes()], bump)]
    pub template: Box<Account<'info, Template>>,
    /// CHECK: the template's hook program (checked in the handler).
    pub template_program: UncheckedAccount<'info>,
    /// CHECK: its ProgramData (checked in the handler).
    pub programdata: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct RetireTemplate<'info> {
    pub admin: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump, has_one = admin @ ArmoryError::NotAdmin)]
    pub config: Box<Account<'info, ArmoryConfig>>,
    #[account(mut, seeds = [seeds::TEMPLATE, &template.id.to_le_bytes()], bump = template.bump)]
    pub template: Box<Account<'info, Template>>,
}

/// Token-program accounts every path that calls it takes.
#[derive(Accounts)]
pub struct TokenAccounts<'info> {
    /// CHECK: the token program.
    #[account(address = bordrless_token::ID)]
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: its event authority.
    #[account(address = bordrless_token::client::event_authority())]
    pub token_event_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> TokenAccounts<'info> {
    fn infos(&self) -> (AccountInfo<'info>, AccountInfo<'info>, AccountInfo<'info>) {
        (
            self.token_program.to_account_info(),
            self.token_event_authority.to_account_info(),
            self.system_program.to_account_info(),
        )
    }
}

#[event_cpi]
#[derive(Accounts)]
#[instruction(template_id: u16)]
pub struct CreateItem<'info> {
    #[account(mut)]
    pub author: Signer<'info>,
    #[account(mut, seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, ArmoryConfig>>,
    #[account(seeds = [seeds::TEMPLATE, &template_id.to_le_bytes()], bump = template.bump)]
    pub template: Box<Account<'info, Template>>,
    /// CHECK: `["minter"]`.
    #[account(address = MINTER)]
    pub minter: UncheckedAccount<'info>,
    /// CHECK: `["item-mint", items_minted]`, created here.
    #[account(mut, seeds = [seeds::ITEM_MINT, &config.items_minted.to_le_bytes()], bump)]
    pub item_mint: UncheckedAccount<'info>,
    #[account(init, payer = author, space = 8 + Item::INIT_SPACE,
        seeds = [seeds::ITEM, item_mint.key().as_ref()], bump)]
    pub item: Box<Account<'info, Item>>,
    /// CHECK: the author's holding of the item mint, created here.
    #[account(mut)]
    pub recipient_holding: UncheckedAccount<'info>,
    /// CHECK: `["armory"]`.
    #[account(address = ARMORY_SIGNER)]
    pub armory_signer: UncheckedAccount<'info>,
    /// CHECK: the items program.
    #[account(address = ids::ITEMS_ID)]
    pub items_program: UncheckedAccount<'info>,
    pub token: TokenAccounts<'info>,
    pub system_program: Program<'info, System>,
}

/// Accounts of `create_composite` (M3b).
#[event_cpi]
#[derive(Accounts)]
pub struct CreateComposite<'info> {
    #[account(mut)]
    pub author: Signer<'info>,
    #[account(mut, seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, ArmoryConfig>>,
    /// The Composite template (id 41).
    #[account(seeds = [seeds::TEMPLATE, &hookwars_common::template_id::COMPOSITE.to_le_bytes()], bump = template.bump)]
    pub template: Box<Account<'info, Template>>,
    /// CHECK: `["minter"]`.
    #[account(address = MINTER)]
    pub minter: UncheckedAccount<'info>,
    /// CHECK: `["item-mint", items_minted]`, created here.
    #[account(mut, seeds = [seeds::ITEM_MINT, &config.items_minted.to_le_bytes()], bump)]
    pub item_mint: UncheckedAccount<'info>,
    #[account(init, payer = author, space = 8 + Item::INIT_SPACE,
        seeds = [seeds::ITEM, item_mint.key().as_ref()], bump)]
    pub item: Box<Account<'info, Item>>,
    #[account(init, payer = author, space = hookwars_common::composite::CompositeItem::SPACE,
        seeds = [hookwars_common::composite::SEED, item.key().as_ref()], bump)]
    pub composite: Box<Account<'info, CompositeItem>>,
    /// CHECK: the author's holding of the item mint, created here.
    #[account(mut)]
    pub recipient_holding: UncheckedAccount<'info>,
    pub token: TokenAccounts<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
#[instruction(template_id: u16)]
pub struct MintLoot<'info> {
    /// The war program's `["loot-signer"]`.
    #[account(address = LOOT_SIGNER @ ArmoryError::NotLootSigner)]
    pub loot_signer: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: who receives the item.
    pub owner: UncheckedAccount<'info>,
    #[account(mut, seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, ArmoryConfig>>,
    #[account(seeds = [seeds::TEMPLATE, &template_id.to_le_bytes()], bump = template.bump)]
    pub template: Box<Account<'info, Template>>,
    /// CHECK: `["minter"]`.
    #[account(address = MINTER)]
    pub minter: UncheckedAccount<'info>,
    /// CHECK: `["item-mint", items_minted]`.
    #[account(mut, seeds = [seeds::ITEM_MINT, &config.items_minted.to_le_bytes()], bump)]
    pub item_mint: UncheckedAccount<'info>,
    #[account(init, payer = payer, space = 8 + Item::INIT_SPACE,
        seeds = [seeds::ITEM, item_mint.key().as_ref()], bump)]
    pub item: Box<Account<'info, Item>>,
    /// CHECK: the owner's holding of the item mint.
    #[account(mut)]
    pub recipient_holding: UncheckedAccount<'info>,
    /// CHECK: `["armory"]`.
    #[account(address = ARMORY_SIGNER)]
    pub armory_signer: UncheckedAccount<'info>,
    /// CHECK: the items program.
    #[account(address = ids::ITEMS_ID)]
    pub items_program: UncheckedAccount<'info>,
    pub token: TokenAccounts<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct ClaimRoyalty<'info> {
    #[account(mut)]
    pub claimant: Signer<'info>,
    pub item: Box<Account<'info, Item>>,
    /// CHECK: the claimant's holding of the item mint (read in the handler).
    pub item_holding: UncheckedAccount<'info>,
    /// CHECK: `["royalty", item]`.
    #[account(seeds = [seeds::ROYALTY, item.key().as_ref()], bump = item.royalty_owner_bump)]
    pub royalty_owner: UncheckedAccount<'info>,
    /// CHECK: the cut mint.
    pub cut_mint: UncheckedAccount<'info>,
    /// CHECK: `["holding", cut_mint, royalty_owner]`.
    #[account(mut)]
    pub royalty_holding: UncheckedAccount<'info>,
    /// CHECK: the claimant's holding of the cut mint, created if missing.
    #[account(mut)]
    pub destination: UncheckedAccount<'info>,
    pub token: TokenAccounts<'info>,
}

/// The accounts every equip path takes (02 section 6.4).
#[derive(Accounts)]
pub struct EquipCtx<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: the token (read with `read_mint`).
    #[account(mut, owner = bordrless_token::ID)]
    pub token_mint: UncheckedAccount<'info>,
    /// CHECK: `["slots", token_mint]` (checked in the handler).
    pub slot_authority: UncheckedAccount<'info>,
    /// CHECK: the slot's `EquipState` under items.
    #[account(mut)]
    pub equip_state: UncheckedAccount<'info>,
    #[account(mut)]
    pub old_item: Option<Box<Account<'info, Item>>>,
    /// CHECK: the outgoing item's equip vault, when it may cut on the token side.
    pub old_equip_vault: Option<UncheckedAccount<'info>>,
    #[account(mut)]
    pub new_item: Option<Box<Account<'info, Item>>>,
    pub new_template: Option<Box<Account<'info, Template>>>,
    /// CHECK: the new template's program.
    pub template_program: Option<UncheckedAccount<'info>>,
    /// CHECK: its ProgramData.
    pub template_programdata: Option<UncheckedAccount<'info>>,
    /// CHECK: the new item's registry under items.
    #[account(mut)]
    pub registry: Option<UncheckedAccount<'info>>,
    /// CHECK: the new item's equip vault.
    #[account(mut)]
    pub new_equip_vault: Option<UncheckedAccount<'info>>,
    /// CHECK: `["royalty", new_item]`.
    pub royalty_owner: Option<UncheckedAccount<'info>>,
    /// CHECK: its holding of the token.
    #[account(mut)]
    pub royalty_holding_token: Option<UncheckedAccount<'info>>,
    /// CHECK: bridged SOL.
    pub quote_mint: Option<UncheckedAccount<'info>>,
    /// CHECK: the royalty owner's holding of bridged SOL.
    #[account(mut)]
    pub royalty_holding_quote: Option<UncheckedAccount<'info>>,
    /// CHECK: Hookwars M3b: the new item's module list (`["composite", new_item]`) when it is a
    /// composite; read by the items program's `init_equip`.
    pub new_composite: Option<UncheckedAccount<'info>>,
    /// CHECK: `["armory"]`.
    #[account(address = ARMORY_SIGNER)]
    pub armory_signer: UncheckedAccount<'info>,
    /// CHECK: the items program.
    #[account(address = ids::ITEMS_ID)]
    pub items_program: UncheckedAccount<'info>,
    pub token: TokenAccounts<'info>,
}

#[event_cpi]
#[derive(Accounts)]
#[instruction(entry: LaunchEquip)]
pub struct EquipLaunch<'info> {
    /// The launchpad's `["armory-caller", mint]` (checked in the handler).
    pub launch_caller: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, ArmoryConfig>>,
    #[account(init, payer = equip.payer, space = SlotState::space(entry.config.targets.len()),
        seeds = [seeds::SLOT_STATE, equip.token_mint.key().as_ref(), &[entry.slot]], bump)]
    pub slot_state: Box<Account<'info, SlotState>>,
    pub equip: EquipCtx<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
#[instruction(slot: u8, item: Option<Pubkey>, equip_config: EquipConfig)]
pub struct Propose<'info> {
    #[account(mut)]
    pub proposer: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, ArmoryConfig>>,
    /// CHECK: the token (read with `read_mint`).
    #[account(owner = bordrless_token::ID)]
    pub token_mint: UncheckedAccount<'info>,
    #[account(mut, seeds = [seeds::SLOT_STATE, token_mint.key().as_ref(), &[slot]], bump = slot_state.bump)]
    pub slot_state: Box<Account<'info, SlotState>>,
    #[account(init, payer = proposer, space = Proposal::space(equip_config.targets.len()),
        seeds = [seeds::PROPOSAL, token_mint.key().as_ref(), &[slot], &slot_state.next_nonce.to_le_bytes()], bump)]
    pub proposal: Box<Account<'info, Proposal>>,
    pub item: Option<Box<Account<'info, Item>>>,
    pub template: Option<Box<Account<'info, Template>>>,
    /// CHECK: the template's program.
    pub template_program: Option<UncheckedAccount<'info>>,
    /// CHECK: its ProgramData.
    pub template_programdata: Option<UncheckedAccount<'info>>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct Vote<'info> {
    #[account(mut)]
    pub voter: Signer<'info>,
    #[account(mut)]
    pub proposal: Box<Account<'info, Proposal>>,
    #[account(init, payer = voter, space = 8 + VoteLock::INIT_SPACE,
        seeds = [seeds::VOTE, proposal.key().as_ref(), voter.key().as_ref()], bump)]
    pub vote_lock: Box<Account<'info, VoteLock>>,
    /// CHECK: the voter's holding of the token (read in the handler).
    #[account(mut)]
    pub holding: UncheckedAccount<'info>,
    /// CHECK: the token.
    #[account(address = proposal.mint @ ArmoryError::WrongAccount)]
    pub token_mint: UncheckedAccount<'info>,
    /// CHECK: `["slots", token_mint]`.
    #[account(seeds = [seeds::SLOTS, token_mint.key().as_ref()], bump)]
    pub slot_authority: UncheckedAccount<'info>,
    pub token: TokenAccounts<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct Finalize<'info> {
    #[account(seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, ArmoryConfig>>,
    #[account(mut)]
    pub proposal: Box<Account<'info, Proposal>>,
    #[account(mut, seeds = [seeds::SLOT_STATE, proposal.mint.as_ref(), &[proposal.slot]], bump = slot_state.bump)]
    pub slot_state: Box<Account<'info, SlotState>>,
    /// CHECK: the token.
    #[account(address = proposal.mint @ ArmoryError::WrongAccount)]
    pub token_mint: UncheckedAccount<'info>,
    /// CHECK: the token's `Launch` (`["launch", mint]` under the launchpad), if it launched.
    pub launch: Option<UncheckedAccount<'info>>,
    /// CHECK: the launch pool's base vault (`holding(mint, pool)`).
    pub pool_base_vault: Option<UncheckedAccount<'info>>,
    /// CHECK: the launch PDA's holding of the token.
    pub launch_holding: Option<UncheckedAccount<'info>>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct Execute<'info> {
    #[account(seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, ArmoryConfig>>,
    #[account(mut)]
    pub proposal: Box<Account<'info, Proposal>>,
    #[account(mut, seeds = [seeds::SLOT_STATE, proposal.mint.as_ref(), &[proposal.slot]], bump = slot_state.bump)]
    pub slot_state: Box<Account<'info, SlotState>>,
    pub equip: EquipCtx<'info>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct FailStale<'info> {
    #[account(seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, ArmoryConfig>>,
    #[account(mut)]
    pub proposal: Box<Account<'info, Proposal>>,
    #[account(mut, seeds = [seeds::SLOT_STATE, proposal.mint.as_ref(), &[proposal.slot]], bump = slot_state.bump)]
    pub slot_state: Box<Account<'info, SlotState>>,
    /// CHECK: the token.
    #[account(address = proposal.mint @ ArmoryError::WrongAccount)]
    pub token_mint: UncheckedAccount<'info>,
    pub item: Option<Box<Account<'info, Item>>>,
    pub template: Option<Box<Account<'info, Template>>>,
    /// CHECK: the template's program.
    pub template_program: Option<UncheckedAccount<'info>>,
    /// CHECK: its ProgramData.
    pub template_programdata: Option<UncheckedAccount<'info>>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct Cancel<'info> {
    pub proposer: Signer<'info>,
    #[account(mut, has_one = proposer @ ArmoryError::WrongAccount)]
    pub proposal: Box<Account<'info, Proposal>>,
    #[account(mut, seeds = [seeds::SLOT_STATE, proposal.mint.as_ref(), &[proposal.slot]], bump = slot_state.bump)]
    pub slot_state: Box<Account<'info, SlotState>>,
}

#[derive(Accounts)]
pub struct CloseProposal<'info> {
    /// CHECK: receives the rent.
    #[account(mut, address = proposal.proposer @ ArmoryError::WrongAccount)]
    pub proposer: UncheckedAccount<'info>,
    #[account(mut, close = proposer)]
    pub proposal: Box<Account<'info, Proposal>>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct CloseVote<'info> {
    /// CHECK: receives the rent.
    #[account(mut, address = vote_lock.voter @ ArmoryError::WrongAccount)]
    pub voter: UncheckedAccount<'info>,
    #[account(mut)]
    pub proposal: Box<Account<'info, Proposal>>,
    #[account(mut, close = voter, has_one = proposal @ ArmoryError::WrongAccount)]
    pub vote_lock: Box<Account<'info, VoteLock>>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct CheckPerformance<'info> {
    #[account(seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, ArmoryConfig>>,
    #[account(mut)]
    pub slot_state: Box<Account<'info, SlotState>>,
    /// CHECK: `["launch", mint]` under the launchpad (read raw: pool at offset 74).
    pub launch: UncheckedAccount<'info>,
    /// The launch pool (its observation ring is in its tail, 03 M3a notes).
    pub pool: Box<Account<'info, bordrless_swap::state::Pool>>,
    /// The slot's open proposal, cancelled by a revert.
    #[account(mut)]
    pub open_proposal: Option<Box<Account<'info, Proposal>>>,
    pub equip: EquipCtx<'info>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct Forge<'info> {
    #[account(mut)]
    pub forger: Signer<'info>,
    #[account(mut, seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, ArmoryConfig>>,
    #[account(init_if_needed, payer = forger, space = 8 + ForgeCounter::INIT_SPACE,
        seeds = [seeds::FORGES, forger.key().as_ref()], bump)]
    pub forge_counter: Box<Account<'info, ForgeCounter>>,
    #[account(mut, close = forger)]
    pub item_a: Box<Account<'info, Item>>,
    #[account(mut, close = forger)]
    pub item_b: Box<Account<'info, Item>>,
    /// CHECK: item a's mint.
    #[account(mut, address = item_a.item_mint @ ArmoryError::WrongAccount)]
    pub mint_a: UncheckedAccount<'info>,
    /// CHECK: item b's mint.
    #[account(mut, address = item_b.item_mint @ ArmoryError::WrongAccount)]
    pub mint_b: UncheckedAccount<'info>,
    /// CHECK: the forger's holding of item a.
    #[account(mut)]
    pub holding_a: UncheckedAccount<'info>,
    /// CHECK: the forger's holding of item b.
    #[account(mut)]
    pub holding_b: UncheckedAccount<'info>,
    #[account(seeds = [seeds::TEMPLATE, &item_a.template_id.to_le_bytes()], bump = template.bump)]
    pub template: Box<Account<'info, Template>>,
    /// CHECK: `["minter"]`.
    #[account(address = MINTER)]
    pub minter: UncheckedAccount<'info>,
    /// CHECK: `["item-mint", items_minted]`.
    #[account(mut, seeds = [seeds::ITEM_MINT, &config.items_minted.to_le_bytes()], bump)]
    pub item_mint: UncheckedAccount<'info>,
    #[account(init, payer = forger, space = 8 + Item::INIT_SPACE,
        seeds = [seeds::ITEM, item_mint.key().as_ref()], bump)]
    pub item: Box<Account<'info, Item>>,
    /// CHECK: the forger's holding of the new item mint.
    #[account(mut)]
    pub recipient_holding: UncheckedAccount<'info>,
    /// CHECK: `["armory"]`.
    #[account(address = ARMORY_SIGNER)]
    pub armory_signer: UncheckedAccount<'info>,
    /// CHECK: the items program.
    #[account(address = ids::ITEMS_ID)]
    pub items_program: UncheckedAccount<'info>,
    pub token: TokenAccounts<'info>,
    pub system_program: Program<'info, System>,
}

// ------------------------------------------------------------------------------ handlers

fn process_register_template(
    ctx: Context<RegisterTemplate>,
    args: RegisterTemplateArgs,
) -> Result<()> {
    let config = &mut ctx.accounts.config;
    // Hookwars M3b: ids follow the template list (04, 08 section 6), so they may be sparse; each id
    // is registered once (its `Template` account is created here) and must be a known template.
    require!(
        args.id >= 1 && hookwars_common::shape(args.id).is_some(),
        ArmoryError::InvalidSchema
    );
    let program = ctx.accounts.template_program.to_account_info();
    let deploy_slot = hookwars_common::check_program_authority(
        &program,
        &ctx.accounts.programdata.to_account_info(),
    )
    .map_err(|e| match e {
        hookwars_common::AuthorityError::Upgradeable => ArmoryError::TemplateUpgradeable,
        hookwars_common::AuthorityError::ProgramDataMissing => ArmoryError::ProgramDataMissing,
    })?;
    require!(program.executable, ArmoryError::InvalidTemplateProgram);
    let protocol = [
        ids::TOKEN_ID,
        ids::SWAP_ID,
        ids::LAUNCH_ID,
        ids::KIT_ID,
        ids::BRIDGE_ID,
        ids::COMPANION_ID,
        crate::ID,
        ids::WAR_ID,
        anchor_lang::system_program::ID,
        Pubkey::default(),
    ];
    require!(
        !protocol.contains(&program.key()),
        ArmoryError::InvalidTemplateProgram
    );
    require!(
        args.kind <= hookwars_common::kind::WAR && args.kind != hookwars_common::kind::LOCKED,
        ArmoryError::InvalidKind
    );
    require!(
        usize::from(args.field_count) <= PARAM_FIELDS && args.max_level >= 1,
        ArmoryError::InvalidSchema
    );
    for i in 0..PARAM_FIELDS {
        if i < usize::from(args.field_count) {
            require!(
                args.field_min[i] <= args.field_max[i],
                ArmoryError::InvalidSchema
            );
        } else {
            require!(
                args.field_min[i] == 0 && args.field_max[i] == 0,
                ArmoryError::InvalidSchema
            );
        }
    }
    if program.key() == ids::ITEMS_ID {
        let s = shape(args.id).ok_or(ArmoryError::InvalidSchema)?;
        require!(
            s.kind == args.kind && s.field_count == args.field_count,
            ArmoryError::InvalidSchema
        );
        require!(
            s.forgeable || !args.forge_enabled,
            ArmoryError::InvalidSchema
        );
    }
    require!(
        args.loot_royalty_bps <= config.params.max_royalty_bps,
        ArmoryError::RoyaltyTooHigh
    );
    require!(args.name.len() <= 32, ArmoryError::InvalidSchema);
    config.templates = args.id;
    let t = &mut ctx.accounts.template;
    let ts = now()?;
    t.version = VERSION;
    t.bump = ctx.bumps.template;
    t.id = args.id;
    t.program = program.key();
    t.code_hash = args.code_hash;
    t.deploy_slot = deploy_slot;
    t.kind = args.kind;
    t.field_count = args.field_count;
    t.field_min = args.field_min;
    t.field_max = args.field_max;
    t.open_authoring = args.open_authoring;
    t.loot_enabled = args.loot_enabled;
    t.forge_enabled = args.forge_enabled;
    t.max_level = args.max_level;
    t.loot_royalty_bps = args.loot_royalty_bps;
    t.max_targets = args.max_targets;
    t.status = template_status::ACTIVE;
    t.name = args.name.clone();
    t.registered_by = ctx.accounts.admin.key();
    t.created_at = ts;
    t.reserved = [0; 32];
    emit_cpi!(TemplateRegistered {
        template_id: args.id,
        program: program.key(),
        code_hash: args.code_hash,
        deploy_slot,
        kind: args.kind,
        field_count: args.field_count,
        field_min: args.field_min,
        field_max: args.field_max,
        name: args.name,
        ts
    });
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_item(
    item: &mut Item,
    item_key: Pubkey,
    bump: u8,
    item_mint: Pubkey,
    template_id: u16,
    params: [u32; PARAM_FIELDS],
    manifest: hookwars_common::Manifest,
    author: Pubkey,
    royalty_bps: u16,
    level: u8,
    source_: u8,
    ts: i64,
) {
    item.version = VERSION;
    item.bump = bump;
    item.item_mint = item_mint;
    item.template_id = template_id;
    item.params = params;
    item.manifest = manifest;
    item.author = author;
    item.royalty_bps = royalty_bps;
    item.level = level;
    item.source = source_;
    item.equipped_count = 0;
    item.royalty_owner_bump = pda::royalty_owner(&item_key).1;
    item.created_at = ts;
    item.reserved = [0; 32];
}

fn process_create_item(
    ctx: Context<CreateItem>,
    template_id: u16,
    params: [u32; PARAM_FIELDS],
    royalty_bps: u16,
) -> Result<()> {
    let a = &ctx.accounts;
    require!(a.template.open_authoring, ArmoryError::TemplateClosed);
    require!(
        royalty_bps <= a.config.params.max_royalty_bps,
        ArmoryError::RoyaltyTooHigh
    );
    let signer = a.armory_signer.to_account_info();
    let items = a.items_program.to_account_info();
    let manifest = validate_item(&signer, &items, &a.template, &params)?;
    let (tp, ea, sp) = a.token.infos();
    let t = TokenInfos {
        token_program: &tp,
        event_authority: &ea,
        system_program: &sp,
    };
    let n = a.config.items_minted;
    mint_item(
        &t,
        &a.author.to_account_info(),
        &a.minter.to_account_info(),
        &a.item_mint.to_account_info(),
        ctx.bumps.item_mint,
        n,
        &a.author.to_account_info(),
        &a.recipient_holding.to_account_info(),
        &a.template.name,
    )?;
    let ts = now()?;
    let item_key = ctx.accounts.item.key();
    let item_mint = ctx.accounts.item_mint.key();
    let author = ctx.accounts.author.key();
    write_item(
        &mut ctx.accounts.item,
        item_key,
        ctx.bumps.item,
        item_mint,
        template_id,
        params,
        manifest,
        author,
        royalty_bps,
        1,
        source::AUTHORED,
        ts,
    );
    ctx.accounts.config.items_minted = n + 1;
    emit_cpi!(ItemCreated {
        item: item_key,
        item_mint,
        template_id,
        params,
        manifest,
        author,
        royalty_bps,
        level: 1,
        source: source::AUTHORED,
        ts
    });
    Ok(())
}

/// Hookwars M3b: `create_composite` (08 sections 2.9 and 2.10).
fn process_create_composite<'info>(
    ctx: Context<'info, CreateComposite<'info>>,
    modules: Vec<hookwars_common::composite::Module>,
    royalty_bps: u16,
) -> Result<()> {
    use hookwars_common::composite::{validate_modules, CompositeError};
    let a = &ctx.accounts;
    require!(
        a.template.status == template_status::ACTIVE && a.template.open_authoring,
        ArmoryError::TemplateClosed
    );
    require!(
        royalty_bps <= a.config.params.max_royalty_bps,
        ArmoryError::RoyaltyTooHigh
    );
    require!(ctx.remaining_accounts.len() == modules.len(), ArmoryError::WrongAccount);
    let mut fields: Vec<(u16, Params, Params)> = Vec::with_capacity(modules.len());
    for (m, info) in modules.iter().zip(ctx.remaining_accounts.iter()) {
        require_keys_eq!(*info.owner, crate::ID, ArmoryError::WrongAccount);
        require_keys_eq!(info.key(), pda::template(m.template_id).0, ArmoryError::WrongAccount);
        let t = Template::try_deserialize(&mut &info.try_borrow_data()?[..])?;
        require!(
            t.status == template_status::ACTIVE && t.open_authoring,
            ArmoryError::TemplateClosed
        );
        fields.push((t.id, t.field_min, t.field_max));
    }
    let lookup = |id: u16| fields.iter().find(|f| f.0 == id).map(|f| (f.1, f.2));
    let manifest = validate_modules(&modules, lookup, a.template.max_targets, 63).map_err(|e| match e {
        CompositeError::BadParams => error!(ArmoryError::ParamOutOfRange),
        CompositeError::KindMismatch => error!(ArmoryError::KindMismatch),
        _ => error!(ArmoryError::InvalidSchema),
    })?;
    let (tp, ea, sp) = a.token.infos();
    let t = TokenInfos {
        token_program: &tp,
        event_authority: &ea,
        system_program: &sp,
    };
    let n = a.config.items_minted;
    mint_item(
        &t,
        &a.author.to_account_info(),
        &a.minter.to_account_info(),
        &a.item_mint.to_account_info(),
        ctx.bumps.item_mint,
        n,
        &a.author.to_account_info(),
        &a.recipient_holding.to_account_info(),
        &a.template.name,
    )?;
    let ts = now()?;
    let item_key = ctx.accounts.item.key();
    let item_mint = ctx.accounts.item_mint.key();
    let author = ctx.accounts.author.key();
    let mut params = [0u32; PARAM_FIELDS];
    params[0] = modules.len() as u32;
    params[1] = 1;
    let template_id = hookwars_common::template_id::COMPOSITE;
    write_item(
        &mut ctx.accounts.item,
        item_key,
        ctx.bumps.item,
        item_mint,
        template_id,
        params,
        manifest,
        author,
        royalty_bps,
        1,
        source::AUTHORED,
        ts,
    );
    let c = &mut ctx.accounts.composite;
    c.version = VERSION;
    c.bump = ctx.bumps.composite;
    c.item = item_key;
    c.modules = modules;
    c.provenance = Vec::new();
    ctx.accounts.config.items_minted = n + 1;
    emit_cpi!(ItemCreated {
        item: item_key,
        item_mint,
        template_id,
        params,
        manifest,
        author,
        royalty_bps,
        level: 1,
        source: source::AUTHORED,
        ts
    });
    Ok(())
}

fn process_mint_loot(ctx: Context<MintLoot>, template_id: u16, params: Params) -> Result<()> {
    let a = &ctx.accounts;
    require!(a.template.loot_enabled, ArmoryError::TemplateClosed);
    let signer = a.armory_signer.to_account_info();
    let items = a.items_program.to_account_info();
    let manifest = validate_item(&signer, &items, &a.template, &params)?;
    let (tp, ea, sp) = a.token.infos();
    let t = TokenInfos {
        token_program: &tp,
        event_authority: &ea,
        system_program: &sp,
    };
    let n = a.config.items_minted;
    mint_item(
        &t,
        &a.payer.to_account_info(),
        &a.minter.to_account_info(),
        &a.item_mint.to_account_info(),
        ctx.bumps.item_mint,
        n,
        &a.owner.to_account_info(),
        &a.recipient_holding.to_account_info(),
        &a.template.name,
    )?;
    let ts = now()?;
    let royalty_bps = a.template.loot_royalty_bps;
    let item_key = ctx.accounts.item.key();
    let item_mint = ctx.accounts.item_mint.key();
    let owner = ctx.accounts.owner.key();
    write_item(
        &mut ctx.accounts.item,
        item_key,
        ctx.bumps.item,
        item_mint,
        template_id,
        params,
        manifest,
        LOOT_SIGNER,
        royalty_bps,
        1,
        source::LOOT,
        ts,
    );
    ctx.accounts.config.items_minted = n + 1;
    emit_cpi!(ItemCreated {
        item: item_key,
        item_mint,
        template_id,
        params,
        manifest,
        author: LOOT_SIGNER,
        royalty_bps,
        level: 1,
        source: source::LOOT,
        ts
    });
    emit_cpi!(LootMinted {
        item: item_key,
        owner,
        template_id,
        params,
        ts
    });
    Ok(())
}

fn process_claim_royalty<'info>(
    ctx: Context<'info, ClaimRoyalty<'info>>,
    amount: u64,
) -> Result<()> {
    let a = &ctx.accounts;
    let claimant = a.claimant.key();
    let item_key = a.item.key();
    // The claimant holds the item.
    let h = bordrless_token::client::read_holding(&a.item_holding.to_account_info())
        .map_err(|_| ArmoryError::NotItemOwner)?;
    require!(
        h.mint == a.item.item_mint && h.owner == claimant && h.amount == 1 && !h.frozen,
        ArmoryError::NotItemOwner
    );
    let cut_mint = a.cut_mint.key();
    let owner_key = a.royalty_owner.key();
    require_keys_eq!(
        a.royalty_holding.key(),
        pda::holding(&cut_mint, &owner_key),
        ArmoryError::WrongAccount
    );
    require_keys_eq!(
        a.destination.key(),
        pda::holding(&cut_mint, &claimant),
        ArmoryError::WrongAccount
    );
    let balance = bordrless_token::client::read_holding(&a.royalty_holding.to_account_info())?.amount;
    let amount = if amount == u64::MAX { balance } else { amount };
    require!(
        amount > 0 && amount <= balance,
        ArmoryError::InsufficientRoyalty
    );
    let (tp, ea, sp) = a.token.infos();
    let t = TokenInfos {
        token_program: &tp,
        event_authority: &ea,
        system_program: &sp,
    };
    create_holding(
        &t,
        &a.claimant.to_account_info(),
        &a.cut_mint.to_account_info(),
        &a.claimant.to_account_info(),
        &a.destination.to_account_info(),
    )?;
    let bump = [a.item.royalty_owner_bump];
    let seeds_vec: Vec<Vec<u8>> = vec![
        seeds::ROYALTY.to_vec(),
        item_key.to_bytes().to_vec(),
        bump.to_vec(),
    ];
    let extras: Vec<_> = ctx
        .remaining_accounts
        .iter()
        .map(|i| {
            if i.is_writable {
                anchor_lang::solana_program::instruction::AccountMeta::new(*i.key, false)
            } else {
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(*i.key, false)
            }
        })
        .collect();
    let ix = bordrless_token::client::transfer_from_protocol(
        owner_key,
        a.royalty_holding.key(),
        a.destination.key(),
        cut_mint,
        extras,
        amount,
        crate::ID,
        seeds_vec,
    );
    let mut infos = vec![
        a.royalty_owner.to_account_info(),
        a.royalty_holding.to_account_info(),
        a.destination.to_account_info(),
        a.cut_mint.to_account_info(),
        tp.clone(),
        ea.clone(),
    ];
    infos.extend(ctx.remaining_accounts.iter().cloned());
    let signer: &[&[u8]] = &[seeds::ROYALTY, item_key.as_ref(), &bump];
    anchor_lang::solana_program::program::invoke_signed(&ix, &infos, &[signer])?;
    emit_cpi!(RoyaltyClaimed {
        item: item_key,
        cut_mint,
        claimant,
        amount,
        ts: now()?
    });
    Ok(())
}

impl<'info> EquipCtx<'info> {
    /// Moves the slot from its current item to `new_item` (02 section 6.4 steps 2 to 4) and
    /// keeps both items' `equipped_count`. Returns the old and new items.
    #[allow(clippy::too_many_arguments)]
    fn equip_to(
        &mut self,
        params: &ArmoryParams,
        slot: u8,
        new_item: Option<Pubkey>,
        config: &EquipConfig,
        launch: bool,
        revert: bool,
    ) -> Result<(Option<Pubkey>, Option<Pubkey>)> {
        let mint_key = self.token_mint.key();
        let (auth, auth_bump) = pda::slot_authority(&mint_key);
        require_keys_eq!(self.slot_authority.key(), auth, ArmoryError::WrongAccount);
        let mint = read_mint(&self.token_mint.to_account_info())?;
        require!(slot < mint.slot_count, ArmoryError::SlotIndexOutOfRange);
        let current = mint.slots[usize::from(slot)].item;
        let old = (current != Pubkey::default()).then_some(current);
        require!(old != new_item, ArmoryError::NoChange);
        if let Some(o) = old {
            let oi = self.old_item.as_ref().ok_or(ArmoryError::WrongAccount)?;
            require_keys_eq!(oi.key(), o, ArmoryError::WrongAccount);
        }
        let mut new_equip = None;
        if let Some(k) = new_item {
            let item = self.new_item.as_ref().ok_or(ArmoryError::WrongAccount)?;
            require_keys_eq!(item.key(), k, ArmoryError::WrongAccount);
            let template = self.new_template.as_ref().ok_or(ArmoryError::WrongAccount)?;
            require_keys_eq!(
                template.key(),
                pda::template(item.template_id).0,
                ArmoryError::WrongAccount
            );
            let program = self.template_program.as_ref().map(|a| a.to_account_info());
            let programdata = self.template_programdata.as_ref().map(|a| a.to_account_info());
            check_fits(
                &mint,
                slot,
                &k,
                &item.manifest,
                template,
                program.as_ref(),
                programdata.as_ref(),
                params,
                launch,
                revert,
            )?;
            new_equip = Some((item.template_id, item.manifest, template.max_targets));
        }
        let old_vault = self.old_equip_vault.as_ref().map(|a| a.to_account_info());
        let new_vault = self.new_equip_vault.as_ref().map(|a| a.to_account_info());
        let registry = self.registry.as_ref().map(|a| a.to_account_info());
        let royalty_owner = self.royalty_owner.as_ref().map(|a| a.to_account_info());
        let rht = self.royalty_holding_token.as_ref().map(|a| a.to_account_info());
        let qm = self.quote_mint.as_ref().map(|a| a.to_account_info());
        let rhq = self.royalty_holding_quote.as_ref().map(|a| a.to_account_info());
        let (tp, ea, sp) = self.token.infos();
        let payer = self.payer.to_account_info();
        let mint_info = self.token_mint.to_account_info();
        let sa = self.slot_authority.to_account_info();
        let es = self.equip_state.to_account_info();
        let signer = self.armory_signer.to_account_info();
        let items = self.items_program.to_account_info();
        let new_item_info = self.new_item.as_ref().map(|a| a.to_account_info());
        let composite_info = self.new_composite.as_ref().map(|a| a.to_account_info());
        let e = EquipInfos {
            payer: &payer,
            mint: &mint_info,
            slot_authority: &sa,
            slot_authority_bump: auth_bump,
            equip_state: &es,
            registry: registry.as_ref(),
            old_equip_vault: old_vault.as_ref(),
            new_equip_vault: new_vault.as_ref(),
            royalty_owner: royalty_owner.as_ref(),
            royalty_holding_token: rht.as_ref(),
            quote_mint: qm.as_ref(),
            royalty_holding_quote: rhq.as_ref(),
            armory_signer: &signer,
            items_program: &items,
            token: TokenInfos {
                token_program: &tp,
                event_authority: &ea,
                system_program: &sp,
            },
        };
        let new = match (new_equip, new_item_info.as_ref()) {
            (Some((template_id, manifest, max_targets)), Some(info)) => Some(NewEquip {
                item: info,
                template_id,
                manifest,
                max_targets,
                config: config.clone(),
                composite: composite_info.as_ref(),
            }),
            _ => None,
        };
        apply_equip(&e, slot, old.is_some(), new)?;
        if old.is_some() {
            if let Some(oi) = self.old_item.as_mut() {
                oi.equipped_count = oi.equipped_count.saturating_sub(1);
            }
        }
        if new_item.is_some() {
            if let Some(ni) = self.new_item.as_mut() {
                ni.equipped_count = ni.equipped_count.saturating_add(1);
            }
        }
        Ok((old, new_item))
    }
}

fn process_equip_launch(ctx: Context<EquipLaunch>, entry: LaunchEquip) -> Result<()> {
    let mint_key = ctx.accounts.equip.token_mint.key();
    require_keys_eq!(
        ctx.accounts.launch_caller.key(),
        pda::armory_caller(&mint_key).0,
        ArmoryError::NotLaunchCaller
    );
    let mint = read_mint(&ctx.accounts.equip.token_mint.to_account_info())?;
    require!(entry.slot < mint.slot_count, ArmoryError::SlotIndexOutOfRange);
    let s = mint.slots[usize::from(entry.slot)];
    require!(
        mint.supply == 0 && s.item == Pubkey::default(),
        ArmoryError::NotFreshMint
    );
    require!(
        s.kind != hookwars_common::kind::LOCKED,
        ArmoryError::SlotLocked
    );
    let params = ctx.accounts.config.params;
    let voting = s.equip_rule == bordrless_hook::equip_rule::VOTE
        || s.equip_rule == bordrless_hook::equip_rule::PERFORMANCE;
    if voting {
        require!(
            entry.notice_secs >= params.min_notice_secs && entry.notice_secs <= params.max_notice_secs,
            ArmoryError::InvalidNotice
        );
    }
    let rule = if s.equip_rule == bordrless_hook::equip_rule::PERFORMANCE {
        let r = entry.rule.ok_or(ArmoryError::InvalidRule)?;
        require!(
            r.metric <= 2
                && r.op <= 1
                && r.window_secs >= params.min_twap_secs
                && r.base_window_secs > r.window_secs
                && entry.item.is_some(),
            ArmoryError::InvalidRule
        );
        Some(r)
    } else {
        require!(entry.rule.is_none(), ArmoryError::InvalidRule);
        None
    };
    if let Some(item) = entry.item {
        ctx.accounts
            .equip
            .equip_to(&params, entry.slot, Some(item), &entry.config, true, false)?;
    }
    let st = &mut ctx.accounts.slot_state;
    st.version = VERSION;
    st.bump = ctx.bumps.slot_state;
    st.mint = mint_key;
    st.slot = entry.slot;
    st.equip_rule = s.equip_rule;
    st.notice_secs = if voting { entry.notice_secs } else { 0 };
    st.launch_item = entry.item;
    st.launch_config = entry.config.clone();
    st.rule = rule;
    st.open_proposal = None;
    st.next_nonce = 0;
    st.condition_since = None;
    st.last_check = 0;
    st.reserved = [0; 32];
    if entry.item.is_some() {
        emit_cpi!(EquipApplied {
            mint: mint_key,
            slot: entry.slot,
            old_item: None,
            new_item: entry.item,
            by: equip_by::LAUNCH,
            ts: now()?
        });
    }
    Ok(())
}

/// Checks a proposed item against the slot (02 section 6.1) with the accounts a proposal path
/// takes.
#[allow(clippy::too_many_arguments)]
fn check_proposed(
    params: &ArmoryParams,
    mint: &Mint,
    slot: u8,
    item: Option<Pubkey>,
    config: &EquipConfig,
    item_acc: Option<&Account<Item>>,
    template: Option<&Account<Template>>,
    program: Option<&UncheckedAccount>,
    programdata: Option<&UncheckedAccount>,
) -> Result<()> {
    require!(slot < mint.slot_count, ArmoryError::SlotIndexOutOfRange);
    let s = mint.slots[usize::from(slot)];
    require!(
        s.kind != hookwars_common::kind::LOCKED
            && s.equip_rule != bordrless_hook::equip_rule::LOCKED,
        ArmoryError::SlotLocked
    );
    let Some(k) = item else {
        return Ok(());
    };
    let i = item_acc.ok_or(ArmoryError::WrongAccount)?;
    require_keys_eq!(i.key(), k, ArmoryError::WrongAccount);
    let t = template.ok_or(ArmoryError::WrongAccount)?;
    require_keys_eq!(t.key(), pda::template(i.template_id).0, ArmoryError::WrongAccount);
    require!(
        config.targets.len() <= usize::from(t.max_targets),
        ArmoryError::OverBounds
    );
    let p = program.map(|a| a.to_account_info());
    let pd = programdata.map(|a| a.to_account_info());
    check_fits(
        mint,
        slot,
        &k,
        &i.manifest,
        t,
        p.as_ref(),
        pd.as_ref(),
        params,
        false,
        false,
    )
}

fn process_propose(
    ctx: Context<Propose>,
    slot: u8,
    item: Option<Pubkey>,
    config: EquipConfig,
) -> Result<()> {
    let a = &ctx.accounts;
    let mint = read_mint(&a.token_mint.to_account_info())?;
    require!(a.slot_state.open_proposal.is_none(), ArmoryError::ProposalOpen);
    check_proposed(
        &a.config.params,
        &mint,
        slot,
        item,
        &config,
        a.item.as_deref(),
        a.template.as_deref(),
        a.template_program.as_ref(),
        a.template_programdata.as_ref(),
    )?;
    if mint.slots[usize::from(slot)].item != Pubkey::default() || item.is_some() {
        require!(
            mint.slots[usize::from(slot)].item != item.unwrap_or_default(),
            ArmoryError::NoChange
        );
    }
    let ts = now()?;
    let vote_end = ts + i64::from(a.config.params.vote_period_secs);
    let executable_at = vote_end + i64::from(a.slot_state.notice_secs);
    let nonce = a.slot_state.next_nonce;
    let mint_key = a.token_mint.key();
    let proposer = a.proposer.key();
    let p = &mut ctx.accounts.proposal;
    p.version = VERSION;
    p.bump = ctx.bumps.proposal;
    p.mint = mint_key;
    p.slot = slot;
    p.nonce = nonce;
    p.proposer = proposer;
    p.item = item;
    p.config = config;
    p.created_at = ts;
    p.vote_end = vote_end;
    p.executable_at = executable_at;
    p.votes_for = 0;
    p.votes_against = 0;
    p.voters_open = 0;
    p.status = proposal_status::OPEN;
    p.reserved = [0; 32];
    let st = &mut ctx.accounts.slot_state;
    st.open_proposal = Some(nonce);
    st.next_nonce = nonce + 1;
    emit_cpi!(ProposalCreated {
        mint: mint_key,
        slot,
        nonce,
        proposer,
        item,
        vote_end,
        executable_at
    });
    Ok(())
}

fn process_vote(ctx: Context<Vote>, support: bool, amount: u64) -> Result<()> {
    let ts = now()?;
    let a = &ctx.accounts;
    let p = &a.proposal;
    require!(
        p.status == proposal_status::OPEN && ts < p.vote_end,
        ArmoryError::VotingClosed
    );
    require_keys_eq!(
        p.key(),
        pda::proposal(&p.mint, p.slot, p.nonce).0,
        ArmoryError::WrongAccount
    );
    let voter = a.voter.key();
    let holding = bordrless_token::client::read_holding(&a.holding.to_account_info())?;
    require!(
        holding.owner == voter && holding.mint == p.mint,
        ArmoryError::WrongAccount
    );
    require!(amount > 0 && amount <= holding.amount, ArmoryError::InvalidAmount);
    // One lock backs votes on several proposals: keep the larger amount and the later end.
    let (lock_amount, until) = if holding.vote_lock_until > ts {
        (
            holding.vote_locked.max(amount),
            holding.vote_lock_until.max(p.vote_end),
        )
    } else {
        (amount, p.vote_end)
    };
    let mint_key = a.token_mint.key();
    let ix = bordrless_token::client::set_vote_lock(
        a.slot_authority.key(),
        mint_key,
        a.holding.key(),
        lock_amount,
        until,
    );
    let bump = [ctx.bumps.slot_authority];
    let signer: &[&[u8]] = &[seeds::SLOTS, mint_key.as_ref(), &bump];
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            a.slot_authority.to_account_info(),
            a.token_mint.to_account_info(),
            a.holding.to_account_info(),
            a.token.token_event_authority.to_account_info(),
            a.token.token_program.to_account_info(),
        ],
        &[signer],
    )?;
    let proposal_key = ctx.accounts.proposal.key();
    let p = &mut ctx.accounts.proposal;
    if support {
        p.votes_for = p.votes_for.checked_add(amount).ok_or(ArmoryError::InvalidAmount)?;
    } else {
        p.votes_against = p
            .votes_against
            .checked_add(amount)
            .ok_or(ArmoryError::InvalidAmount)?;
    }
    p.voters_open += 1;
    let v = &mut ctx.accounts.vote_lock;
    v.proposal = proposal_key;
    v.voter = voter;
    v.amount = amount;
    v.support = support;
    v.bump = ctx.bumps.vote_lock;
    emit_cpi!(VoteLocked {
        proposal: proposal_key,
        voter,
        support,
        amount,
        until
    });
    Ok(())
}

fn process_finalize(ctx: Context<Finalize>) -> Result<()> {
    let ts = now()?;
    let a = &ctx.accounts;
    let p = &a.proposal;
    require!(p.status == proposal_status::OPEN, ArmoryError::ProposalNotFinal);
    require!(ts >= p.vote_end, ArmoryError::VotingNotEnded);
    let mint = read_mint(&a.token_mint.to_account_info())?;
    let mut eligible = mint.supply;
    if let Some(launch) = a.launch.as_ref() {
        let (launch_key, _) = pda::launch(&p.mint);
        require_keys_eq!(launch.key(), launch_key, ArmoryError::WrongAccount);
        require_keys_eq!(*launch.owner, ids::LAUNCH_ID, ArmoryError::WrongAccount);
        let pool = hookwars_common::launch_pool(&launch.try_borrow_data()?, &p.mint)
            .ok_or(ArmoryError::WrongAccount)?;
        for (acc, owner) in [
            (a.pool_base_vault.as_ref(), pool),
            (a.launch_holding.as_ref(), launch_key),
        ] {
            let acc = acc.ok_or(ArmoryError::WrongAccount)?;
            require_keys_eq!(
                acc.key(),
                pda::holding(&p.mint, &owner),
                ArmoryError::WrongAccount
            );
            if acc.data_len() > 0 {
                let h = bordrless_token::client::read_holding(&acc.to_account_info())?;
                eligible = eligible.saturating_sub(h.amount);
            }
        }
    }
    let total = u128::from(p.votes_for) + u128::from(p.votes_against);
    let passed = p.votes_for > p.votes_against;
    let quorum_bps = a.config.params.vote_quorum_bps;
    let p_key = ctx.accounts.proposal.key();
    let (status, vf, va) = {
        let ok = passed && total * 10_000 >= u128::from(eligible) * u128::from(quorum_bps);
        let p = &mut ctx.accounts.proposal;
        p.status = if ok {
            proposal_status::PASSED
        } else {
            proposal_status::FAILED
        };
        (p.status, p.votes_for, p.votes_against)
    };
    if status == proposal_status::FAILED {
        ctx.accounts.slot_state.open_proposal = None;
    }
    emit_cpi!(ProposalResolved {
        proposal: p_key,
        status,
        votes_for: vf,
        votes_against: va,
        eligible,
        ts
    });
    Ok(())
}

fn process_execute(ctx: Context<Execute>) -> Result<()> {
    let ts = now()?;
    let p = &ctx.accounts.proposal;
    require!(
        p.status == proposal_status::PASSED && ts >= p.executable_at,
        ArmoryError::NotExecutable
    );
    require_keys_eq!(
        p.key(),
        pda::proposal(&p.mint, p.slot, p.nonce).0,
        ArmoryError::WrongAccount
    );
    require_keys_eq!(
        ctx.accounts.equip.token_mint.key(),
        p.mint,
        ArmoryError::WrongAccount
    );
    let (slot, item, config) = (p.slot, p.item, p.config.clone());
    let params = ctx.accounts.config.params;
    let (old, new) = ctx
        .accounts
        .equip
        .equip_to(&params, slot, item, &config, false, false)?;
    ctx.accounts.proposal.status = proposal_status::EXECUTED;
    ctx.accounts.slot_state.open_proposal = None;
    emit_cpi!(EquipApplied {
        mint: ctx.accounts.proposal.mint,
        slot,
        old_item: old,
        new_item: new,
        by: equip_by::VOTE,
        ts
    });
    Ok(())
}

fn process_fail_stale(ctx: Context<FailStale>) -> Result<()> {
    let a = &ctx.accounts;
    let p = &a.proposal;
    require!(p.status == proposal_status::PASSED, ArmoryError::NotExecutable);
    let mint = read_mint(&a.token_mint.to_account_info())?;
    let fits = check_proposed(
        &a.config.params,
        &mint,
        p.slot,
        p.item,
        &p.config,
        a.item.as_deref(),
        a.template.as_deref(),
        a.template_program.as_ref(),
        a.template_programdata.as_ref(),
    );
    require!(fits.is_err(), ArmoryError::NotStale);
    let p_key = p.key();
    let (vf, va) = (p.votes_for, p.votes_against);
    ctx.accounts.proposal.status = proposal_status::FAILED;
    ctx.accounts.slot_state.open_proposal = None;
    emit_cpi!(ProposalResolved {
        proposal: p_key,
        status: proposal_status::FAILED,
        votes_for: vf,
        votes_against: va,
        eligible: 0,
        ts: now()?
    });
    Ok(())
}

fn process_check_performance(ctx: Context<CheckPerformance>) -> Result<()> {
    let ts = now()?;
    let st = &ctx.accounts.slot_state;
    let mint_key = ctx.accounts.equip.token_mint.key();
    require_keys_eq!(
        st.key(),
        pda::slot_state(&mint_key, st.slot).0,
        ArmoryError::WrongAccount
    );
    let rule = st.rule.ok_or(ArmoryError::InvalidRule)?;
    let (launch_key, _) = pda::launch(&mint_key);
    let launch = &ctx.accounts.launch;
    require_keys_eq!(launch.key(), launch_key, ArmoryError::WrongAccount);
    require_keys_eq!(*launch.owner, ids::LAUNCH_ID, ArmoryError::WrongAccount);
    let pool_key = hookwars_common::launch_pool(&launch.try_borrow_data()?, &mint_key)
        .ok_or(ArmoryError::WrongAccount)?;
    require_keys_eq!(ctx.accounts.pool.key(), pool_key, ArmoryError::WrongAccount);
    // Hookwars M3b: the ring lives in the pool account (03 M3a notes).
    let pool_info = ctx.accounts.pool.to_account_info();
    let read = |w: u32| {
        bordrless_swap::obs::window_read(&pool_info, ts, i64::from(w), 1)
            .ok()
            .map(|r| hookwars_common::WindowRead {
                twap_q64: r.twap_q64,
                quote_volume: r.quote_volume,
                swaps: r.swaps,
                seconds: r.span_secs,
            })
    };
    let holds = rule.holds(read(rule.window_secs), read(rule.base_window_secs));
    let slot = st.slot;
    let launch_item = st.launch_item;
    let mint = read_mint(&ctx.accounts.equip.token_mint.to_account_info())?;
    let current = mint.slots[usize::from(slot)].item;
    let st = &mut ctx.accounts.slot_state;
    st.last_check = ts;
    let since = if holds {
        Some(st.condition_since.unwrap_or(ts))
    } else {
        None
    };
    if since != st.condition_since {
        st.condition_since = since;
        emit_cpi!(PerformanceCondition {
            mint: mint_key,
            slot,
            true_since: since
        });
    }
    let due = since.is_some_and(|s| ts - s >= i64::from(rule.hold_secs));
    if !due || current == launch_item.unwrap_or_default() {
        return Ok(());
    }
    // Revert to the launch item; an open proposal is cancelled.
    if let Some(nonce) = ctx.accounts.slot_state.open_proposal {
        let p = ctx
            .accounts
            .open_proposal
            .as_mut()
            .ok_or(ArmoryError::WrongAccount)?;
        require_keys_eq!(
            p.key(),
            pda::proposal(&mint_key, slot, nonce).0,
            ArmoryError::WrongAccount
        );
        p.status = proposal_status::CANCELLED;
        ctx.accounts.slot_state.open_proposal = None;
    }
    let params = ctx.accounts.config.params;
    let config = ctx.accounts.slot_state.launch_config.clone();
    let (old, new) = ctx
        .accounts
        .equip
        .equip_to(&params, slot, launch_item, &config, false, true)?;
    ctx.accounts.slot_state.condition_since = None;
    emit_cpi!(PerformanceReverted {
        mint: mint_key,
        slot,
        from_item: old,
        to_item: new,
        ts
    });
    emit_cpi!(EquipApplied {
        mint: mint_key,
        slot,
        old_item: old,
        new_item: new,
        by: equip_by::PERFORMANCE,
        ts
    });
    Ok(())
}

fn process_forge(ctx: Context<Forge>) -> Result<()> {
    let a = &ctx.accounts;
    let forger = a.forger.key();
    require_keys_neq!(a.item_a.key(), a.item_b.key(), ArmoryError::NotItemOwner);
    for (holding, item) in [(&a.holding_a, &a.item_a), (&a.holding_b, &a.item_b)] {
        let h = bordrless_token::client::read_holding(&holding.to_account_info())
            .map_err(|_| ArmoryError::NotItemOwner)?;
        require!(
            h.mint == item.item_mint && h.owner == forger && h.amount == 1 && !h.frozen,
            ArmoryError::NotItemOwner
        );
    }
    require!(
        a.item_a.template_id == a.item_b.template_id,
        ArmoryError::TemplateMismatch
    );
    let t = &a.template;
    require!(
        t.status == template_status::ACTIVE && t.forge_enabled,
        ArmoryError::TemplateClosed
    );
    require!(
        a.item_a.equipped_count == 0 && a.item_b.equipped_count == 0,
        ArmoryError::ItemEquipped
    );
    let level = a.item_a.level.max(a.item_b.level) + 1;
    require!(level <= t.max_level, ArmoryError::MaxLevel);
    let signer = a.armory_signer.to_account_info();
    let items = a.items_program.to_account_info();
    let mut params = combine_params(
        &signer,
        &items,
        t,
        a.config.params.forge_gain_bps,
        &a.item_a.params,
        &a.item_b.params,
    )?;
    // Clamp again: a defect in combine_params can never exceed the ceiling (02 section 9.1).
    let s = shape(t.id);
    for i in 0..PARAM_FIELDS {
        if i < usize::from(t.field_count) {
            let off = s.is_some_and(|s| s.zero_off[i]) && params[i] == 0;
            if !off {
                params[i] = params[i].clamp(t.field_min[i], t.field_max[i]);
            }
        } else {
            params[i] = 0;
        }
    }
    let manifest = validate_item(&signer, &items, t, &params)?;
    let (tp, ea, sp) = a.token.infos();
    // Burn both item tokens (item mints have no hook).
    for (holding, mint) in [(&a.holding_a, &a.mint_a), (&a.holding_b, &a.mint_b)] {
        let ix = bordrless_token::client::burn(forger, holding.key(), mint.key(), None, vec![], 1);
        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                a.forger.to_account_info(),
                holding.to_account_info(),
                mint.to_account_info(),
                ea.clone(),
                tp.clone(),
            ],
        )?;
    }
    let tinfo = TokenInfos {
        token_program: &tp,
        event_authority: &ea,
        system_program: &sp,
    };
    let n = a.config.items_minted;
    mint_item(
        &tinfo,
        &a.forger.to_account_info(),
        &a.minter.to_account_info(),
        &a.item_mint.to_account_info(),
        ctx.bumps.item_mint,
        n,
        &a.forger.to_account_info(),
        &a.recipient_holding.to_account_info(),
        &t.name,
    )?;
    let royalty_bps = a.item_a.royalty_bps.max(a.item_b.royalty_bps);
    let burned = [a.item_a.key(), a.item_b.key()];
    let template_id = t.id;
    let ts = now()?;
    let item_key = ctx.accounts.item.key();
    let item_mint = ctx.accounts.item_mint.key();
    write_item(
        &mut ctx.accounts.item,
        item_key,
        ctx.bumps.item,
        item_mint,
        template_id,
        params,
        manifest,
        forger,
        royalty_bps,
        level,
        source::FORGED,
        ts,
    );
    ctx.accounts.config.items_minted = n + 1;
    let fc = &mut ctx.accounts.forge_counter;
    fc.wallet = forger;
    fc.bump = ctx.bumps.forge_counter;
    fc.count += 1;
    emit_cpi!(ItemCreated {
        item: item_key,
        item_mint,
        template_id,
        params,
        manifest,
        author: forger,
        royalty_bps,
        level,
        source: source::FORGED,
        ts
    });
    emit_cpi!(Forged {
        burned,
        item: item_key,
        template_id,
        params,
        level,
        forger,
        ts
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    /// Hookwars R21: burns fit only slots that allow them.
    #[test]
    fn a_burning_item_fits_only_a_may_burn_slot() {
        use crate::cpi::burn_fits;
        assert!(burn_fits(false, false));
        assert!(burn_fits(false, true));
        assert!(burn_fits(true, true));
        assert!(!burn_fits(true, false));
    }

    use super::*;

    #[test]
    fn signer_constants_match_their_seeds() {
        assert_eq!(pda::armory_signer(), (ARMORY_SIGNER, ARMORY_SIGNER_BUMP));
        assert_eq!(pda::minter(), (MINTER, MINTER_BUMP));
        assert_eq!(pda::loot_signer().0, LOOT_SIGNER);
        assert_eq!(ids::ARMORY_ID, crate::ID);
        assert_eq!(bordrless_token::constants::ARMORY_ID, crate::ID);
        assert_eq!(bordrless_token::constants::ITEMS_ID, ids::ITEMS_ID);
        assert_eq!(bordrless_token::constants::WAR_ID, ids::WAR_ID);
        assert_eq!(bordrless_token::ID, ids::TOKEN_ID);
        assert_eq!(bordrless_swap::ID, ids::SWAP_ID);
        assert_eq!(bordrless_token::constants::LAUNCH_ID, ids::LAUNCH_ID);
        assert_eq!(bordrless_swap::constants::BRIDGED_SOL_MINT, ids::BRIDGED_SOL_MINT);
    }
}
