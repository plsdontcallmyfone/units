// Changed by Hookwars: new program (hook economy, docs/spec/11-hook-economy.md section 5).
//! `hookwars_craft`: materials, recipes, repairs and item wear.
//!
//! - **Materials** are plain units mints (no hook) whose only mint authority is `["craft-minter"]`
//!   under this program. They enter only through `drop`, which accepts only the `["craft-caller"]`
//!   PDA of a program the config names, and never more than the material's cap for the current
//!   season (R42). No material is redeemable for anything the protocol holds.
//! - **Recipes** burn materials and a SOL fee: `craft` asks the output program (the armory's
//!   `mint_crafted`, integration request) to make an item, signed by `["craft-signer"]`; `repair`
//!   restores an item's charges.
//! - **Wear** lives in `Wear` at `["wear", item]`: the items engine opens it at creation and adds
//!   the runs it applied (both through the caller PDA, integration requests). At `max_charges` the
//!   item is dormant and `ItemWorn` is emitted once; reading dormancy never makes a trade fail
//!   (R38). An item without a `Wear` never wears.
//!
//! Every admin change (params, caps, drop rules, recipes) waits `admin_timelock_secs`. Spot only.

#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::program::{invoke, invoke_signed};
use anchor_lang::system_program;
use hookwars_social::profiles::{record_wallet_cpi, RecordAccs};
use hookwars_common::economy::{self as eco, counter, fee_source, skill};

pub mod error;
pub mod events;
pub mod state;

use error::CraftError;
use events::*;
use state::*;

declare_id!("39LXQBGqZtg591jkGnZi9BELQ9hp1ZngbAxu6K1cC29Y");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "units craft",
    project_url: "https://github.com/plsdontcallmyfone/units",
    contacts: "link:https://github.com/plsdontcallmyfone/units/security/advisories/new",
    policy: "https://github.com/plsdontcallmyfone/units/blob/main/SECURITY.md",
    source_code: "https://github.com/plsdontcallmyfone/units"
}

/// `sha256("global:mint_crafted")[..8]`: the output program's entry point.
pub const MINT_CRAFTED_DISC: [u8; 8] = [121, 196, 34, 30, 90, 249, 235, 177];

fn now() -> Result<i64> {
    Ok(Clock::get()?.unix_timestamp)
}

fn upgrade_authority(program_data: &AccountInfo) -> Result<Option<Pubkey>> {
    require_keys_eq!(
        *program_data.key,
        hookwars_common::programdata_address(&crate::ID),
        CraftError::NotUpgradeAuthority
    );
    require_keys_eq!(
        *program_data.owner,
        hookwars_common::ids::BPF_LOADER_UPGRADEABLE_ID,
        CraftError::NotUpgradeAuthority
    );
    let data = program_data.try_borrow_data()?;
    require!(
        data.len() >= 45 && data[..4] == [3, 0, 0, 0],
        CraftError::NotUpgradeAuthority
    );
    if data[12] == 0 {
        return Ok(None);
    }
    Ok(Some(Pubkey::new_from_array(data[13..45].try_into().unwrap())))
}

fn check_params(p: &CraftParams) -> Result<()> {
    require!(
        p.recipe_protocol_bps <= 10_000
            && p.max_inputs > 0
            && usize::from(p.max_inputs) <= INPUTS_CAP
            && p.season_secs > 0,
        CraftError::BadParams
    );
    Ok(())
}

fn check_callers(callers: &[Pubkey]) -> Result<()> {
    require!(callers.len() <= CALLERS_CAP, CraftError::BadParams);
    for (i, c) in callers.iter().enumerate() {
        require!(!callers[..i].contains(c), CraftError::BadParams);
    }
    Ok(())
}

fn check_recipe(t: &RecipeTerms, p: &CraftParams) -> Result<()> {
    require!(
        t.kind <= recipe_kind::REPAIR
            && !t.inputs.is_empty()
            && t.inputs.len() <= usize::from(p.max_inputs)
            && t.inputs.iter().all(|i| i.amount > 0),
        CraftError::BadRecipe
    );
    for (i, a) in t.inputs.iter().enumerate() {
        require!(
            !t.inputs[..i].iter().any(|b| b.material_id == a.material_id),
            CraftError::BadRecipe
        );
    }
    if t.kind == recipe_kind::CRAFT {
        require!(
            t.param_min.iter().zip(t.param_max.iter()).all(|(a, b)| a <= b),
            CraftError::BadRecipe
        );
    } else {
        require!(t.charges_restored > 0, CraftError::BadRecipe);
    }
    Ok(())
}

/// The signer is the `["craft-caller"]` PDA of `caller_program`, a program the config names.
fn check_caller(config: &CraftConfig, caller: &Pubkey, caller_program: &Pubkey) -> Result<()> {
    require!(
        config.callers.contains(caller_program)
            && *caller == eco::caller_pda(eco::CRAFT_CALLER_SEED, caller_program).0,
        CraftError::NotCaller
    );
    Ok(())
}

fn ready_at(config: &CraftConfig) -> Result<i64> {
    now()?
        .checked_add(i64::from(config.params.admin_timelock_secs))
        .ok_or_else(|| error!(CraftError::Overflow))
}

/// Moves `lamports` from the signer `from` to `to`. Zero is a no-op.
fn pay_sol<'info>(system: &AccountInfo<'info>, from: &AccountInfo<'info>, to: &AccountInfo<'info>, lamports: u64) -> Result<()> {
    if lamports == 0 {
        return Ok(());
    }
    system_program::transfer(
        CpiContext::new(
            system.key(),
            system_program::Transfer {
                from: from.clone(),
                to: to.clone(),
            },
        ),
        lamports,
    )
}

/// The token program's accounts every token call here takes.
pub struct TokenAccs<'a, 'info> {
    pub token_program: &'a AccountInfo<'info>,
    pub event_authority: &'a AccountInfo<'info>,
    pub system_program: &'a AccountInfo<'info>,
}

impl TokenAccs<'_, '_> {
    fn check(&self) -> Result<()> {
        require_keys_eq!(*self.token_program.key, bordrless_token::ID, CraftError::WrongAccount);
        require_keys_eq!(
            *self.event_authority.key,
            bordrless_token::client::event_authority(),
            CraftError::WrongAccount
        );
        Ok(())
    }
}

fn create_holding<'info>(
    t: &TokenAccs<'_, 'info>,
    payer: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    owner: &AccountInfo<'info>,
    holding: &AccountInfo<'info>,
) -> Result<()> {
    require_keys_eq!(
        *holding.key,
        bordrless_token::client::holding_address(mint.key, owner.key),
        CraftError::WrongAccount
    );
    let ix = bordrless_token::client::create_holding(payer.key(), mint.key(), owner.key());
    invoke(
        &ix,
        &[
            payer.clone(),
            mint.clone(),
            owner.clone(),
            holding.clone(),
            t.system_program.clone(),
            t.event_authority.clone(),
            t.token_program.clone(),
        ],
    )?;
    Ok(())
}

/// Burns `amount` of the material `mint` from `holding`, signed by `owner`.
fn burn<'info>(
    t: &TokenAccs<'_, 'info>,
    owner: &AccountInfo<'info>,
    holding: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    amount: u64,
) -> Result<()> {
    let ix = bordrless_token::client::burn(owner.key(), holding.key(), mint.key(), None, vec![], amount);
    invoke(
        &ix,
        &[
            owner.clone(),
            holding.clone(),
            mint.clone(),
            t.token_program.clone(),
            t.event_authority.clone(),
        ],
    )?;
    Ok(())
}

/// Burns a recipe's inputs: `accounts` holds `(material, material_mint, holding)` per input, in the
/// recipe's order. Returns how many accounts it used.
fn burn_inputs<'info>(
    t: &TokenAccs<'_, 'info>,
    owner: &AccountInfo<'info>,
    inputs: &[RecipeInput],
    accounts: &[AccountInfo<'info>],
) -> Result<usize> {
    let n = inputs.len() * 3;
    require!(accounts.len() >= n, CraftError::WrongAccount);
    for (i, input) in inputs.iter().enumerate() {
        let (m_info, mint, holding) = (&accounts[i * 3], &accounts[i * 3 + 1], &accounts[i * 3 + 2]);
        require_keys_eq!(*m_info.owner, crate::ID, CraftError::WrongAccount);
        require_keys_eq!(*m_info.key, material_address(input.material_id).0, CraftError::WrongAccount);
        let mut m = {
            let data = m_info.try_borrow_data()?;
            Material::try_deserialize(&mut &data[..])?
        };
        require_keys_eq!(m.mint, *mint.key, CraftError::WrongAccount);
        burn(t, owner, holding, mint, input.amount)?;
        m.burned_total = m.burned_total.saturating_add(input.amount);
        let mut data = m_info.try_borrow_mut_data()?;
        let mut w: &mut [u8] = &mut data[..];
        m.try_serialize(&mut w)?;
    }
    Ok(n)
}

/// Pays a recipe fee: `RECIPE_PROTOCOL_BPS` to the treasury, the rest to the season pool.
#[allow(clippy::too_many_arguments)]
fn pay_fee<'info>(
    config: &mut CraftConfig,
    system: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    treasury: &AccountInfo<'info>,
    season_pool: &AccountInfo<'info>,
    fee: u64,
    reference: [u8; 32],
    ts: i64,
) -> Result<u64> {
    let protocol = eco::bps(fee, config.params.recipe_protocol_bps);
    pay_sol(system, payer, treasury, protocol)?;
    pay_sol(system, payer, season_pool, fee - protocol)?;
    config.protocol_fees_total = config.protocol_fees_total.saturating_add(u128::from(protocol));
    if protocol > 0 {
        emit!(ProtocolFee {
            source: fee_source::RECIPE,
            mint: Pubkey::default(),
            amount: protocol,
            reference,
            ts
        });
    }
    Ok(protocol)
}

/// Applies a recipe's pending terms once ready (lazily, from any instruction that reads it).
fn settle_recipe(r: &mut Recipe, ts: i64) {
    if let Some(p) = r.pending.clone() {
        if ts >= r.pending_at {
            r.terms = p;
            r.pending = None;
        }
    }
}

#[program]
pub mod hookwars_craft {
    use super::*;

    // ---- config

    /// Creates the config; the program's upgrade authority signs. Season 0 starts now.
    pub fn init(
        ctx: Context<Init>,
        admin: Pubkey,
        treasury: Pubkey,
        season_pool: Pubkey,
        output_program: Pubkey,
        callers: Vec<Pubkey>,
        params: CraftParams,
    ) -> Result<()> {
        let up = upgrade_authority(&ctx.accounts.program_data)?;
        require!(up == Some(ctx.accounts.authority.key()), CraftError::NotUpgradeAuthority);
        check_params(&params)?;
        check_callers(&callers)?;
        let c = &mut ctx.accounts.config;
        c.version = VERSION;
        c.bump = ctx.bumps.config;
        c.admin = admin;
        c.treasury = treasury;
        c.season_pool = season_pool;
        c.output_program = output_program;
        c.callers = callers;
        c.params = params;
        c.season_origin = now()?;
        c.protocol_fees_total = 0;
        c.reserved = [0; 32];
        Ok(())
    }

    /// The admin proposes a new treasury, season pool, output program, callers and params; they
    /// apply after `admin_timelock_secs`.
    pub fn propose_config(
        ctx: Context<ProposeConfig>,
        treasury: Pubkey,
        season_pool: Pubkey,
        output_program: Pubkey,
        callers: Vec<Pubkey>,
        params: CraftParams,
    ) -> Result<()> {
        check_params(&params)?;
        check_callers(&callers)?;
        let ready = ready_at(&ctx.accounts.config)?;
        let p = &mut ctx.accounts.pending;
        p.bump = ctx.bumps.pending;
        p.treasury = treasury;
        p.season_pool = season_pool;
        p.output_program = output_program;
        p.callers = callers;
        p.params = params;
        p.ready_at = ready;
        p.active = true;
        emit_cpi!(CraftParamsProposed { ready_at: ready });
        Ok(())
    }

    /// Anyone applies a pending config change once it is ready.
    pub fn apply_config(ctx: Context<ApplyConfig>) -> Result<()> {
        let p = &mut ctx.accounts.pending;
        require!(p.active && now()? >= p.ready_at, CraftError::NotReady);
        let c = &mut ctx.accounts.config;
        c.treasury = p.treasury;
        c.season_pool = p.season_pool;
        c.output_program = p.output_program;
        c.callers = p.callers.clone();
        c.params = p.params;
        p.active = false;
        Ok(())
    }

    // ---- materials and drops (5.2, R42)

    /// The admin creates a material and its mint (no hook, decimals 0, mint authority
    /// `["craft-minter"]`). Nothing drops until a drop rule names it.
    pub fn create_material(
        ctx: Context<CreateMaterial>,
        id: u16,
        name: String,
        symbol: String,
        emission_cap_per_season: u64,
    ) -> Result<()> {
        require!(
            !name.is_empty() && name.len() <= NAME_MAX_LEN && !symbol.is_empty() && symbol.len() <= SYMBOL_MAX_LEN,
            CraftError::BadName
        );
        let a = &ctx.accounts;
        let t = TokenAccs {
            token_program: &a.token_program,
            event_authority: &a.token_event_authority,
            system_program: &a.system_program.to_account_info(),
        };
        t.check()?;
        let (minter, _) = minter_address();
        let args = bordrless_token::CreateMintArgs {
            decimals: 0,
            name: name.clone(),
            symbol,
            uri: String::new(),
            max_supply: 0,
            mint_authority: Some(minter),
            freeze_authority: None,
            hook_program: None,
            hook_flags: 0,
            hook_authority: None,
            metadata_authority: None,
        };
        let ix = bordrless_token::client::create_mint(a.admin.key(), a.material_mint.key(), args);
        let id_bytes = id.to_le_bytes();
        let mint_seeds: &[&[u8]] = &[seeds::MATERIAL_MINT, &id_bytes, &[ctx.bumps.material_mint]];
        invoke_signed(
            &ix,
            &[
                a.admin.to_account_info(),
                a.material_mint.to_account_info(),
                a.system_program.to_account_info(),
                a.token_event_authority.to_account_info(),
                a.token_program.to_account_info(),
            ],
            &[mint_seeds],
        )?;
        let ts = now()?;
        let season = a.config.season_at(ts);
        let mint = a.material_mint.key();
        let m = &mut ctx.accounts.material;
        m.version = VERSION;
        m.bump = ctx.bumps.material;
        m.id = id;
        m.mint = mint;
        m.name = name;
        m.emission_cap_per_season = emission_cap_per_season;
        m.emitted_this_season = 0;
        m.season = season;
        m.emitted_total = 0;
        m.burned_total = 0;
        m.pending_cap = 0;
        m.pending_cap_at = 0;
        emit_cpi!(MaterialCreated {
            id,
            mint,
            emission_cap_per_season,
            ts
        });
        Ok(())
    }

    /// The admin proposes a new season cap for a material; anyone applies it after the delay
    /// (`apply_material_cap`).
    pub fn propose_material_cap(ctx: Context<ProposeMaterialCap>, cap: u64) -> Result<()> {
        let ready = ready_at(&ctx.accounts.config)?;
        let m = &mut ctx.accounts.material;
        m.pending_cap = cap;
        m.pending_cap_at = ready;
        emit_cpi!(MaterialCapProposed {
            id: m.id,
            cap,
            ready_at: ready
        });
        Ok(())
    }

    /// Anyone applies a pending cap once ready.
    pub fn apply_material_cap(ctx: Context<ApplyMaterialCap>) -> Result<()> {
        let m = &mut ctx.accounts.material;
        require!(m.pending_cap_at != 0 && now()? >= m.pending_cap_at, CraftError::NotReady);
        m.emission_cap_per_season = m.pending_cap;
        m.pending_cap = 0;
        m.pending_cap_at = 0;
        Ok(())
    }

    /// The admin sets a drop rule. A new rule drops nothing until the delay has passed; a change
    /// to an existing rule applies after the delay (lazily, at the next `drop`).
    pub fn set_drop_rule(ctx: Context<SetDropRule>, source: u8, terms: DropTerms) -> Result<()> {
        require!(
            source < source::COUNT && terms.per_unit_den > 0,
            CraftError::BadDropRule
        );
        let ready = ready_at(&ctx.accounts.config)?;
        let r = &mut ctx.accounts.drop_rule;
        if r.version == 0 {
            r.version = VERSION;
            r.bump = ctx.bumps.drop_rule;
            r.source = source;
            r.material_id = terms.material_id;
            r.per_unit_num = terms.per_unit_num;
            r.per_unit_den = terms.per_unit_den;
            r.effective_at = ready;
            r.pending = None;
            r.pending_at = 0;
        } else {
            r.pending = Some(terms);
            r.pending_at = ready;
        }
        emit_cpi!(DropRuleProposed {
            source,
            material_id: terms.material_id,
            per_unit_num: terms.per_unit_num,
            per_unit_den: terms.per_unit_den,
            ready_at: ready
        });
        Ok(())
    }

    /// Drops material for `measured` units of verified activity of `source` to `recipient`. Only a
    /// registered caller's `["craft-caller"]` PDA signs. Never fails for a spent cap or a rule
    /// not yet in effect (it drops nothing), so the caller's own path is never blocked.
    pub fn drop<'info>(
        ctx: Context<'info, Drop<'info>>,
        caller_program: Pubkey,
        source: u8,
        measured: u64,
    ) -> Result<()> {
        check_caller(&ctx.accounts.config, &ctx.accounts.caller.key(), &caller_program)?;
        let ts = now()?;
        let r = &mut ctx.accounts.drop_rule;
        require!(r.source == source, CraftError::WrongAccount);
        if let Some(p) = r.pending {
            if ts >= r.pending_at {
                r.material_id = p.material_id;
                r.per_unit_num = p.per_unit_num;
                r.per_unit_den = p.per_unit_den;
                r.pending = None;
            }
        }
        if ts < r.effective_at {
            return Ok(());
        }
        let m = &mut ctx.accounts.material;
        require!(m.id == r.material_id, CraftError::WrongAccount);
        let season = ctx.accounts.config.season_at(ts);
        if m.season != season {
            m.season = season;
            m.emitted_this_season = 0;
        }
        let want = u128::from(measured) * u128::from(r.per_unit_num) / u128::from(r.per_unit_den);
        let left = m.emission_cap_per_season.saturating_sub(m.emitted_this_season);
        let amount = u64::try_from(want.min(u128::from(left))).unwrap_or(left);
        if amount == 0 {
            return Ok(());
        }
        m.emitted_this_season += amount;
        m.emitted_total = m.emitted_total.saturating_add(amount);
        let material_id = m.id;
        let a = &ctx.accounts;
        let sys = a.system_program.to_account_info();
        let t = TokenAccs {
            token_program: &a.token_program,
            event_authority: &a.token_event_authority,
            system_program: &sys,
        };
        t.check()?;
        create_holding(
            &t,
            &a.payer.to_account_info(),
            &a.material_mint,
            &a.recipient,
            &a.recipient_holding,
        )?;
        let (minter, bump) = minter_address();
        let ix = bordrless_token::client::mint_to(
            minter,
            a.material_mint.key(),
            a.recipient_holding.key(),
            None,
            vec![],
            amount,
        );
        invoke_signed(
            &ix,
            &[
                a.minter.to_account_info(),
                a.material_mint.to_account_info(),
                a.recipient_holding.to_account_info(),
                a.token_event_authority.to_account_info(),
                a.token_program.to_account_info(),
            ],
            &[&[seeds::MINTER, &[bump]]],
        )?;
        emit_cpi!(Dropped {
            source,
            material_id,
            caller_program,
            recipient: a.recipient.key(),
            measured,
            amount,
            season,
            ts
        });
        Ok(())
    }

    // ---- recipes (5.3)

    /// The admin creates a recipe; it is usable after the delay.
    pub fn create_recipe(ctx: Context<CreateRecipe>, id: u16, terms: RecipeTerms) -> Result<()> {
        check_recipe(&terms, &ctx.accounts.config.params)?;
        let ready = ready_at(&ctx.accounts.config)?;
        let r = &mut ctx.accounts.recipe;
        r.version = VERSION;
        r.bump = ctx.bumps.recipe;
        r.id = id;
        r.terms = terms;
        r.effective_at = ready;
        r.pending = None;
        r.pending_at = 0;
        r.uses = 0;
        emit_cpi!(RecipeProposed { id, ready_at: ready });
        Ok(())
    }

    /// The admin proposes new terms for a recipe (including `active`); they apply after the delay.
    pub fn set_recipe(ctx: Context<SetRecipe>, terms: RecipeTerms) -> Result<()> {
        check_recipe(&terms, &ctx.accounts.config.params)?;
        let ready = ready_at(&ctx.accounts.config)?;
        let r = &mut ctx.accounts.recipe;
        r.pending = Some(terms);
        r.pending_at = ready;
        emit_cpi!(RecipeProposed { id: r.id, ready_at: ready });
        Ok(())
    }

    /// Crafts: burns the inputs, pays the fee and asks the output program to make the item.
    /// Remaining accounts: `(material, material_mint, crafter holding)` per input, then the output
    /// program's `mint_crafted` accounts after its first (the craft signer).
    pub fn craft<'info>(
        mut ctx: Context<'info, CraftItem<'info>>,
        recipe_id: u16,
        reference: [u8; 32],
    ) -> Result<()> {
        let ts = now()?;
        let a = &mut ctx.accounts;
        settle_recipe(&mut a.recipe, ts);
        let terms = a.recipe.terms.clone();
        require!(a.recipe.id == recipe_id, CraftError::WrongAccount);
        require!(terms.active && ts >= a.recipe.effective_at, CraftError::RecipeClosed);
        require!(terms.kind == recipe_kind::CRAFT, CraftError::WrongRecipe);
        let s = RecordAccs {
            skills: &a.skills,
            profile: &a.profile,
            caller: &a.social_caller,
            event_authority: &a.social_event_authority,
            program: &a.social_program,
        };
        if terms.min_level > 0 {
            let lvl = hookwars_social::profiles::wallet_level(s.profile, s.skills, &a.crafter.key(), skill::CRAFTER)?;
            require!(lvl >= terms.min_level, CraftError::LevelTooLow);
        }
        let sys = a.system_program.to_account_info();
        let t = TokenAccs {
            token_program: &a.token_program,
            event_authority: &a.token_event_authority,
            system_program: &sys,
        };
        t.check()?;
        let crafter = a.crafter.to_account_info();
        let used = burn_inputs(&t, &crafter, &terms.inputs, ctx.remaining_accounts)?;
        pay_fee(
            &mut a.config,
            &sys,
            &crafter,
            &a.treasury,
            &a.season_pool,
            terms.fee_lamports,
            reference,
            ts,
        )?;
        // The output program makes the item (the armory's `mint_crafted`, integration request).
        let (signer, bump) = signer_address();
        let rest = &ctx.remaining_accounts[used..];
        let mut metas = vec![AccountMeta::new_readonly(signer, true)];
        metas.extend(rest.iter().map(|i| AccountMeta {
            pubkey: *i.key,
            is_signer: i.is_signer,
            is_writable: i.is_writable,
        }));
        let mut data = MINT_CRAFTED_DISC.to_vec();
        (
            a.crafter.key(),
            terms.template_id,
            terms.param_min.to_vec(),
            terms.param_max.to_vec(),
            recipe_id,
        )
            .serialize(&mut data)?;
        let ix = Instruction {
            program_id: a.config.output_program,
            accounts: metas,
            data,
        };
        let mut infos = vec![a.craft_signer.to_account_info()];
        infos.extend(rest.iter().cloned());
        infos.push(a.output_program.to_account_info());
        invoke_signed(&ix, &infos, &[&[seeds::SIGNER, &[bump]]])?;
        a.recipe.uses = a.recipe.uses.saturating_add(1);
        record_wallet_cpi(&s, &crate::ID, &a.crafter.key(), counter::ITEMS_CRAFTED, 1)?;
        let crafter = a.crafter.key();
        emit_cpi!(Crafted {
            recipe: recipe_id,
            crafter,
            template_id: terms.template_id,
            fee: terms.fee_lamports,
            reference,
            ts
        });
        Ok(())
    }

    /// Repairs an item the signer holds: burns the inputs, pays the fee, restores charges and
    /// wakes a dormant item. Remaining accounts: `(material, material_mint, holder holding)` per
    /// input.
    pub fn repair<'info>(
        mut ctx: Context<'info, RepairItem<'info>>,
        recipe_id: u16,
        reference: [u8; 32],
    ) -> Result<()> {
        let ts = now()?;
        let a = &mut ctx.accounts;
        settle_recipe(&mut a.recipe, ts);
        let terms = a.recipe.terms.clone();
        require!(a.recipe.id == recipe_id, CraftError::WrongAccount);
        require!(terms.active && ts >= a.recipe.effective_at, CraftError::RecipeClosed);
        require!(terms.kind == recipe_kind::REPAIR, CraftError::WrongRecipe);
        // The item: an armory item of the recipe's template, held by the signer.
        let item_mint = a.item_mint.key();
        require_keys_eq!(*a.item.owner, hookwars_common::ids::ARMORY_ID, CraftError::NotAnItem);
        require_keys_eq!(a.item.key(), hookwars_common::pda::item(&item_mint).0, CraftError::NotAnItem);
        let template_id = {
            let data = a.item.try_borrow_data()?;
            let it = hookwars_armory::state::Item::try_deserialize(&mut &data[..])
                .map_err(|_| error!(CraftError::NotAnItem))?;
            require_keys_eq!(it.item_mint, item_mint, CraftError::NotAnItem);
            it.template_id
        };
        require!(template_id == terms.template_id, CraftError::WrongRecipe);
        require_keys_eq!(
            a.holder_item_holding.key(),
            bordrless_token::client::holding_address(&item_mint, &a.holder.key()),
            CraftError::NotItemHolder
        );
        let h = bordrless_token::client::read_holding(&a.holder_item_holding)?;
        require!(h.mint == item_mint && h.owner == a.holder.key() && h.amount == 1, CraftError::NotItemHolder);
        require!(a.wear.max_charges > 0, CraftError::NeverWears);
        let s = RecordAccs {
            skills: &a.skills,
            profile: &a.profile,
            caller: &a.social_caller,
            event_authority: &a.social_event_authority,
            program: &a.social_program,
        };
        if terms.min_level > 0 {
            let lvl = hookwars_social::profiles::wallet_level(s.profile, s.skills, &a.holder.key(), skill::CRAFTER)?;
            require!(lvl >= terms.min_level, CraftError::LevelTooLow);
        }
        let sys = a.system_program.to_account_info();
        let t = TokenAccs {
            token_program: &a.token_program,
            event_authority: &a.token_event_authority,
            system_program: &sys,
        };
        t.check()?;
        let holder = a.holder.to_account_info();
        burn_inputs(&t, &holder, &terms.inputs, ctx.remaining_accounts)?;
        pay_fee(
            &mut a.config,
            &sys,
            &holder,
            &a.treasury,
            &a.season_pool,
            terms.fee_lamports,
            reference,
            ts,
        )?;
        let w = &mut a.wear;
        w.used = w.used.saturating_sub(terms.charges_restored);
        if w.used < w.max_charges {
            w.dormant = false;
        }
        w.repairs = w.repairs.saturating_add(1);
        w.updated_at = ts;
        let used = w.used;
        a.recipe.uses = a.recipe.uses.saturating_add(1);
        record_wallet_cpi(&s, &crate::ID, &a.holder.key(), counter::REPAIRS, 1)?;
        let (item, holder) = (a.item.key(), a.holder.key());
        emit_cpi!(Repaired {
            recipe: recipe_id,
            item,
            holder,
            restored: terms.charges_restored,
            used,
            fee: terms.fee_lamports,
            reference,
            ts
        });
        Ok(())
    }

    // ---- wear (5.4)

    /// Opens an item's wear with `max_charges` (0 = never wears). Only a registered caller's
    /// `["craft-caller"]` PDA signs (the items engine at item creation, integration request).
    pub fn init_wear(ctx: Context<InitWear>, caller_program: Pubkey, item: Pubkey, max_charges: u32) -> Result<()> {
        check_caller(&ctx.accounts.config, &ctx.accounts.caller.key(), &caller_program)?;
        let ts = now()?;
        let w = &mut ctx.accounts.wear;
        w.version = VERSION;
        w.bump = ctx.bumps.wear;
        w.item = item;
        w.max_charges = max_charges;
        w.used = 0;
        w.dormant = false;
        w.repairs = 0;
        w.updated_at = ts;
        emit_cpi!(WearOpened { item, max_charges, ts });
        Ok(())
    }

    /// Adds `runs` applied effects to an item's wear. Only a registered caller's PDA signs (the
    /// items engine at settlement, integration request). Never fails because the item is worn:
    /// at `max_charges` it turns dormant and `ItemWorn` is emitted once.
    pub fn wear(ctx: Context<WearItem>, caller_program: Pubkey, runs: u32) -> Result<()> {
        check_caller(&ctx.accounts.config, &ctx.accounts.caller.key(), &caller_program)?;
        let ts = now()?;
        let w = &mut ctx.accounts.wear;
        if w.max_charges == 0 || runs == 0 {
            return Ok(());
        }
        w.used = w.used.saturating_add(runs).min(w.max_charges);
        w.updated_at = ts;
        if w.used >= w.max_charges && !w.dormant {
            w.dormant = true;
            let item = w.item;
            emit_cpi!(ItemWorn { item, ts });
        }
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Init<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(init, payer = authority, space = 8 + CraftConfig::INIT_SPACE, seeds = [seeds::CONFIG], bump)]
    pub config: Box<Account<'info, CraftConfig>>,
    /// CHECK: this program's ProgramData, parsed in the handler.
    pub program_data: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct ProposeConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump, has_one = admin @ CraftError::NotAdmin)]
    pub config: Box<Account<'info, CraftConfig>>,
    #[account(init_if_needed, payer = admin, space = 8 + PendingCraft::INIT_SPACE, seeds = [seeds::PENDING], bump)]
    pub pending: Box<Account<'info, PendingCraft>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ApplyConfig<'info> {
    #[account(mut, seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, CraftConfig>>,
    #[account(mut, seeds = [seeds::PENDING], bump = pending.bump)]
    pub pending: Box<Account<'info, PendingCraft>>,
}

#[event_cpi]
#[derive(Accounts)]
#[instruction(id: u16)]
pub struct CreateMaterial<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump, has_one = admin @ CraftError::NotAdmin)]
    pub config: Box<Account<'info, CraftConfig>>,
    #[account(init, payer = admin, space = 8 + Material::INIT_SPACE, seeds = [seeds::MATERIAL, &id.to_le_bytes()], bump)]
    pub material: Box<Account<'info, Material>>,
    /// CHECK: created by the token program at this PDA.
    #[account(mut, seeds = [seeds::MATERIAL_MINT, &id.to_le_bytes()], bump)]
    pub material_mint: UncheckedAccount<'info>,
    /// CHECK: the token program (checked in the handler).
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: the token program's event authority (checked in the handler).
    pub token_event_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct ProposeMaterialCap<'info> {
    pub admin: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump, has_one = admin @ CraftError::NotAdmin)]
    pub config: Box<Account<'info, CraftConfig>>,
    #[account(mut, seeds = [seeds::MATERIAL, &material.id.to_le_bytes()], bump = material.bump)]
    pub material: Box<Account<'info, Material>>,
}

#[derive(Accounts)]
pub struct ApplyMaterialCap<'info> {
    #[account(mut, seeds = [seeds::MATERIAL, &material.id.to_le_bytes()], bump = material.bump)]
    pub material: Box<Account<'info, Material>>,
}

#[event_cpi]
#[derive(Accounts)]
#[instruction(source: u8)]
pub struct SetDropRule<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump, has_one = admin @ CraftError::NotAdmin)]
    pub config: Box<Account<'info, CraftConfig>>,
    #[account(init_if_needed, payer = admin, space = 8 + DropRule::INIT_SPACE, seeds = [seeds::DROP, &[source]], bump)]
    pub drop_rule: Box<Account<'info, DropRule>>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
#[instruction(caller_program: Pubkey, source: u8)]
pub struct Drop<'info> {
    /// A registered caller's `["craft-caller"]` PDA.
    pub caller: Signer<'info>,
    /// Pays the recipient's holding when it does not exist yet.
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, CraftConfig>>,
    #[account(mut, seeds = [seeds::DROP, &[source]], bump = drop_rule.bump)]
    pub drop_rule: Box<Account<'info, DropRule>>,
    #[account(mut, seeds = [seeds::MATERIAL, &material.id.to_le_bytes()], bump = material.bump)]
    pub material: Box<Account<'info, Material>>,
    /// CHECK: the material's mint.
    #[account(mut, address = material.mint @ CraftError::WrongAccount)]
    pub material_mint: UncheckedAccount<'info>,
    /// CHECK: `["craft-minter"]`.
    #[account(seeds = [seeds::MINTER], bump)]
    pub minter: UncheckedAccount<'info>,
    /// CHECK: any wallet.
    pub recipient: UncheckedAccount<'info>,
    /// CHECK: the recipient's holding of the material (created if needed, address checked).
    #[account(mut)]
    pub recipient_holding: UncheckedAccount<'info>,
    /// CHECK: the token program (checked in the handler).
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: the token program's event authority (checked in the handler).
    pub token_event_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
#[instruction(id: u16)]
pub struct CreateRecipe<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump, has_one = admin @ CraftError::NotAdmin)]
    pub config: Box<Account<'info, CraftConfig>>,
    #[account(init, payer = admin, space = 8 + Recipe::INIT_SPACE, seeds = [seeds::RECIPE, &id.to_le_bytes()], bump)]
    pub recipe: Box<Account<'info, Recipe>>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct SetRecipe<'info> {
    pub admin: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump, has_one = admin @ CraftError::NotAdmin)]
    pub config: Box<Account<'info, CraftConfig>>,
    #[account(mut, seeds = [seeds::RECIPE, &recipe.id.to_le_bytes()], bump = recipe.bump)]
    pub recipe: Box<Account<'info, Recipe>>,
}

#[event_cpi]
#[derive(Accounts)]
#[instruction(recipe_id: u16)]
pub struct CraftItem<'info> {
    #[account(mut)]
    pub crafter: Signer<'info>,
    #[account(mut, seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, CraftConfig>>,
    #[account(mut, seeds = [seeds::RECIPE, &recipe_id.to_le_bytes()], bump = recipe.bump)]
    pub recipe: Box<Account<'info, Recipe>>,
    /// CHECK: the config's treasury.
    #[account(mut, address = config.treasury @ CraftError::WrongAccount)]
    pub treasury: UncheckedAccount<'info>,
    /// CHECK: the config's season pool.
    #[account(mut, address = config.season_pool @ CraftError::WrongAccount)]
    pub season_pool: UncheckedAccount<'info>,
    /// CHECK: `["craft-signer"]`, the only signer the output program accepts.
    #[account(seeds = [seeds::SIGNER], bump)]
    pub craft_signer: UncheckedAccount<'info>,
    /// CHECK: the config's output program.
    #[account(address = config.output_program @ CraftError::WrongAccount)]
    pub output_program: UncheckedAccount<'info>,
    /// CHECK: social `["skills"]` (read with every check).
    pub skills: UncheckedAccount<'info>,
    /// CHECK: the crafter's social profile, or its empty address (read with every check).
    #[account(mut)]
    pub profile: UncheckedAccount<'info>,
    /// CHECK: `["social-caller"]` under this program.
    pub social_caller: UncheckedAccount<'info>,
    /// CHECK: social's event authority.
    pub social_event_authority: UncheckedAccount<'info>,
    /// CHECK: the social program (checked).
    pub social_program: UncheckedAccount<'info>,
    /// CHECK: the token program (checked in the handler).
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: the token program's event authority (checked in the handler).
    pub token_event_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
#[instruction(recipe_id: u16)]
pub struct RepairItem<'info> {
    #[account(mut)]
    pub holder: Signer<'info>,
    #[account(mut, seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, CraftConfig>>,
    #[account(mut, seeds = [seeds::RECIPE, &recipe_id.to_le_bytes()], bump = recipe.bump)]
    pub recipe: Box<Account<'info, Recipe>>,
    /// CHECK: the armory item (owner, address and layout checked in the handler).
    pub item: UncheckedAccount<'info>,
    /// CHECK: the item's mint (bound through the item).
    pub item_mint: UncheckedAccount<'info>,
    /// CHECK: the holder's holding of the item mint (read with every check).
    pub holder_item_holding: UncheckedAccount<'info>,
    #[account(mut, seeds = [seeds::WEAR, item.key().as_ref()], bump = wear.bump)]
    pub wear: Box<Account<'info, Wear>>,
    /// CHECK: the config's treasury.
    #[account(mut, address = config.treasury @ CraftError::WrongAccount)]
    pub treasury: UncheckedAccount<'info>,
    /// CHECK: the config's season pool.
    #[account(mut, address = config.season_pool @ CraftError::WrongAccount)]
    pub season_pool: UncheckedAccount<'info>,
    /// CHECK: social `["skills"]`.
    pub skills: UncheckedAccount<'info>,
    /// CHECK: the holder's social profile, or its empty address.
    #[account(mut)]
    pub profile: UncheckedAccount<'info>,
    /// CHECK: `["social-caller"]` under this program.
    pub social_caller: UncheckedAccount<'info>,
    /// CHECK: social's event authority.
    pub social_event_authority: UncheckedAccount<'info>,
    /// CHECK: the social program (checked).
    pub social_program: UncheckedAccount<'info>,
    /// CHECK: the token program (checked in the handler).
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: the token program's event authority (checked in the handler).
    pub token_event_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
#[instruction(caller_program: Pubkey, item: Pubkey)]
pub struct InitWear<'info> {
    /// A registered caller's `["craft-caller"]` PDA.
    pub caller: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, CraftConfig>>,
    #[account(init, payer = payer, space = 8 + Wear::INIT_SPACE, seeds = [seeds::WEAR, item.as_ref()], bump)]
    pub wear: Box<Account<'info, Wear>>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct WearItem<'info> {
    /// A registered caller's `["craft-caller"]` PDA.
    pub caller: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, CraftConfig>>,
    #[account(mut, seeds = [seeds::WEAR, wear.item.as_ref()], bump = wear.bump)]
    pub wear: Box<Account<'info, Wear>>,
}
