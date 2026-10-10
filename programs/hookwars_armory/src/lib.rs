// Changed by Hookwars: new file (M2), the armory; M3b: composites (create_composite, the module
// list passed to init_equip), the Performance reader on the pool's ring, settle_bounty_bps;
// security review 1: H-1 fail_stale, H-2 royalty recipients, M-1 finalize, M-2 proposal threshold, I-3;
// security review 2, L-D: execute and the performance revert refresh a slot launch's pool registry.
// Integration pass 2: badge equip by the agents caller, admin Soulbound item, close_proposal bond
// guard, agent record calls, lease gate, revert_for_lease_end, listed claim refusal, badge counters.
// Integration pass 3: set_template_economy, set_item_protocol_bps (E-7, E-2), Template and Item economy fields,
// mint_crafted (E-5), wear on creation (E-3), social counters (E-6), fuse and presets (08 wave F).
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
            && p.proposal_min_bps <= 10_000
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
        c.item_protocol_bps = 0;
        c.reserved = [0; 60];
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
    pub fn register_template<'info>(
        ctx: Context<'info, RegisterTemplate<'info>>,
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

    /// Integration pass 3 (E-7, 11 sections 1.2, 2.2, 5.4): the admin sets a template's economy
    /// fields. Admin-direct like `register_template` and `retire_template` (review 1 L-1 puts all
    /// three behind the timelock; see 13-integration-3).
    pub fn set_template_economy(
        ctx: Context<RetireTemplate>,
        template_id: u16,
        author_bps: u16,
        default_access: u8,
        allowed_access: u8,
        charges_on_create: u32,
    ) -> Result<()> {
        let t = &mut ctx.accounts.template;
        require!(t.id == template_id, ArmoryError::WrongAccount);
        require!(author_bps <= 10_000, ArmoryError::RoyaltyTooHigh);
        require!(
            allowed_access == 0 || allowed_access & (1u8 << default_access.min(7)) != 0,
            ArmoryError::InvalidSchema
        );
        t.author_bps = author_bps;
        t.default_access = default_access;
        t.allowed_access = allowed_access;
        t.charges_on_create = charges_on_create;
        emit_cpi!(TemplateEconomySet {
            template_id,
            author_bps,
            default_access,
            allowed_access,
            charges_on_create,
        });
        Ok(())
    }

    /// Integration pass 3 (E-2, R37): the admin sets `ITEM_PROTOCOL_BPS`, the protocol's share of
    /// each token-side cut `settle_equip` settles. Admin-direct (see L-1 in 13-integration-3).
    pub fn set_item_protocol_bps(ctx: Context<AdminOnly>, item_protocol_bps: u16) -> Result<()> {
        require!(
            item_protocol_bps <= ctx.accounts.config.params.max_royalty_bps,
            ArmoryError::RoyaltyTooHigh
        );
        ctx.accounts.config.item_protocol_bps = item_protocol_bps;
        emit_cpi!(ItemProtocolBpsSet { item_protocol_bps });
        Ok(())
    }

    /// Anyone authors an item from an open template (02 section 4.2).
    pub fn create_item<'info>(
        ctx: Context<'info, CreateItem<'info>>,
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
        process_create_composite(ctx, modules, royalty_bps, Mode::Plain)
    }

    /// Integration pass 3 (08 wave F, section 2.9): the holder of every component fuses them into
    /// one composite: each component is burned and becomes a module (its template and params, the
    /// targets given here), in order. Remaining accounts: the components' templates in order, then
    /// `(item, item mint (mut), holder's holding (mut))` per component, then the optional suffixes
    /// `create_composite` takes. One-way: there is no unfuse.
    pub fn fuse<'info>(
        ctx: Context<'info, CreateComposite<'info>>,
        targets: Vec<(u8, u8)>,
        royalty_bps: u16,
    ) -> Result<()> {
        process_create_composite(ctx, Vec::new(), royalty_bps, Mode::Fuse(targets))
    }

    /// Integration pass 3 (08 section 4.8): the admin registers a composite preset (the module
    /// templates in order). Admin-direct like `register_template` (see L-1 in 13-integration-3).
    pub fn register_preset(ctx: Context<RegisterPreset>, id: u16, template_ids: Vec<u16>, name: String) -> Result<()> {
        require!(
            template_ids.len() >= 2 && template_ids.len() <= hookwars_common::MAX_MODULES && name.len() <= 32,
            ArmoryError::InvalidSchema
        );
        require!(
            template_ids.iter().all(|t| *t != hookwars_common::template_id::COMPOSITE),
            ArmoryError::InvalidSchema
        );
        let p = &mut ctx.accounts.preset;
        p.version = VERSION;
        p.bump = ctx.bumps.preset;
        p.id = id;
        p.name = name;
        p.template_ids = template_ids.clone();
        p.registered_at = now()?;
        emit_cpi!(PresetRegistered { id, template_ids });
        Ok(())
    }

    /// Integration pass 3 (08 section 2.9): `create_composite` whose module templates must be the
    /// preset's, in its order. Remaining accounts: the preset, then what `create_composite` takes.
    pub fn mint_composite<'info>(
        ctx: Context<'info, CreateComposite<'info>>,
        preset_id: u16,
        modules: Vec<hookwars_common::composite::Module>,
        royalty_bps: u16,
    ) -> Result<()> {
        process_create_composite(ctx, modules, royalty_bps, Mode::Preset(preset_id))
    }

    /// The war program mints a loot item (02 section 4.3).
    pub fn mint_loot<'info>(
        ctx: Context<'info, MintLoot<'info>>,
        template_id: u16,
        params: [u32; PARAM_FIELDS],
    ) -> Result<()> {
        process_mint_loot(ctx, template_id, params)
    }

    /// Integration pass 3 (E-5, 11 section 5.3): craft's output. Only `["craft-signer"]` under
    /// craft may call it; the crafter receives the item (`source = CRAFTED`).
    pub fn mint_crafted<'info>(
        ctx: Context<'info, MintCrafted<'info>>,
        crafter: Pubkey,
        template_id: u16,
        param_min: Vec<u32>,
        param_max: Vec<u32>,
        recipe_id: u16,
    ) -> Result<()> {
        process_mint_crafted(ctx, crafter, template_id, param_min, param_max, recipe_id)
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
    pub fn execute<'info>(ctx: Context<'info, Execute<'info>>) -> Result<()> {
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
        // Integration pass 2 (09 section 21 item 6): a proposal a diplomat bonded stays open until
        // the bond leaves `Posted`, or the bond's lamports would stay locked.
        cpi::require_no_posted_bond(
            &ctx.accounts.proposal.key(),
            &ctx.accounts.bond_mark.to_account_info(),
            ctx.accounts.bond.as_ref().map(|b| b.to_account_info()),
        )?;
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
    pub fn check_performance<'info>(ctx: Context<'info, CheckPerformance<'info>>) -> Result<()> {
        process_check_performance(ctx)
    }

    /// Integration pass 2 (10 section 17 I-3): the market ends a lease. When `slot` still holds the
    /// leased `item`, it is settled out and reverted to its launch item (or emptied), as a
    /// performance revert does. Signed by `["market-caller"]` under the market.
    pub fn revert_for_lease_end<'info>(
        ctx: Context<'info, RevertForLeaseEnd<'info>>,
        slot: u8,
        item: Pubkey,
    ) -> Result<()> {
        process_revert_for_lease_end(ctx, slot, item)
    }

    /// Forges two items of one template into one (02 section 9).
    pub fn forge(ctx: Context<Forge>) -> Result<()> {
        process_forge(ctx)
    }

    /// Integration pass 2 (10 section 17 I-5): creates `wallet`'s badge counters
    /// (`["authored", wallet]`, `["claimed", wallet]`); permissionless, idempotent.
    pub fn init_counters(ctx: Context<InitCounters>, wallet: Pubkey) -> Result<()> {
        process_init_counters(ctx, wallet)
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

/// Integration pass 3: accounts of `register_preset`.
#[event_cpi]
#[derive(Accounts)]
#[instruction(id: u16)]
pub struct RegisterPreset<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump, has_one = admin @ ArmoryError::NotAdmin)]
    pub config: Box<Account<'info, ArmoryConfig>>,
    #[account(init, payer = admin, space = 8 + Preset::INIT_SPACE, seeds = [b"preset", &id.to_le_bytes()], bump)]
    pub preset: Box<Account<'info, Preset>>,
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

/// Integration pass 3 (E-5): `MintLoot` with craft's signer in place of the war program's.
#[event_cpi]
#[derive(Accounts)]
#[instruction(crafter: Pubkey, template_id: u16)]
pub struct MintCrafted<'info> {
    /// Craft's `["craft-signer"]` (checked in the handler).
    pub craft_signer: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: who receives the item (the crafter).
    #[account(address = crafter @ ArmoryError::WrongAccount)]
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
    /// CHECK: the proposer's holding of the token (security review 1, M-2: the proposal
    /// threshold, locked in place for the vote period; address-checked in the handler).
    #[account(mut)]
    pub proposer_holding: UncheckedAccount<'info>,
    /// CHECK: `["slots", token_mint]`, which signs the proposer's vote lock.
    #[account(seeds = [seeds::SLOTS, token_mint.key().as_ref()], bump)]
    pub slot_authority: UncheckedAccount<'info>,
    pub token: TokenAccounts<'info>,
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
    /// CHECK: the token's `Launch` address (`["launch", mint]` under the launchpad), always
    /// passed (security review 1, M-1); when it holds a launch, the two holdings below are required.
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
    /// CHECK: `["bond-mark", proposal]` under the agents program (checked in the handler).
    pub bond_mark: UncheckedAccount<'info>,
    /// CHECK: the bond the mark names, when a mark exists (checked in the handler).
    pub bond: Option<UncheckedAccount<'info>>,
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
#[instruction(slot: u8)]
pub struct RevertForLeaseEnd<'info> {
    /// `["market-caller"]` under the market (checked in the handler).
    pub market_caller: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, ArmoryConfig>>,
    #[account(mut, seeds = [seeds::SLOT_STATE, equip.token_mint.key().as_ref(), &[slot]], bump = slot_state.bump)]
    pub slot_state: Box<Account<'info, SlotState>>,
    pub equip: EquipCtx<'info>,
}

/// Integration pass 2 (I-5).
#[derive(Accounts)]
#[instruction(wallet: Pubkey)]
pub struct InitCounters<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(init_if_needed, payer = payer, space = 8 + AuthorCounter::INIT_SPACE,
        seeds = [seeds::AUTHORED, wallet.as_ref()], bump)]
    pub author_counter: Box<Account<'info, AuthorCounter>>,
    #[account(init_if_needed, payer = payer, space = 8 + ClaimCounter::INIT_SPACE,
        seeds = [seeds::CLAIMED, wallet.as_ref()], bump)]
    pub claim_counter: Box<Account<'info, ClaimCounter>>,
    pub system_program: Program<'info, System>,
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

fn process_register_template<'info>(
    ctx: Context<'info, RegisterTemplate<'info>>,
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
    t.author_bps = 0;
    t.default_access = 0;
    t.allowed_access = 0;
    t.charges_on_create = 0;
    t.reserved = [0; 24];
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
    // Integration pass 3 (E-6): the registrant's TEMPLATES_REGISTERED when the social suffix is given.
    let (_, _, social) = split_eco(ctx.remaining_accounts);
    record_social(social, &ctx.accounts.admin.key(), hookwars_common::economy::counter::TEMPLATES_REGISTERED)?;
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
    item.has_wear = false;
    item.reserved = [0; 31];
}

fn process_create_item<'info>(
    ctx: Context<'info, CreateItem<'info>>,
    template_id: u16,
    params: [u32; PARAM_FIELDS],
    royalty_bps: u16,
) -> Result<()> {
    let a = &ctx.accounts;
    // Integration pass 2 (09 section 21 item 2): the armory admin may create an item of a closed
    // template (the one Soulbound item).
    require!(
        a.template.open_authoring || a.author.key() == a.config.admin,
        ArmoryError::TemplateClosed
    );
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
    // Integration pass 2 (09 section 21 item 3): optional agent attribution, after the effects.
    let (rest, rec) = hookwars_common::agents_record::split(ctx.remaining_accounts, &crate::ID);
    // Integration pass 3 (E-3, E-6): wear and the social counter.
    let (rest, craft, social) = split_eco(rest);
    let charges = ctx.accounts.template.charges_on_create;
    let payer = ctx.accounts.author.to_account_info();
    open_wear(craft, &payer, &mut ctx.accounts.item, &item_key, charges)?;
    bump_author_counter(rest, &author)?;
    record_social(social, &author, hookwars_common::economy::counter::ITEMS_AUTHORED)?;
    hookwars_common::agents_record::record(rec, &crate::ID, &ctx.accounts.author.key(), hookwars_common::agents_record::ITEMS_AUTHORED, 0)?;
    Ok(())
}

/// Integration pass 2 (10 section 17 I-5): the counter last in `rest` when it is `key`'s, owned
/// here; the rest without it.
fn take_counter<'a, 'info>(
    rest: &'a [AccountInfo<'info>],
    key: &Pubkey,
) -> (&'a [AccountInfo<'info>], Option<&'a AccountInfo<'info>>) {
    match rest.last() {
        Some(a) if a.key == key && *a.owner == crate::ID => (&rest[..rest.len() - 1], Some(a)),
        _ => (rest, None),
    }
}

/// Bumps `author`'s `AuthorCounter` when the remaining accounts end with it.
fn bump_author_counter<'info>(rest: &[AccountInfo<'info>], author: &Pubkey) -> Result<()> {
    let (_, c) = take_counter(rest, &pda::author_counter(author).0);
    let Some(c) = c else { return Ok(()) };
    require!(c.is_writable, ArmoryError::WrongAccount);
    let mut v = AuthorCounter::try_deserialize(&mut &c.try_borrow_data()?[..])?;
    v.items = v.items.saturating_add(1);
    v.try_serialize(&mut &mut c.try_borrow_mut_data()?[..])?;
    Ok(())
}

/// Integration pass 3 (E-3, E-6): the optional economy suffixes, before the agents suffix:
/// `[..., craft init-wear (6), social (5)]`. Returns the rest, the craft and the social suffix.
#[allow(clippy::type_complexity)]
fn split_eco<'a, 'info>(
    rem: &'a [AccountInfo<'info>],
) -> (&'a [AccountInfo<'info>], Option<&'a [AccountInfo<'info>]>, Option<&'a [AccountInfo<'info>]>) {
    use hookwars_common::{economy as eco, eco_cpi};
    let (rem, social) = eco_cpi::split_tagged(rem, &eco::SOCIAL_ID, eco_cpi::SOCIAL_SUFFIX);
    let (rem, craft) = eco_cpi::split_tagged(rem, &eco::CRAFT_ID, eco_cpi::INIT_WEAR_SUFFIX);
    (rem, craft, social)
}

/// Integration pass 3 (E-3): opens the item's craft `Wear` when its template wears; the init-wear
/// suffix is then required, so no item of a wearing template is made without one.
fn open_wear<'info>(
    craft: Option<&[AccountInfo<'info>]>,
    payer: &AccountInfo<'info>,
    item: &mut Item,
    item_key: &Pubkey,
    charges: u32,
) -> Result<()> {
    if charges == 0 {
        return Ok(());
    }
    let s = craft.ok_or(ArmoryError::WearAccountsMissing)?;
    hookwars_common::eco_cpi::init_wear(s, &crate::ID, payer, item_key, charges)?;
    item.has_wear = true;
    Ok(())
}

/// Integration pass 3 (E-6): a social counter when the social suffix was given.
fn record_social<'info>(social: Option<&[AccountInfo<'info>]>, wallet: &Pubkey, counter: u8) -> Result<()> {
    match social {
        Some(s) => hookwars_common::eco_cpi::record_wallet(s, &crate::ID, wallet, counter, 1),
        None => Ok(()),
    }
}

/// Integration pass 2 (I-5): creates `wallet`'s `AuthorCounter` and `ClaimCounter` (anyone pays).
fn process_init_counters(ctx: Context<InitCounters>, wallet: Pubkey) -> Result<()> {
    let a = &mut ctx.accounts.author_counter;
    if a.wallet == Pubkey::default() {
        a.wallet = wallet;
        a.bump = ctx.bumps.author_counter;
    }
    let c = &mut ctx.accounts.claim_counter;
    if c.wallet == Pubkey::default() {
        c.wallet = wallet;
        c.bump = ctx.bumps.claim_counter;
    }
    Ok(())
}

/// Hookwars M3b: `create_composite` (08 sections 2.9 and 2.10).
/// Integration pass 3: how `process_create_composite` gets its modules.
enum Mode {
    Plain,
    /// `fuse`: the components' targets.
    Fuse(Vec<(u8, u8)>),
    /// `mint_composite`: the preset's id.
    Preset(u16),
}

fn process_create_composite<'info>(
    ctx: Context<'info, CreateComposite<'info>>,
    modules: Vec<hookwars_common::composite::Module>,
    royalty_bps: u16,
    mode: Mode,
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
    // Integration pass 2: an optional agent attribution suffix comes after the module templates.
    let (rem, rec) = hookwars_common::agents_record::split(ctx.remaining_accounts, &crate::ID);
    // Integration pass 3 (E-3, E-6): optional craft and social suffixes before it.
    let (rem, craft, social) = split_eco(rem);
    // I-5: an optional `AuthorCounter` after the module templates.
    let (rem, counter) = take_counter(rem, &pda::author_counter(&a.author.key()).0);
    // Integration pass 3 (wave F): `fuse` builds the modules from the components and burns them.
    let (rem, modules, provenance) = match &mode {
        Mode::Plain => (rem, modules, Vec::new()),
        Mode::Preset(id) => {
            let (info, rest) = rem.split_first().ok_or(ArmoryError::WrongAccount)?;
            require_keys_eq!(*info.owner, crate::ID, ArmoryError::WrongAccount);
            require_keys_eq!(
                info.key(),
                Pubkey::find_program_address(&[b"preset", &id.to_le_bytes()], &crate::ID).0,
                ArmoryError::WrongAccount
            );
            let p = Preset::try_deserialize(&mut &info.try_borrow_data()?[..])?;
            require!(
                p.template_ids == modules.iter().map(|m| m.template_id).collect::<Vec<_>>(),
                ArmoryError::InvalidSchema
            );
            (rest, modules, Vec::new())
        }
        Mode::Fuse(tg) => {
            let k = tg.len();
            require!(k >= 2 && rem.len() == 4 * k, ArmoryError::WrongAccount);
            let (templates, groups) = rem.split_at(k);
            let mut built = Vec::with_capacity(k);
            let mut provenance = Vec::with_capacity(k);
            for (i, &(start, count)) in tg.iter().enumerate() {
                let (item_info, mint_info, holding) = (&groups[3 * i], &groups[3 * i + 1], &groups[3 * i + 2]);
                require_keys_eq!(*item_info.owner, crate::ID, ArmoryError::WrongAccount);
                let it = Item::try_deserialize(&mut &item_info.try_borrow_data()?[..])?;
                require_keys_eq!(mint_info.key(), it.item_mint, ArmoryError::WrongAccount);
                require_keys_eq!(item_info.key(), pda::item(&it.item_mint).0, ArmoryError::WrongAccount);
                require!(it.template_id != hookwars_common::template_id::COMPOSITE, ArmoryError::InvalidSchema);
                require!(it.equipped_count == 0, ArmoryError::ItemEquipped);
                require_keys_eq!(
                    holding.key(),
                    bordrless_token::client::holding_address(&it.item_mint, &a.author.key()),
                    ArmoryError::NotItemHolder
                );
                require!(
                    *holding.owner == bordrless_token::ID
                        && holding.data_len() > 0
                        && bordrless_token::client::read_holding(holding)?.amount == 1,
                    ArmoryError::NotItemHolder
                );
                let data_bytes = hookwars_common::manifest(it.template_id, &it.params, count)
                    .map_err(|_| error!(ArmoryError::InvalidSchema))?
                    .data_bytes;
                built.push(hookwars_common::composite::Module {
                    template_id: it.template_id,
                    params: it.params,
                    target_start: start,
                    target_count: count,
                    data_bytes,
                    reads_module: hookwars_common::composite::NO_READ,
                });
                let (tp, ea, _) = a.token.infos();
                let ix = bordrless_token::client::burn(a.author.key(), holding.key(), it.item_mint, None, Vec::new(), 1);
                anchor_lang::solana_program::program::invoke(
                    &ix,
                    &[a.author.to_account_info(), holding.clone(), mint_info.clone(), tp, ea],
                )?;
                provenance.push(item_info.key());
            }
            (templates, built, provenance)
        }
    };
    require!(rem.len() == modules.len(), ArmoryError::WrongAccount);
    let mut fields: Vec<(u16, Params, Params)> = Vec::with_capacity(modules.len());
    for (m, info) in modules.iter().zip(rem.iter()) {
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
    c.provenance = provenance;
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
    let charges = ctx.accounts.template.charges_on_create;
    let payer = ctx.accounts.author.to_account_info();
    open_wear(craft, &payer, &mut ctx.accounts.item, &item_key, charges)?;
    if let Some(c) = counter {
        bump_author_counter(core::slice::from_ref(c), &ctx.accounts.author.key())?;
    }
    record_social(social, &author, hookwars_common::economy::counter::ITEMS_AUTHORED)?;
    hookwars_common::agents_record::record(rec, &crate::ID, &ctx.accounts.author.key(), hookwars_common::agents_record::ITEMS_AUTHORED, 0)?;
    Ok(())
}

/// Integration pass 3 (E-5): the crafted item's fields are fixed, not drawn: each is the
/// midpoint of the recipe's range (so a range with `min == max` gives exactly that value). The
/// loot randomness adapter is not wired here (deviation, 13-integration-3).
fn process_mint_crafted<'info>(
    ctx: Context<'info, MintCrafted<'info>>,
    crafter: Pubkey,
    template_id: u16,
    param_min: Vec<u32>,
    param_max: Vec<u32>,
    recipe_id: u16,
) -> Result<()> {
    let (craft_signer, _) = Pubkey::find_program_address(
        &[b"craft-signer"],
        &hookwars_common::economy::CRAFT_ID,
    );
    require_keys_eq!(ctx.accounts.craft_signer.key(), craft_signer, ArmoryError::NotCraftSigner);
    require!(
        param_min.len() == param_max.len() && param_min.len() <= PARAM_FIELDS,
        ArmoryError::InvalidSchema
    );
    let mut params = [0u32; PARAM_FIELDS];
    for (i, (lo, hi)) in param_min.iter().zip(param_max.iter()).enumerate() {
        require!(lo <= hi, ArmoryError::InvalidSchema);
        params[i] = lo + (hi - lo) / 2;
    }
    let a = &ctx.accounts;
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
    write_item(
        &mut ctx.accounts.item,
        item_key,
        ctx.bumps.item,
        item_mint,
        template_id,
        params,
        manifest,
        craft_signer,
        royalty_bps,
        1,
        source::CRAFTED,
        ts,
    );
    let (_, craft, _) = split_eco(ctx.remaining_accounts);
    let charges = ctx.accounts.template.charges_on_create;
    let payer = ctx.accounts.payer.to_account_info();
    open_wear(craft, &payer, &mut ctx.accounts.item, &item_key, charges)?;
    ctx.accounts.config.items_minted = n + 1;
    emit_cpi!(ItemCreated {
        item: item_key,
        item_mint,
        template_id,
        params,
        manifest,
        author: craft_signer,
        royalty_bps,
        level: 1,
        source: source::CRAFTED,
        ts
    });
    emit_cpi!(ItemCrafted {
        item: item_key,
        owner: crafter,
        template_id,
        recipe_id,
        params,
        ts
    });
    Ok(())
}

fn process_mint_loot<'info>(ctx: Context<'info, MintLoot<'info>>, template_id: u16, params: Params) -> Result<()> {
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
    // Integration pass 3 (E-3): wear for a wearing template (the war program passes the suffix).
    let (_, craft, _) = split_eco(ctx.remaining_accounts);
    let charges = ctx.accounts.template.charges_on_create;
    let payer = ctx.accounts.payer.to_account_info();
    open_wear(craft, &payer, &mut ctx.accounts.item, &item_key, charges)?;
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
    // Integration pass 2 (10 section 17 I-4, R31): royalties stay with a listed item and go to its
    // buyer, so nothing may claim from the market's escrow while it is listed.
    require_keys_neq!(claimant, hookwars_common::market::escrow(&a.item.item_mint), ArmoryError::ItemListed);
    let cut_mint = a.cut_mint.key();
    // Security review 1, H-2: the kit counts a protocol transfer's program-owned destination as an
    // excluded vault, so a royalty of a kit token must go to a wallet (a key on the curve), never to
    // a program address that could later move it as an ordinary holder.
    {
        let m = read_mint(&a.cut_mint.to_account_info())?;
        let runs_kit = m.hook_program == Some(ids::KIT_ID)
            || m.slots[..usize::from(m.slot_count)].iter().any(|sl| {
                sl.kind == hookwars_common::kind::LOCKED && sl.program == ids::KIT_ID
            });
        require!(!runs_kit || claimant.is_on_curve(), ArmoryError::RecipientOffCurve);
    }
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
    // Integration pass 2: the agent suffix and the `ClaimCounter` (I-5) are not token extras.
    let (rest, rec) = hookwars_common::agents_record::split(ctx.remaining_accounts, &crate::ID);
    let (rest, counter) = take_counter(rest, &pda::claim_counter(&claimant).0);
    let extras: Vec<_> = rest
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
    infos.extend(rest.iter().cloned());
    let signer: &[&[u8]] = &[seeds::ROYALTY, item_key.as_ref(), &bump];
    anchor_lang::solana_program::program::invoke_signed(&ix, &infos, &[signer])?;
    emit_cpi!(RoyaltyClaimed {
        item: item_key,
        cut_mint,
        claimant,
        amount,
        ts: now()?
    });
    let value = if cut_mint == hookwars_common::ids::BRIDGED_SOL_MINT { amount } else { 0 };
    if let Some(c) = counter {
        require!(c.is_writable, ArmoryError::WrongAccount);
        let mut v = ClaimCounter::try_deserialize(&mut &c.try_borrow_data()?[..])?;
        v.lamports = v.lamports.saturating_add(value);
        v.try_serialize(&mut &mut c.try_borrow_mut_data()?[..])?;
    }
    hookwars_common::agents_record::record(rec, &crate::ID, &claimant, hookwars_common::agents_record::ROYALTY_CLAIM, value)?;
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
    let mint = read_mint(&ctx.accounts.equip.token_mint.to_account_info())?;
    let caller = ctx.accounts.launch_caller.key();
    if caller != pda::armory_caller(&mint_key).0 {
        // Integration pass 2 (09 section 21 item 1, R28): the agents program equips a badge, and
        // only a badge: one Locked Defense slot, the agents signer freezing, a Soulbound item.
        require_keys_eq!(caller, cpi::agents_armory_caller(&mint_key), ArmoryError::NotLaunchCaller);
        require!(cpi::is_badge(&mint) && entry.slot == 0, ArmoryError::NotBadge);
        let item = ctx.accounts.equip.new_item.as_deref().ok_or(ArmoryError::NotBadge)?;
        require!(
            entry.item == Some(item.key()) && item.template_id == hookwars_common::template_id::SOULBOUND,
            ArmoryError::NotBadge
        );
    }
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
    let (_, rec) = hookwars_common::agents_record::split(ctx.remaining_accounts, &crate::ID);
    if let Some(author) = ctx.accounts.equip.new_item.as_ref().map(|i| i.author) {
        if author != ctx.accounts.equip.payer.key() {
            hookwars_common::agents_record::record(rec, &crate::ID, &author, hookwars_common::agents_record::ITEMS_EQUIPPED, 0)?;
        }
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
        // Security review 1, I-3: an emptying proposal names no targets.
        require!(config.targets.is_empty(), ArmoryError::OverBounds);
        return Ok(());
    };
    let i = item_acc.ok_or(ArmoryError::WrongAccount)?;
    require_keys_eq!(i.key(), k, ArmoryError::WrongAccount);
    let t = template.ok_or(ArmoryError::WrongAccount)?;
    require_keys_eq!(t.key(), pda::template(i.template_id).0, ArmoryError::WrongAccount);
    // Integration pass 2 (09 section 21 item 2): Soulbound lives only on agent badges.
    require!(
        i.template_id != hookwars_common::template_id::SOULBOUND || cpi::is_badge(mint),
        ArmoryError::NotBadge
    );
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

/// Locks `amount` of `holding` in place until `until` through the token program's `set_vote_lock`,
/// signed by `["slots", mint]`. One lock backs several votes and proposals: an existing lock keeps
/// the larger amount and the later end.
#[allow(clippy::too_many_arguments)]
fn lock_in_place<'info>(
    slot_authority: &AccountInfo<'info>,
    slot_authority_bump: u8,
    token_mint: &AccountInfo<'info>,
    holding_info: &AccountInfo<'info>,
    holding: &bordrless_token::state::Holding,
    token: &TokenAccounts<'info>,
    amount: u64,
    until: i64,
    now: i64,
) -> Result<(u64, i64)> {
    let (lock_amount, until) = if holding.vote_lock_until > now {
        (
            holding.vote_locked.max(amount),
            holding.vote_lock_until.max(until),
        )
    } else {
        (amount, until)
    };
    let mint_key = token_mint.key();
    let ix = bordrless_token::client::set_vote_lock(
        slot_authority.key(),
        mint_key,
        holding_info.key(),
        lock_amount,
        until,
    );
    let bump = [slot_authority_bump];
    let signer: &[&[u8]] = &[seeds::SLOTS, mint_key.as_ref(), &bump];
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            slot_authority.clone(),
            token_mint.clone(),
            holding_info.clone(),
            token.token_event_authority.to_account_info(),
            token.token_program.to_account_info(),
        ],
        &[signer],
    )?;
    Ok((lock_amount, until))
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
    // Integration pass 2 (10 section 17 I-3): a proposed item's `["lease", item]` under the market
    // comes first in the remaining accounts. A lease that exists must be Active and name this token
    // and slot; an item offered for lease, or leased elsewhere, is refused.
    if let Some(k) = item {
        let lease = ctx.remaining_accounts.first().ok_or(ArmoryError::WrongAccount)?;
        require_keys_eq!(lease.key(), hookwars_common::market::lease(&k), ArmoryError::WrongAccount);
        if let Some(l) = hookwars_common::market::read_lease(lease) {
            require!(
                l.state == hookwars_common::market::LEASE_ACTIVE
                    && l.token_mint == a.token_mint.key()
                    && l.slot == slot,
                ArmoryError::ItemLeasedElsewhere
            );
        }
    }
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
    // Security review 1, M-2: a proposer holds at least the threshold, locked for the vote period,
    // so one wallet with no tokens cannot hold a slot's only open-proposal seat.
    let min_bps = a.config.params.proposal_min_bps;
    if min_bps > 0 {
        let mint_key = a.token_mint.key();
        let proposer = a.proposer.key();
        require_keys_eq!(
            a.proposer_holding.key(),
            pda::holding(&mint_key, &proposer),
            ArmoryError::WrongAccount
        );
        let h = bordrless_token::client::read_holding(&a.proposer_holding.to_account_info())
            .map_err(|_| ArmoryError::BelowProposalThreshold)?;
        require!(
            h.owner == proposer && h.mint == mint_key,
            ArmoryError::WrongAccount
        );
        let need = u64::try_from(
            (u128::from(mint.supply) * u128::from(min_bps)).div_ceil(10_000),
        )
        .map_err(|_| ArmoryError::InvalidAmount)?
        .max(1);
        require!(h.amount >= need, ArmoryError::BelowProposalThreshold);
        let ts = now()?;
        let until = ts + i64::from(a.config.params.vote_period_secs);
        lock_in_place(
            &a.slot_authority.to_account_info(),
            ctx.bumps.slot_authority,
            &a.token_mint.to_account_info(),
            &a.proposer_holding.to_account_info(),
            &h,
            &a.token,
            need,
            until,
            ts,
        )?;
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
    let (_, until) = lock_in_place(
        &a.slot_authority.to_account_info(),
        ctx.bumps.slot_authority,
        &a.token_mint.to_account_info(),
        &a.holding.to_account_info(),
        &holding,
        &a.token,
        amount,
        p.vote_end,
        ts,
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
    // Security review 1, M-1: the launch address is always passed, so nobody can finalize against
    // the whole supply by omitting it; when it holds a launch, both holdings are required.
    let launch = a.launch.as_ref().ok_or(ArmoryError::WrongAccount)?;
    let (launch_key, _) = pda::launch(&p.mint);
    require_keys_eq!(launch.key(), launch_key, ArmoryError::WrongAccount);
    if *launch.owner == ids::LAUNCH_ID {
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

/// Security review 2, L-D: after an equip of a `Pool` or `Relation` slot, refresh the launch pool's
/// registry in the same instruction, so no swap runs against a registry that does not list the
/// slot's new item (the launchpad refuses such swaps). The remaining accounts of the equip are
/// `[launch program, launch, pool registry, then each forwarded pool slot's item registry in slot
/// order]`; the first two are always required for these slots. A token that is not a slot launch
/// (no launch account yet, or an upstream launch) has no pool items to forward: nothing to refresh.
fn refresh_after_equip<'info>(
    equip: &EquipCtx<'info>,
    slot: u8,
    remaining: &[AccountInfo<'info>],
) -> Result<()> {
    let mint = read_mint(&equip.token_mint.to_account_info())?;
    let kind = mint.slots[usize::from(slot)].kind;
    if kind != hookwars_common::kind::POOL && kind != hookwars_common::kind::RELATION {
        return Ok(());
    }
    require!(remaining.len() >= 2, ArmoryError::WrongAccount);
    let mint_key = equip.token_mint.key();
    require_keys_eq!(remaining[0].key(), ids::LAUNCH_ID, ArmoryError::WrongAccount);
    require_keys_eq!(remaining[1].key(), pda::launch(&mint_key).0, ArmoryError::WrongAccount);
    let launch = &remaining[1];
    if *launch.owner != ids::LAUNCH_ID || launch.data_is_empty() {
        return Ok(());
    }
    let slot_launch = bordrless_launch::state::Launch::try_deserialize(&mut &launch.try_borrow_data()?[..])
        .map(|l| l.is_slot_launch())
        .unwrap_or(false);
    if !slot_launch {
        return Ok(());
    }
    require!(remaining.len() >= 3, ArmoryError::WrongAccount);
    let payer = equip.payer.to_account_info();
    let system = equip.token.system_program.to_account_info();
    let mut metas = vec![
        AccountMeta::new(payer.key(), true),
        AccountMeta::new_readonly(launch.key(), false),
        AccountMeta::new_readonly(mint_key, false),
        AccountMeta::new(remaining[2].key(), false),
        AccountMeta::new_readonly(system.key(), false),
    ];
    let mut infos = vec![
        payer,
        launch.clone(),
        equip.token_mint.to_account_info(),
        remaining[2].clone(),
        system,
    ];
    for r in &remaining[3..] {
        metas.push(AccountMeta {
            pubkey: r.key(),
            is_signer: false,
            is_writable: r.is_writable,
        });
        infos.push(r.clone());
    }
    infos.push(remaining[0].clone());
    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: ids::LAUNCH_ID,
        accounts: metas,
        data: <bordrless_launch::instruction::RefreshPoolRegistry as anchor_lang::Discriminator>::DISCRIMINATOR.to_vec(),
    };
    anchor_lang::solana_program::program::invoke(&ix, &infos)?;
    Ok(())
}

fn process_execute<'info>(ctx: Context<'info, Execute<'info>>) -> Result<()> {
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
    let (rem, rec) = hookwars_common::agents_record::split(ctx.remaining_accounts, &crate::ID);
    refresh_after_equip(&ctx.accounts.equip, slot, rem)?;
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
    // The equipped item's author is credited when someone else proposed it.
    if let Some(author) = ctx.accounts.equip.new_item.as_ref().map(|i| i.author) {
        if author != ctx.accounts.proposal.proposer {
            hookwars_common::agents_record::record(rec, &crate::ID, &author, hookwars_common::agents_record::ITEMS_EQUIPPED, 0)?;
        }
    }
    Ok(())
}

/// Security review 1, H-1: the errors that mean a passed proposal no longer fits its slot (the
/// world changed since the vote). Any other error (a missing or wrong account) is the caller's
/// mistake and fails the transaction instead of the proposal.
fn is_stale_error(e: &Error) -> bool {
    let Error::AnchorError(a) = e else {
        return false;
    };
    let offset = anchor_lang::error::ERROR_CODE_OFFSET;
    [
        ArmoryError::TemplateClosed as u32,
        ArmoryError::TemplateChanged as u32,
        ArmoryError::KindMismatch as u32,
        ArmoryError::OverBounds as u32,
        ArmoryError::DataRangeTooSmall as u32,
        ArmoryError::AlreadyEquipped as u32,
        ArmoryError::SlotLocked as u32,
        ArmoryError::SlotIndexOutOfRange as u32,
    ]
    .iter()
    .any(|c| c + offset == a.error_code_number)
}

fn process_fail_stale(ctx: Context<FailStale>) -> Result<()> {
    let a = &ctx.accounts;
    let p = &a.proposal;
    require!(p.status == proposal_status::PASSED, ArmoryError::NotExecutable);
    let mint = read_mint(&a.token_mint.to_account_info())?;
    // Security review 1, H-1: the proposal's own accounts are required, so leaving them out is an
    // error, never staleness.
    if let Some(k) = p.item {
        let i = a.item.as_deref().ok_or(ArmoryError::WrongAccount)?;
        require_keys_eq!(i.key(), k, ArmoryError::WrongAccount);
        let t = a.template.as_deref().ok_or(ArmoryError::WrongAccount)?;
        require_keys_eq!(t.key(), pda::template(i.template_id).0, ArmoryError::WrongAccount);
        if t.deploy_slot.is_some() {
            let program = a.template_program.as_ref().ok_or(ArmoryError::ProgramDataMissing)?;
            require_keys_eq!(program.key(), t.program, ArmoryError::WrongAccount);
            if *program.owner != ids::LOADER_V4_ID {
                let pd = a
                    .template_programdata
                    .as_ref()
                    .ok_or(ArmoryError::ProgramDataMissing)?;
                require_keys_eq!(
                    pd.key(),
                    hookwars_common::programdata_address(&t.program),
                    ArmoryError::ProgramDataMissing
                );
            }
        }
    }
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
    match fits {
        Ok(()) => return err!(ArmoryError::NotStale),
        Err(e) if !is_stale_error(&e) => return Err(e),
        Err(_) => {}
    }
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

fn process_check_performance<'info>(ctx: Context<'info, CheckPerformance<'info>>) -> Result<()> {
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
    refresh_after_equip(&ctx.accounts.equip, slot, ctx.remaining_accounts)?;
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
    let (_, rec) = hookwars_common::agents_record::split(ctx.remaining_accounts, &crate::ID);
    hookwars_common::agents_record::record(rec, &crate::ID, &ctx.accounts.forger.key(), hookwars_common::agents_record::ITEMS_FORGED, 0)?;
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


/// Integration pass 2 (10 section 17 I-3): see `revert_for_lease_end`.
fn process_revert_for_lease_end<'info>(
    ctx: Context<'info, RevertForLeaseEnd<'info>>,
    slot: u8,
    item: Pubkey,
) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.market_caller.key(),
        hookwars_common::market::caller().0,
        ArmoryError::NotMarketCaller
    );
    let mint = read_mint(&ctx.accounts.equip.token_mint.to_account_info())?;
    require!(slot < mint.slot_count, ArmoryError::SlotIndexOutOfRange);
    // A vote may already have replaced the leased item: then there is nothing to revert.
    if mint.slots[usize::from(slot)].item != item {
        return Ok(());
    }
    let launch_item = ctx.accounts.slot_state.launch_item;
    if launch_item == Some(item) {
        return Ok(());
    }
    let ts = now()?;
    let params = ctx.accounts.config.params;
    let config = ctx.accounts.slot_state.launch_config.clone();
    let (old, new) = ctx
        .accounts
        .equip
        .equip_to(&params, slot, launch_item, &config, false, true)?;
    refresh_after_equip(&ctx.accounts.equip, slot, ctx.remaining_accounts)?;
    emit_cpi!(EquipApplied {
        mint: ctx.accounts.equip.token_mint.key(),
        slot,
        old_item: old,
        new_item: new,
        by: equip_by::LEASE_END,
        ts
    });
    Ok(())
}
