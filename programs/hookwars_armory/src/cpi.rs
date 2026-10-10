// Changed by Hookwars: new file (M2); R21 burns fit only may_burn slots; integration pass 3: init_equip's wear flag (E-3);
// protocol pass 4a: external templates validate against their declared manifest and go into the slot with
// their own program.
//! Calls out of the armory: into `hookwars_items` (signed by `["armory"]`, 02 section 2.2) and into
//! the token program (signed by `["minter"]`, `["slots", mint]` or `["royalty", item]`), plus the
//! compatibility check (`check_fits`, 02 section 6.1) and the equip steps every path shares
//! (02 section 6.4 steps 2 to 4).

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::{get_return_data, invoke, invoke_signed};
use bordrless_token::state::Mint;
use hookwars_common::{
    current_deploy_slot, ids, seeds, token_flags, EquipConfig, Manifest, Params,
};

use crate::error::ArmoryError;
use crate::state::*;

/// `["armory"]` under the armory and its bump (checked in the unit tests).
pub const ARMORY_SIGNER: Pubkey =
    Pubkey::from_str_const("2NSNJ4W51G5ZZSc4wVqkjyR8yr5yzuEjquKv7iprzSHP");
pub const ARMORY_SIGNER_BUMP: u8 = 254;
/// `["minter"]` under the armory and its bump.
pub const MINTER: Pubkey = Pubkey::from_str_const("GMD68dRY3cr58rLgwyNtxyGFjGRVg2RUzmeNo7X7CVPe");
pub const MINTER_BUMP: u8 = 254;
/// `["loot-signer"]` under the war program: the only signer `mint_loot` accepts.
pub const LOOT_SIGNER: Pubkey =
    Pubkey::from_str_const("DK5PUA6578wDF96DAEqxqiUDypvqPvdaoYASisgyPuZv");

/// `sha256("global:<name>")[..8]` of the items program's entry points.
pub mod disc {
    pub const VALIDATE_PARAMS: [u8; 8] = [107, 173, 134, 85, 141, 25, 150, 229];
    pub const MANIFEST: [u8; 8] = [64, 185, 112, 86, 80, 97, 120, 179];
    pub const COMBINE_PARAMS: [u8; 8] = [14, 103, 151, 46, 238, 102, 239, 27];
    pub const INIT_EQUIP: [u8; 8] = [244, 251, 246, 211, 6, 225, 213, 3];
    pub const CLOSE_EQUIP: [u8; 8] = [56, 2, 244, 11, 20, 110, 162, 153];
}

fn armory_seeds() -> [&'static [u8]; 2] {
    [seeds::ARMORY, &[ARMORY_SIGNER_BUMP]]
}

fn items_call<'info>(
    data: Vec<u8>,
    metas: Vec<AccountMeta>,
    infos: &[AccountInfo<'info>],
) -> Result<()> {
    let ix = Instruction {
        program_id: ids::ITEMS_ID,
        accounts: metas,
        data,
    };
    invoke_signed(&ix, infos, &[&armory_seeds()])?;
    Ok(())
}

fn items_return() -> Result<Vec<u8>> {
    let (program, data) = get_return_data().ok_or(ArmoryError::BadReturn)?;
    require_keys_eq!(program, ids::ITEMS_ID, ArmoryError::BadReturn);
    Ok(data)
}

/// `validate_params` (02 section 4.1 step 3).
pub fn validate_params<'info>(
    signer: &AccountInfo<'info>,
    items: &AccountInfo<'info>,
    template: &Template,
    params: &Params,
) -> Result<()> {
    let mut data = disc::VALIDATE_PARAMS.to_vec();
    (template.id, template.field_min, template.field_max, *params).serialize(&mut data)?;
    items_call(
        data,
        vec![AccountMeta::new_readonly(ARMORY_SIGNER, true)],
        &[signer.clone(), items.clone()],
    )
}

/// `manifest` (02 section 4.1 step 4).
pub fn manifest<'info>(
    signer: &AccountInfo<'info>,
    items: &AccountInfo<'info>,
    template: &Template,
    params: &Params,
) -> Result<Manifest> {
    let mut data = disc::MANIFEST.to_vec();
    (template.id, *params, template.max_targets).serialize(&mut data)?;
    items_call(
        data,
        vec![AccountMeta::new_readonly(ARMORY_SIGNER, true)],
        &[signer.clone(), items.clone()],
    )?;
    let ret = items_return()?;
    Manifest::try_from_slice(&ret).map_err(|_| ArmoryError::BadReturn.into())
}

/// `combine_params` (02 section 9.1 step 1).
pub fn combine_params<'info>(
    signer: &AccountInfo<'info>,
    items: &AccountInfo<'info>,
    template: &Template,
    gain_bps: u16,
    a: &Params,
    b: &Params,
) -> Result<Params> {
    let mut data = disc::COMBINE_PARAMS.to_vec();
    (template.id, template.field_min, template.field_max, gain_bps, *a, *b).serialize(&mut data)?;
    items_call(
        data,
        vec![AccountMeta::new_readonly(ARMORY_SIGNER, true)],
        &[signer.clone(), items.clone()],
    )?;
    let ret = items_return()?;
    Params::try_from_slice(&ret).map_err(|_| ArmoryError::BadReturn.into())
}

/// Validation shared by every creation path (02 section 4.1): returns the manifest.
pub fn validate_item<'info>(
    signer: &AccountInfo<'info>,
    items: &AccountInfo<'info>,
    template: &Template,
    params: &Params,
) -> Result<Manifest> {
    require!(
        template.status == template_status::ACTIVE,
        ArmoryError::TemplateClosed
    );
    // Protocol pass 4a: an external template's fields are checked against its floors and
    // ceilings; its items carry the manifest its registrant declared (the lab measured the
    // program against it).
    if template.external {
        for (i, p) in params.iter().enumerate() {
            let ok = if i < usize::from(template.field_count) {
                *p >= template.field_min[i] && *p <= template.field_max[i]
            } else {
                *p == 0
            };
            require!(ok, ArmoryError::ParamOutOfRange);
        }
        return Ok(template.ext_manifest);
    }
    hookwars_common::check_fields(template.id, &template.field_min, &template.field_max, params)
        .map_err(|_| ArmoryError::ParamOutOfRange)?;
    validate_params(signer, items, template, params)?;
    let m = manifest(signer, items, template, params)?;
    require!(m.kind == template.kind, ArmoryError::KindMismatch);
    Ok(m)
}

/// Infos of a token CPI: every account `ix` names, in any order, plus the token program.
fn token_invoke<'info>(
    ix: &Instruction,
    infos: &[AccountInfo<'info>],
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    if signer_seeds.is_empty() {
        invoke(ix, infos)?;
    } else {
        invoke_signed(ix, infos, signer_seeds)?;
    }
    Ok(())
}

/// Accounts the token CPIs of the armory use.
pub struct TokenInfos<'a, 'info> {
    pub token_program: &'a AccountInfo<'info>,
    pub event_authority: &'a AccountInfo<'info>,
    pub system_program: &'a AccountInfo<'info>,
}

/// `create_holding` (idempotent).
pub fn create_holding<'info>(
    t: &TokenInfos<'_, 'info>,
    payer: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    owner: &AccountInfo<'info>,
    holding: &AccountInfo<'info>,
) -> Result<()> {
    let ix = bordrless_token::client::create_holding(payer.key(), mint.key(), owner.key());
    token_invoke(
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
        &[],
    )
}

/// Creates an item mint at `["item-mint", n]` with one token in `holding`, then revokes the mint
/// authority (02 section 2.5).
#[allow(clippy::too_many_arguments)]
pub fn mint_item<'info>(
    t: &TokenInfos<'_, 'info>,
    payer: &AccountInfo<'info>,
    minter: &AccountInfo<'info>,
    item_mint: &AccountInfo<'info>,
    item_mint_bump: u8,
    n: u64,
    owner: &AccountInfo<'info>,
    holding: &AccountInfo<'info>,
    name: &str,
) -> Result<()> {
    require_keys_eq!(minter.key(), MINTER, ArmoryError::WrongAccount);
    let n_bytes = n.to_le_bytes();
    let mint_seeds: &[&[u8]] = &[seeds::ITEM_MINT, &n_bytes, &[item_mint_bump]];
    let minter_seeds: &[&[u8]] = &[seeds::MINTER, &[MINTER_BUMP]];
    let args = bordrless_token::CreateMintArgs {
        decimals: 0,
        name: name.to_string(),
        symbol: "ITEM".to_string(),
        uri: String::new(),
        max_supply: 1,
        mint_authority: Some(MINTER),
        freeze_authority: None,
        hook_program: None,
        hook_flags: 0,
        hook_authority: None,
        metadata_authority: None,
    };
    let ix = bordrless_token::client::create_mint(payer.key(), item_mint.key(), args);
    token_invoke(
        &ix,
        &[
            payer.clone(),
            item_mint.clone(),
            t.system_program.clone(),
            t.event_authority.clone(),
            t.token_program.clone(),
        ],
        &[mint_seeds],
    )?;
    create_holding(t, payer, item_mint, owner, holding)?;
    let ix = bordrless_token::client::mint_to(
        MINTER,
        item_mint.key(),
        holding.key(),
        None,
        vec![],
        1,
    );
    token_invoke(
        &ix,
        &[
            minter.clone(),
            item_mint.clone(),
            holding.clone(),
            t.event_authority.clone(),
            t.token_program.clone(),
        ],
        &[minter_seeds],
    )?;
    let ix = bordrless_token::client::set_authority(
        MINTER,
        item_mint.key(),
        bordrless_token::AuthorityKind::Mint,
        None,
    );
    token_invoke(
        &ix,
        &[
            minter.clone(),
            item_mint.clone(),
            t.event_authority.clone(),
            t.token_program.clone(),
        ],
        &[minter_seeds],
    )
}

/// Reads the mint (owner and discriminator checked).
pub fn read_mint(info: &AccountInfo) -> Result<Mint> {
    bordrless_token::client::read_mint(info)
}

/// Integration pass 2 (09 section 21 item 1, R28): an agent badge mint. One slot, kind Defense,
/// equip rule Locked, and the agents signer as freeze authority.
pub fn is_badge(mint: &Mint) -> bool {
    mint.slot_count == 1
        && mint.slots[0].kind == hookwars_common::kind::DEFENSE
        && mint.slots[0].equip_rule == bordrless_hook::equip_rule::LOCKED
        && mint.freeze_authority == Some(hookwars_common::ids::AGENTS_SIGNER)
}

/// Integration pass 2 (09 section 21 item 6): refuses while `["bond-mark", proposal]` under the
/// agents program exists and its bond is `Posted` (bond status byte 0). Layouts: `BondMark` =
/// discriminator, bond (32), bump; `Bond` = discriminator, six keys, amount, posted_at,
/// ratified_at, status.
pub fn require_no_posted_bond(proposal: &Pubkey, mark: &AccountInfo, bond: Option<AccountInfo>) -> Result<()> {
    let agents = hookwars_common::ids::AGENTS_ID;
    let expect = Pubkey::find_program_address(&[b"bond-mark", proposal.as_ref()], &agents).0;
    require_keys_eq!(mark.key(), expect, crate::error::ArmoryError::WrongAccount);
    if mark.owner != &agents || mark.data_is_empty() {
        return Ok(());
    }
    let d = mark.try_borrow_data()?;
    require!(d.len() >= 40, crate::error::ArmoryError::WrongAccount);
    let bond_key = Pubkey::try_from(&d[8..40]).map_err(|_| error!(crate::error::ArmoryError::WrongAccount))?;
    let b = bond.ok_or(crate::error::ArmoryError::BondStillPosted)?;
    require_keys_eq!(b.key(), bond_key, crate::error::ArmoryError::WrongAccount);
    if b.owner != &agents || b.data_is_empty() {
        return Ok(());
    }
    let bd = b.try_borrow_data()?;
    const STATUS: usize = 8 + 32 * 6 + 8 * 3;
    require!(bd.len() > STATUS, crate::error::ArmoryError::WrongAccount);
    require!(bd[STATUS] != 0, crate::error::ArmoryError::BondStillPosted);
    Ok(())
}

/// `["armory-caller", mint]` under the agents program: the signer of a badge's `equip_launch`.
pub fn agents_armory_caller(mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"armory-caller", mint.as_ref()], &hookwars_common::ids::AGENTS_ID).0
}

/// Hookwars R21: an item that may burn fits only a slot whose bounds allow burns.
pub fn burn_fits(item_may_burn: bool, slot_may_burn: bool) -> bool {
    !item_may_burn || slot_may_burn
}

/// The compatibility check (02 section 6.1). `launch`: the `equip_launch` path (fills a slot whose
/// rule is `Locked` once). `revert`: a performance revert (staleness allowed, 02 section 3.3).
#[allow(clippy::too_many_arguments)]
pub fn check_fits(
    mint: &Mint,
    slot: u8,
    item: &Pubkey,
    manifest: &Manifest,
    template: &Template,
    program: Option<&AccountInfo>,
    programdata: Option<&AccountInfo>,
    params: &ArmoryParams,
    launch: bool,
    revert: bool,
) -> Result<()> {
    require!(slot < mint.slot_count, ArmoryError::SlotIndexOutOfRange);
    let s = mint.slots[usize::from(slot)];
    require!(s.kind != hookwars_common::kind::LOCKED, ArmoryError::SlotLocked);
    require!(
        launch || s.equip_rule != bordrless_hook::equip_rule::LOCKED,
        ArmoryError::SlotLocked
    );
    if !revert {
        require!(
            template.status == template_status::ACTIVE,
            ArmoryError::TemplateClosed
        );
        if let Some(expected) = template.deploy_slot {
            let (Some(p), Some(pd)) = (program, programdata) else {
                return err!(ArmoryError::ProgramDataMissing);
            };
            require_keys_eq!(p.key(), template.program, ArmoryError::WrongAccount);
            require!(
                current_deploy_slot(p, pd) == Some(expected),
                ArmoryError::TemplateChanged
            );
        }
    }
    require!(manifest.kind == s.kind, ArmoryError::KindMismatch);
    require!(
        manifest.max_cut_transfer_bps <= s.bounds.max_cut_bps,
        ArmoryError::OverBounds
    );
    require!(
        manifest.max_cut_buy_bps <= params.max_pool_item_cut_bps
            && manifest.max_cut_sell_bps <= params.max_pool_item_cut_bps,
        ArmoryError::OverBounds
    );
    require!(
        manifest.max_discount_bps <= params.max_pool_item_discount_bps,
        ArmoryError::OverBounds
    );
    require!(
        manifest.reads_other_pools <= params.max_item_reads,
        ArmoryError::OverBounds
    );
    require!(!manifest.may_refuse || s.bounds.may_refuse, ArmoryError::OverBounds);
    // Hookwars R21: an item that may burn fits only a slot whose bounds allow burns (Pool slots).
    require!(burn_fits(manifest.may_burn, s.bounds.may_burn), ArmoryError::OverBounds);
    require!(
        manifest.token_flags & token_flags::MINT == 0,
        ArmoryError::OverBounds
    );
    require!(
        manifest.token_flags & token_flags::ANSWERS_TOUCH == 0 || s.bounds.may_answer_touch,
        ArmoryError::OverBounds
    );
    let writes = manifest.token_flags & token_flags::WRITES_HOOK_DATA != 0;
    if writes || manifest.data_bytes > 0 {
        require!(s.bounds.may_write_data, ArmoryError::OverBounds);
        require!(
            u16::from(manifest.data_bytes) < u16::from(s.data_len),
            ArmoryError::DataRangeTooSmall
        );
    }
    for (i, other) in mint.slots[..usize::from(mint.slot_count)].iter().enumerate() {
        if i != usize::from(slot) {
            require!(other.item != *item, ArmoryError::AlreadyEquipped);
        }
    }
    Ok(())
}

/// What `apply_equip` puts in a slot.
pub struct NewEquip<'a, 'info> {
    pub item: &'a AccountInfo<'info>,
    pub template_id: u16,
    pub manifest: Manifest,
    pub max_targets: u8,
    pub config: EquipConfig,
    /// Hookwars M3b: the module list of a composite.
    pub composite: Option<&'a AccountInfo<'info>>,
    /// Protocol pass 4a: the program the slot calls (the items program, or an external
    /// template's own program).
    pub program: Pubkey,
}

/// Accounts `apply_equip` uses, all already checked by the caller's context.
pub struct EquipInfos<'a, 'info> {
    pub payer: &'a AccountInfo<'info>,
    pub mint: &'a AccountInfo<'info>,
    pub slot_authority: &'a AccountInfo<'info>,
    pub slot_authority_bump: u8,
    pub equip_state: &'a AccountInfo<'info>,
    pub registry: Option<&'a AccountInfo<'info>>,
    pub old_equip_vault: Option<&'a AccountInfo<'info>>,
    pub new_equip_vault: Option<&'a AccountInfo<'info>>,
    pub royalty_owner: Option<&'a AccountInfo<'info>>,
    pub royalty_holding_token: Option<&'a AccountInfo<'info>>,
    pub quote_mint: Option<&'a AccountInfo<'info>>,
    pub royalty_holding_quote: Option<&'a AccountInfo<'info>>,
    pub armory_signer: &'a AccountInfo<'info>,
    pub items_program: &'a AccountInfo<'info>,
    pub token: TokenInfos<'a, 'info>,
    /// Protocol pass 4a: the new template's program (needed when it is external).
    pub template_program: Option<&'a AccountInfo<'info>>,
}

/// The equip steps (02 section 6.4 steps 2 to 4): unequip the slot's current item (if any),
/// create the new item's royalty holdings, `init_equip`, then `set_slot_item` (or empty the slot).
pub fn apply_equip<'info>(
    e: &EquipInfos<'_, 'info>,
    slot: u8,
    old_present: bool,
    new: Option<NewEquip<'_, 'info>>,
) -> Result<()> {
    let mint_key = e.mint.key();
    let (state_key, _) = hookwars_common::pda::equip_state(&mint_key, slot);
    require_keys_eq!(e.equip_state.key(), state_key, ArmoryError::WrongAccount);
    let items = e.items_program;
    // 2. Unequip.
    if old_present {
        let mut data = disc::CLOSE_EQUIP.to_vec();
        slot.serialize(&mut data)?;
        let vault = e.old_equip_vault.map(|v| v.key());
        let metas = vec![
            AccountMeta::new_readonly(ARMORY_SIGNER, true),
            AccountMeta::new_readonly(mint_key, false),
            AccountMeta::new(state_key, false),
            AccountMeta::new_readonly(vault.unwrap_or(ids::ITEMS_ID), false),
        ];
        let mut infos = vec![
            e.armory_signer.clone(),
            e.mint.clone(),
            e.equip_state.clone(),
            items.clone(),
        ];
        if let Some(v) = e.old_equip_vault {
            infos.push(v.clone());
        }
        items_call(data, metas, &infos)?;
    }
    let authority_seeds: &[&[u8]] = &[
        seeds::SLOTS,
        mint_key.as_ref(),
        &[e.slot_authority_bump],
    ];
    let Some(new) = new else {
        // Empty the slot.
        let ix = bordrless_token::client::set_slot_item(
            e.slot_authority.key(),
            mint_key,
            bordrless_token::ID,
            None,
            slot,
            Pubkey::default(),
            0,
            0,
            0,
        );
        return token_invoke(
            &ix,
            &[
                e.slot_authority.clone(),
                e.mint.clone(),
                e.token.event_authority.clone(),
                e.token.token_program.clone(),
            ],
            &[authority_seeds],
        );
    };
    let m = new.manifest;
    // 3. Royalty holdings for every cut mint the manifest can cut in (02 section 2.6).
    let item_key = new.item.key();
    if m.token_cuts() || m.pool_cuts() {
        let owner = e.royalty_owner.ok_or(ArmoryError::WrongAccount)?;
        require_keys_eq!(
            owner.key(),
            hookwars_common::pda::royalty_owner(&item_key).0,
            ArmoryError::WrongAccount
        );
        if m.token_cuts() {
            let h = e.royalty_holding_token.ok_or(ArmoryError::WrongAccount)?;
            create_holding(&e.token, e.payer, e.mint, owner, h)?;
        }
        if m.pool_cuts() {
            let q = e.quote_mint.ok_or(ArmoryError::WrongAccount)?;
            require_keys_eq!(q.key(), ids::BRIDGED_SOL_MINT, ArmoryError::WrongAccount);
            let h = e.royalty_holding_quote.ok_or(ArmoryError::WrongAccount)?;
            create_holding(&e.token, e.payer, q, owner, h)?;
        }
    }
    // init_equip.
    let registry = e.registry.ok_or(ArmoryError::WrongAccount)?;
    let vault = if m.token_cuts() {
        Some(e.new_equip_vault.ok_or(ArmoryError::WrongAccount)?)
    } else {
        None
    };
    let mut data = disc::INIT_EQUIP.to_vec();
    (
        slot,
        item_key,
        new.template_id,
        m,
        new.config.clone(),
        new.max_targets,
        // Integration pass 3 (E-3): the item reads its craft `Wear` last when it wears.
        crate::state::Item::try_deserialize(&mut &new.item.try_borrow_data()?[..])?.has_wear,
    )
        .serialize(&mut data)?;
    let mut metas = vec![
        AccountMeta::new_readonly(ARMORY_SIGNER, true),
        AccountMeta::new(e.payer.key(), true),
        AccountMeta::new_readonly(mint_key, false),
        AccountMeta::new(state_key, false),
        AccountMeta::new(registry.key(), false),
        match vault {
            Some(v) => AccountMeta::new(v.key(), false),
            None => AccountMeta::new_readonly(ids::ITEMS_ID, false),
        },
        AccountMeta::new_readonly(bordrless_token::ID, false),
        AccountMeta::new_readonly(e.token.event_authority.key(), false),
        AccountMeta::new_readonly(anchor_lang::system_program::ID, false),
    ];
    let mut infos = vec![
        e.armory_signer.clone(),
        e.payer.clone(),
        e.mint.clone(),
        e.equip_state.clone(),
        registry.clone(),
        items.clone(),
        e.token.token_program.clone(),
        e.token.event_authority.clone(),
        e.token.system_program.clone(),
    ];
    if let Some(v) = vault {
        infos.push(v.clone());
    }
    // Hookwars M3b: a composite's module list follows as `init_equip`'s first remaining account.
    if new.template_id == hookwars_common::template_id::COMPOSITE {
        let c = new.composite.ok_or(ArmoryError::WrongAccount)?;
        require_keys_eq!(
            c.key(),
            hookwars_common::composite::CompositeItem::address(&item_key).0,
            ArmoryError::WrongAccount
        );
        metas.push(AccountMeta::new_readonly(c.key(), false));
        infos.push(c.clone());
    }
    items_call(data, metas, &infos)?;
    let ret = items_return()?;
    require!(ret.len() == 1, ArmoryError::BadReturn);
    let extra_count = ret[0];
    // 4. set_slot_item.
    let returns = m.token_flags & token_flags::TRANSFER_RETURNS_DELTA != 0;
    let program = new.program;
    let ix = bordrless_token::client::set_slot_item(
        e.slot_authority.key(),
        mint_key,
        program,
        if returns { vault.map(|v| v.key()) } else { None },
        slot,
        item_key,
        m.token_flags,
        u16::from(m.pool_flags),
        extra_count,
    );
    let program_info = if program == ids::ITEMS_ID {
        items.clone()
    } else {
        e.template_program.ok_or(ArmoryError::WrongAccount)?.clone()
    };
    require_keys_eq!(program_info.key(), program, ArmoryError::WrongAccount);
    let mut infos = vec![
        e.slot_authority.clone(),
        e.mint.clone(),
        program_info,
        e.token.event_authority.clone(),
        e.token.token_program.clone(),
    ];
    if let Some(v) = vault {
        infos.push(v.clone());
    }
    token_invoke(&ix, &infos, &[authority_seeds])
}
