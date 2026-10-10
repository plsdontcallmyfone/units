// Changed by Hookwars: new file, helpers for the hook economy (docs/spec/11-hook-economy.md); integration
// pass 3: Ew::wired (craft output the armory, armory and items as craft and social callers).
//! `hookwars_craft`, `hookwars_book`, market licences and social profiles in the LiteSVM suites:
//! loads the programs into an armory world, initializes them with TEST values (none is a
//! decision: 11 section 11), and builds their instructions. `econ_caller_stub` stands in for the
//! protocol programs that will call craft `drop`/`init_wear`/`wear` and social `record_wallet`
//! after integration, and for the armory's `mint_crafted`.

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{InstructionData, ToAccountMetas};
use bordrless_token::client as token;
use hookwars_book::state::{self as bs, BookParams, ClassKey};
use hookwars_common::economy::{self as eco, counter, skill, SkillDef, MAX_LEVELS};
use hookwars_common::{pda, PARAM_FIELDS};
use hookwars_craft::state::{self as cs, CraftParams, DropTerms, RecipeInput, RecipeTerms};
use hookwars_market::licence::{self as lic, LicenceParams};
use solana_keypair::Keypair;
use solana_signer::Signer;

use crate::armory::Hw;
use crate::env::Tx;
use crate::expansion::{events, load, social_ix};
use crate::program_bytes;

pub const SOL: u64 = 1_000_000_000;

/// TEST values of the craft parameters.
pub const TEST_CRAFT: CraftParams = CraftParams {
    recipe_protocol_bps: 1_000,
    max_inputs: 4,
    season_secs: 7 * 86_400,
    admin_timelock_secs: 600,
};

/// TEST values of the book parameters (few slots, so eviction is reachable).
pub const TEST_BOOK: BookParams = BookParams {
    taker_bps: 30,
    maker_bps: 10,
    slots: 4,
    match_max: 8,
    create_level: 0,
    order_bounty_lamports: 10_000,
    admin_timelock_secs: 600,
    tick_min_lamports: 1,
    tick_max_lamports: SOL,
    min_size_max: 1_000_000,
    skill_min_fee_lamports: 0,
};

/// TEST values of the licence parameters.
pub const TEST_LICENCE: LicenceParams = LicenceParams {
    protocol_bps: 500,
    author_bps: 1_000,
    min_secs: 3_600,
    max_secs: 30 * 86_400,
    skill_min_fee_lamports: 0,
};

/// TEST skill table: Crafter from crafts, Trader from fills, Builder from licences sold.
pub fn test_skills() -> Vec<SkillDef> {
    let t = |v: &[u64]| {
        let mut a = [0u64; MAX_LEVELS];
        a[..v.len()].copy_from_slice(v);
        a
    };
    vec![
        SkillDef { id: skill::CRAFTER, counter: counter::ITEMS_CRAFTED, thresholds: t(&[1, 3, 10]) },
        SkillDef { id: skill::TRADER, counter: counter::BOOK_FILLS, thresholds: t(&[1, 5, 20]) },
        SkillDef { id: skill::BUILDER, counter: counter::LICENCES_SOLD, thresholds: t(&[1, 5]) },
    ]
}

pub const STUB: Pubkey = econ_caller_stub::ID;

/// The stub's `[seed]` PDA.
pub fn stub_pda(seed: &[u8]) -> Pubkey {
    eco::caller_pda(seed, &STUB).0
}

/// Calls `inner` through the stub, signing as its `[seed]` PDA.
pub fn via_stub(seed: &[u8], inner: Instruction) -> Instruction {
    let pda = stub_pda(seed);
    let mut accounts = vec![AccountMeta::new_readonly(inner.program_id, false)];
    accounts.extend(inner.accounts.into_iter().map(|mut m| {
        if m.pubkey == pda {
            m.is_signer = false;
        }
        m
    }));
    Instruction {
        program_id: STUB,
        accounts,
        data: econ_caller_stub::instruction::Call {
            seed: seed.to_vec(),
            data: inner.data,
        }
        .data(),
    }
}

pub fn craft_ix(accounts: impl ToAccountMetas, data: impl InstructionData) -> Instruction {
    Instruction {
        program_id: hookwars_craft::ID,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

pub fn book_ix(accounts: impl ToAccountMetas, data: impl InstructionData) -> Instruction {
    Instruction {
        program_id: hookwars_book::ID,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

pub fn market_ix(accounts: impl ToAccountMetas, data: impl InstructionData) -> Instruction {
    crate::expansion::market_ix(accounts, data)
}

/// The social accounts craft, book and market take for `wallet` called from `program`.
pub struct SocialKeys {
    pub skills: Pubkey,
    pub profile: Pubkey,
    pub caller: Pubkey,
    pub events: Pubkey,
}

pub fn social_keys(wallet: &Pubkey, program: &Pubkey) -> SocialKeys {
    SocialKeys {
        skills: hookwars_social::skills_address().0,
        profile: hookwars_social::profile_address(wallet).0,
        caller: eco::caller_pda(eco::SOCIAL_CALLER_SEED, program).0,
        events: events(&hookwars_social::ID),
    }
}

/// An empty recipe of `kind` (fill in the rest).
pub fn recipe(kind: u8, inputs: &[(u16, u64)], fee: u64, template_id: u16) -> RecipeTerms {
    RecipeTerms {
        kind,
        inputs: inputs.iter().map(|&(material_id, amount)| RecipeInput { material_id, amount }).collect(),
        fee_lamports: fee,
        template_id,
        param_min: [0; PARAM_FIELDS],
        param_max: [0; PARAM_FIELDS],
        charges_restored: if kind == cs::recipe_kind::REPAIR { 5 } else { 0 },
        min_level: 0,
        active: true,
    }
}

/// The economy world: the armory world plus market, social, craft, book and the stub.
pub struct Ew {
    pub hw: Hw,
    pub craft_treasury: Pubkey,
    pub season_pool: Pubkey,
    pub book_treasury: Pubkey,
    pub market_treasury: Pubkey,
}

impl Default for Ew {
    fn default() -> Self {
        Self::new()
    }
}

impl Ew {
    pub fn new() -> Self {
        Self::with(Hw::new(), STUB, vec![STUB], vec![])
    }

    /// Integration pass 3: the economy world wired to the real programs: the arsenal registered,
    /// craft's output program the armory, the armory and items craft callers (with the stub), and
    /// the armory and market social callers.
    pub fn wired() -> Self {
        Self::with(
            crate::items::arsenal(),
            hookwars_common::ids::ARMORY_ID,
            vec![STUB, hookwars_common::ids::ARMORY_ID, hookwars_common::ids::ITEMS_ID],
            vec![hookwars_common::ids::ARMORY_ID],
        )
    }

    pub fn with(hw: Hw, output_program: Pubkey, craft_callers: Vec<Pubkey>, more_social: Vec<Pubkey>) -> Self {
        let mut hw = hw;
        let market_treasury = hw.w.env.funded(SOL).pubkey();
        load(&mut hw.w.env, market_treasury);
        for (name, id) in [
            ("hookwars_craft", hookwars_craft::ID),
            ("hookwars_book", hookwars_book::ID),
            ("econ_caller_stub", STUB),
        ] {
            hw.w.env
                .svm
                .add_program(id, &program_bytes(name))
                .unwrap_or_else(|e| panic!("load {name}: {e:?}"));
            let d = hw.w.env.deployer.pubkey();
            hw.w.env.set_upgrade_authority(id, Some(d));
        }
        let craft_treasury = hw.w.env.funded(SOL).pubkey();
        let season_pool = hw.w.env.funded(SOL).pubkey();
        let book_treasury = hw.w.env.funded(SOL).pubkey();
        let mut ew = Self {
            hw,
            craft_treasury,
            season_pool,
            book_treasury,
            market_treasury,
        };
        let d = ew.deployer();
        let ixs = [
            craft_ix(
                hookwars_craft::accounts::Init {
                    authority: d.pubkey(),
                    config: cs::config_address().0,
                    program_data: hookwars_common::programdata_address(&hookwars_craft::ID),
                    system_program: anchor_lang::system_program::ID,
                },
                hookwars_craft::instruction::Init {
                    admin: d.pubkey(),
                    treasury: craft_treasury,
                    season_pool,
                    output_program,
                    callers: craft_callers,
                    params: TEST_CRAFT,
                },
            ),
            book_ix(
                hookwars_book::accounts::Init {
                    authority: d.pubkey(),
                    config: bs::config_address().0,
                    program_data: hookwars_common::programdata_address(&hookwars_book::ID),
                    system_program: anchor_lang::system_program::ID,
                },
                hookwars_book::instruction::Init {
                    admin: d.pubkey(),
                    treasury: book_treasury,
                    params: TEST_BOOK,
                },
            ),
            social_ix(
                hookwars_social::accounts::InitSkills {
                    admin: d.pubkey(),
                    config: crate::expansion::social_config(),
                    skills: hookwars_social::skills_address().0,
                    system_program: anchor_lang::system_program::ID,
                },
                hookwars_social::instruction::InitSkills {
                    skills: test_skills(),
                    callers: [vec![hookwars_craft::ID, hookwars_book::ID, hookwars_market::ID, STUB], more_social].concat(),
                },
            ),
            market_ix(
                hookwars_market::accounts::InitLicenceConfig {
                    admin: d.pubkey(),
                    config: hookwars_market::state::config_address().0,
                    licence_config: lic::licence_config_address().0,
                    system_program: anchor_lang::system_program::ID,
                },
                hookwars_market::instruction::InitLicenceConfig { params: TEST_LICENCE },
            ),
        ];
        ew.hw.w.env.send(&ixs, &[&d]).ok();
        ew
    }

    pub fn deployer(&self) -> Keypair {
        self.hw.w.env.deployer.insecure_clone()
    }

    pub fn send(&mut self, payer: &Keypair, ixs: &[Instruction]) -> Tx {
        self.hw.w.env.send_paid_by(ixs, payer, &[])
    }

    pub fn funded(&mut self, lamports: u64) -> Keypair {
        self.hw.w.env.funded(lamports)
    }

    pub fn lamports(&self, k: &Pubkey) -> u64 {
        self.hw.w.env.lamports(k)
    }

    pub fn holding(&self, mint: &Pubkey, owner: &Pubkey) -> u64 {
        self.hw.w.env.holding(mint, owner)
    }

    pub fn warp(&mut self, secs: i64) {
        self.hw.w.env.warp(secs);
    }

    pub fn now(&self) -> i64 {
        self.hw.w.env.now
    }

    // ---- profiles

    pub fn open_profile_ix(&self, payer: &Pubkey, wallet: &Pubkey) -> Instruction {
        social_ix(
            hookwars_social::accounts::OpenProfile {
                payer: *payer,
                profile: hookwars_social::profile_address(wallet).0,
                system_program: anchor_lang::system_program::ID,
                event_authority: events(&hookwars_social::ID),
                program: hookwars_social::ID,
            },
            hookwars_social::instruction::OpenProfile { wallet: *wallet },
        )
    }

    pub fn open_profile(&mut self, wallet: &Keypair) {
        let ix = self.open_profile_ix(&wallet.pubkey(), &wallet.pubkey());
        self.send(wallet, &[ix]).ok();
    }

    /// `record_wallet` as `caller_program`'s `["social-caller"]` (signed by `caller`).
    pub fn record_wallet_ix(&self, caller: &Pubkey, caller_program: &Pubkey, wallet: &Pubkey, counter: u8, value: u64) -> Instruction {
        social_ix(
            hookwars_social::accounts::RecordWallet {
                caller: *caller,
                skills: hookwars_social::skills_address().0,
                profile: hookwars_social::profile_address(wallet).0,
                event_authority: events(&hookwars_social::ID),
                program: hookwars_social::ID,
            },
            hookwars_social::instruction::RecordWallet {
                caller_program: *caller_program,
                counter,
                value,
            },
        )
    }

    /// Bumps `wallet`'s counter through the stub (a registered caller).
    pub fn stub_record(&mut self, wallet: &Pubkey, counter: u8, value: u64) -> Tx {
        let caller = stub_pda(eco::SOCIAL_CALLER_SEED);
        let ix = via_stub(eco::SOCIAL_CALLER_SEED, self.record_wallet_ix(&caller, &STUB, wallet, counter, value));
        let payer = self.funded(SOL);
        self.send(&payer, &[ix])
    }

    pub fn profile(&self, wallet: &Pubkey) -> hookwars_social::Profile {
        self.hw.w.env.read(&hookwars_social::profile_address(wallet).0)
    }

    // ---- materials and drops

    pub fn create_material_ix(&self, id: u16, name: &str, cap: u64) -> Instruction {
        let d = self.hw.w.env.deployer.pubkey();
        craft_ix(
            hookwars_craft::accounts::CreateMaterial {
                admin: d,
                config: cs::config_address().0,
                material: cs::material_address(id).0,
                material_mint: cs::material_mint_address(id).0,
                token_program: bordrless_token::ID,
                token_event_authority: token::event_authority(),
                system_program: anchor_lang::system_program::ID,
                event_authority: events(&hookwars_craft::ID),
                program: hookwars_craft::ID,
            },
            hookwars_craft::instruction::CreateMaterial {
                id,
                name: name.to_string(),
                symbol: "MAT".to_string(),
                emission_cap_per_season: cap,
            },
        )
    }

    pub fn create_material(&mut self, id: u16, cap: u64) -> Pubkey {
        let d = self.deployer();
        let ix = self.create_material_ix(id, &format!("Material {id}"), cap);
        self.send(&d, &[ix]).ok();
        cs::material_mint_address(id).0
    }

    pub fn set_drop_rule_ix(&self, source: u8, terms: DropTerms) -> Instruction {
        let d = self.hw.w.env.deployer.pubkey();
        craft_ix(
            hookwars_craft::accounts::SetDropRule {
                admin: d,
                config: cs::config_address().0,
                drop_rule: cs::drop_address(source).0,
                system_program: anchor_lang::system_program::ID,
                event_authority: events(&hookwars_craft::ID),
                program: hookwars_craft::ID,
            },
            hookwars_craft::instruction::SetDropRule { source, terms },
        )
    }

    /// A drop rule in effect: set, then the timelock passes.
    pub fn drop_rule(&mut self, source: u8, material_id: u16, num: u64, den: u64) {
        let d = self.deployer();
        let ix = self.set_drop_rule_ix(source, DropTerms { material_id, per_unit_num: num, per_unit_den: den });
        self.send(&d, &[ix]).ok();
        self.warp(i64::from(TEST_CRAFT.admin_timelock_secs));
    }

    /// `drop` signed by `caller` claiming `caller_program`.
    #[allow(clippy::too_many_arguments)]
    pub fn drop_ix(&self, caller: &Pubkey, caller_program: &Pubkey, payer: &Pubkey, source: u8, material_id: u16, measured: u64, recipient: &Pubkey) -> Instruction {
        let mint = cs::material_mint_address(material_id).0;
        craft_ix(
            hookwars_craft::accounts::Drop {
                caller: *caller,
                payer: *payer,
                config: cs::config_address().0,
                drop_rule: cs::drop_address(source).0,
                material: cs::material_address(material_id).0,
                material_mint: mint,
                minter: cs::minter_address().0,
                recipient: *recipient,
                recipient_holding: token::holding_address(&mint, recipient),
                token_program: bordrless_token::ID,
                token_event_authority: token::event_authority(),
                system_program: anchor_lang::system_program::ID,
                event_authority: events(&hookwars_craft::ID),
                program: hookwars_craft::ID,
            },
            hookwars_craft::instruction::Drop {
                caller_program: *caller_program,
                source,
                measured,
            },
        )
    }

    /// A drop through the stub (a registered caller).
    pub fn stub_drop(&mut self, source: u8, material_id: u16, measured: u64, recipient: &Pubkey) -> Tx {
        let payer = self.funded(SOL);
        let caller = stub_pda(eco::CRAFT_CALLER_SEED);
        let ix = via_stub(eco::CRAFT_CALLER_SEED, self.drop_ix(&caller, &STUB, &payer.pubkey(), source, material_id, measured, recipient));
        self.send(&payer, &[ix])
    }

    pub fn material(&self, id: u16) -> cs::Material {
        self.hw.w.env.read(&cs::material_address(id).0)
    }

    pub fn craft_config(&self) -> cs::CraftConfig {
        self.hw.w.env.read(&cs::config_address().0)
    }

    // ---- recipes

    pub fn create_recipe(&mut self, id: u16, terms: RecipeTerms) -> Tx {
        let d = self.deployer();
        let ix = craft_ix(
            hookwars_craft::accounts::CreateRecipe {
                admin: d.pubkey(),
                config: cs::config_address().0,
                recipe: cs::recipe_address(id).0,
                system_program: anchor_lang::system_program::ID,
                event_authority: events(&hookwars_craft::ID),
                program: hookwars_craft::ID,
            },
            hookwars_craft::instruction::CreateRecipe { id, terms },
        );
        self.send(&d, &[ix])
    }

    pub fn set_recipe(&mut self, id: u16, terms: RecipeTerms) -> Tx {
        let d = self.deployer();
        let ix = craft_ix(
            hookwars_craft::accounts::SetRecipe {
                admin: d.pubkey(),
                config: cs::config_address().0,
                recipe: cs::recipe_address(id).0,
                event_authority: events(&hookwars_craft::ID),
                program: hookwars_craft::ID,
            },
            hookwars_craft::instruction::SetRecipe { terms },
        );
        self.send(&d, &[ix])
    }

    /// `(material, mint, holding)` per input of `inputs` for `owner`.
    pub fn input_metas(inputs: &[(u16, u64)], owner: &Pubkey) -> Vec<AccountMeta> {
        let mut v = Vec::new();
        for &(id, _) in inputs {
            let mint = cs::material_mint_address(id).0;
            v.push(AccountMeta::new(cs::material_address(id).0, false));
            v.push(AccountMeta::new(mint, false));
            v.push(AccountMeta::new(token::holding_address(&mint, owner), false));
        }
        v
    }

    pub fn craft_item_ix(&self, crafter: &Pubkey, recipe_id: u16, inputs: &[(u16, u64)]) -> Instruction {
        let c = self.craft_config();
        let s = social_keys(crafter, &hookwars_craft::ID);
        let mut ix = craft_ix(
            hookwars_craft::accounts::CraftItem {
                crafter: *crafter,
                config: cs::config_address().0,
                recipe: cs::recipe_address(recipe_id).0,
                treasury: c.treasury,
                season_pool: c.season_pool,
                craft_signer: cs::signer_address().0,
                output_program: c.output_program,
                skills: s.skills,
                profile: s.profile,
                social_caller: s.caller,
                social_event_authority: s.events,
                social_program: hookwars_social::ID,
                token_program: bordrless_token::ID,
                token_event_authority: token::event_authority(),
                system_program: anchor_lang::system_program::ID,
                event_authority: events(&hookwars_craft::ID),
                program: hookwars_craft::ID,
            },
            hookwars_craft::instruction::Craft {
                recipe_id,
                reference: [7; 32],
            },
        );
        ix.accounts.extend(Self::input_metas(inputs, crafter));
        ix
    }

    #[allow(clippy::too_many_arguments)]
    pub fn repair_ix(&self, holder: &Pubkey, recipe_id: u16, inputs: &[(u16, u64)], item: &Pubkey, item_mint: &Pubkey) -> Instruction {
        let c = self.craft_config();
        let s = social_keys(holder, &hookwars_craft::ID);
        let mut ix = craft_ix(
            hookwars_craft::accounts::RepairItem {
                holder: *holder,
                config: cs::config_address().0,
                recipe: cs::recipe_address(recipe_id).0,
                item: *item,
                item_mint: *item_mint,
                holder_item_holding: token::holding_address(item_mint, holder),
                wear: cs::wear_address(item).0,
                treasury: c.treasury,
                season_pool: c.season_pool,
                skills: s.skills,
                profile: s.profile,
                social_caller: s.caller,
                social_event_authority: s.events,
                social_program: hookwars_social::ID,
                token_program: bordrless_token::ID,
                token_event_authority: token::event_authority(),
                system_program: anchor_lang::system_program::ID,
                event_authority: events(&hookwars_craft::ID),
                program: hookwars_craft::ID,
            },
            hookwars_craft::instruction::Repair {
                recipe_id,
                reference: [8; 32],
            },
        );
        ix.accounts.extend(Self::input_metas(inputs, holder));
        ix
    }

    // ---- wear

    pub fn init_wear_ix(&self, caller: &Pubkey, caller_program: &Pubkey, payer: &Pubkey, item: &Pubkey, max_charges: u32) -> Instruction {
        craft_ix(
            hookwars_craft::accounts::InitWear {
                caller: *caller,
                payer: *payer,
                config: cs::config_address().0,
                wear: cs::wear_address(item).0,
                system_program: anchor_lang::system_program::ID,
                event_authority: events(&hookwars_craft::ID),
                program: hookwars_craft::ID,
            },
            hookwars_craft::instruction::InitWear {
                caller_program: *caller_program,
                item: *item,
                max_charges,
            },
        )
    }

    pub fn stub_init_wear(&mut self, item: &Pubkey, max_charges: u32) -> Tx {
        let payer = self.funded(SOL);
        let caller = stub_pda(eco::CRAFT_CALLER_SEED);
        let ix = via_stub(eco::CRAFT_CALLER_SEED, self.init_wear_ix(&caller, &STUB, &payer.pubkey(), item, max_charges));
        self.send(&payer, &[ix])
    }

    pub fn wear_ix(&self, caller: &Pubkey, caller_program: &Pubkey, item: &Pubkey, runs: u32) -> Instruction {
        craft_ix(
            hookwars_craft::accounts::WearItem {
                caller: *caller,
                config: cs::config_address().0,
                wear: cs::wear_address(item).0,
                event_authority: events(&hookwars_craft::ID),
                program: hookwars_craft::ID,
            },
            hookwars_craft::instruction::Wear {
                caller_program: *caller_program,
                runs,
            },
        )
    }

    pub fn stub_wear(&mut self, item: &Pubkey, runs: u32) -> Tx {
        let payer = self.funded(SOL);
        let caller = stub_pda(eco::CRAFT_CALLER_SEED);
        let ix = via_stub(eco::CRAFT_CALLER_SEED, self.wear_ix(&caller, &STUB, item, runs));
        self.send(&payer, &[ix])
    }

    pub fn wear(&self, item: &Pubkey) -> cs::Wear {
        self.hw.w.env.read(&cs::wear_address(item).0)
    }

    // ---- book

    pub fn create_market_ix(&self, creator: &Pubkey, material_id: u16, tick: u64, min_size: u64) -> Instruction {
        let base = cs::material_mint_address(material_id).0;
        let market = bs::market_address(&base).0;
        let escrow = bs::escrow_address(&market).0;
        let s = social_keys(creator, &hookwars_book::ID);
        book_ix(
            hookwars_book::accounts::CreateMarket {
                creator: *creator,
                config: bs::config_address().0,
                material: cs::material_address(material_id).0,
                base_mint: base,
                market,
                escrow,
                escrow_holding: token::holding_address(&base, &escrow),
                skills: s.skills,
                profile: s.profile,
                token_program: bordrless_token::ID,
                token_event_authority: token::event_authority(),
                system_program: anchor_lang::system_program::ID,
                event_authority: events(&hookwars_book::ID),
                program: hookwars_book::ID,
            },
            hookwars_book::instruction::CreateMarket {
                tick_lamports: tick,
                min_size,
            },
        )
    }

    /// `place` with `makers` as the `(wallet, holding)` pairs in the order they will be met.
    #[allow(clippy::too_many_arguments)]
    pub fn place_ix(&self, owner: &Pubkey, base: &Pubkey, side: u8, price: u64, size: u64, post_only: bool, expires_at: i64, makers: &[Pubkey]) -> Instruction {
        let market = bs::market_address(base).0;
        let escrow = bs::escrow_address(&market).0;
        let s = social_keys(owner, &hookwars_book::ID);
        let config: bs::BookConfig = self.hw.w.env.read(&bs::config_address().0);
        let mut ix = book_ix(
            hookwars_book::accounts::Place {
                owner: *owner,
                config: bs::config_address().0,
                market,
                escrow,
                base_mint: *base,
                escrow_holding: token::holding_address(base, &escrow),
                owner_holding: token::holding_address(base, owner),
                treasury: config.treasury,
                skills: s.skills,
                profile: s.profile,
                social_caller: s.caller,
                social_event_authority: s.events,
                social_program: hookwars_social::ID,
                token_program: bordrless_token::ID,
                token_event_authority: token::event_authority(),
                system_program: anchor_lang::system_program::ID,
                event_authority: events(&hookwars_book::ID),
                program: hookwars_book::ID,
            },
            hookwars_book::instruction::Place {
                side_: side,
                price,
                size,
                post_only,
                expires_at,
                reference: [9; 32],
            },
        );
        ix.accounts.extend(pairs(base, makers));
        ix
    }

    pub fn cancel_ix(&self, owner: &Pubkey, base: &Pubkey, id: u64) -> Instruction {
        let market = bs::market_address(base).0;
        let escrow = bs::escrow_address(&market).0;
        book_ix(
            hookwars_book::accounts::Cancel {
                owner: *owner,
                market,
                escrow,
                base_mint: *base,
                escrow_holding: token::holding_address(base, &escrow),
                owner_holding: token::holding_address(base, owner),
                token_program: bordrless_token::ID,
                token_event_authority: token::event_authority(),
                system_program: anchor_lang::system_program::ID,
                event_authority: events(&hookwars_book::ID),
                program: hookwars_book::ID,
            },
            hookwars_book::instruction::Cancel { id },
        )
    }

    pub fn crank_ix(&self, cranker: &Pubkey, base: &Pubkey, max: u8, owners: &[Pubkey]) -> Instruction {
        let market = bs::market_address(base).0;
        let escrow = bs::escrow_address(&market).0;
        let mut ix = book_ix(
            hookwars_book::accounts::Crank {
                cranker: *cranker,
                market,
                escrow,
                base_mint: *base,
                escrow_holding: token::holding_address(base, &escrow),
                token_program: bordrless_token::ID,
                token_event_authority: token::event_authority(),
                system_program: anchor_lang::system_program::ID,
            },
            hookwars_book::instruction::Crank { max },
        );
        ix.accounts.extend(pairs(base, owners));
        ix
    }

    pub fn book(&self, base: &Pubkey) -> bs::BookMarket {
        self.hw.w.env.read(&bs::market_address(base).0)
    }

    /// Escrow conservation (11 section 6.6): the escrow's lamports above rent equal every resting
    /// bid's lock plus every bounty, and its base equals every ask's size.
    pub fn assert_escrow(&self, base: &Pubkey) {
        let m = self.book(base);
        let market = bs::market_address(base).0;
        let escrow = bs::escrow_address(&market).0;
        let acc = self.hw.w.env.account(&escrow).expect("escrow");
        let rent = self.hw.w.env.rent(acc.data.len());
        assert_eq!(u128::from(acc.lamports - rent), m.lamports_owed(), "escrow lamports");
        assert_eq!(u128::from(self.holding(base, &escrow)), m.base_owed(), "escrow base");
    }

    pub fn place_class_bid_ix(&self, bidder: &Pubkey, nonce: u64, class: ClassKey, price: u64, expires_at: i64) -> Instruction {
        book_ix(
            hookwars_book::accounts::PlaceClassBid {
                bidder: *bidder,
                config: bs::config_address().0,
                bid: bs::class_bid_address(bidder, nonce).0,
                system_program: anchor_lang::system_program::ID,
                event_authority: events(&hookwars_book::ID),
                program: hookwars_book::ID,
            },
            hookwars_book::instruction::PlaceClassBid {
                nonce,
                class,
                price,
                expires_at,
            },
        )
    }

    pub fn cancel_class_bid_ix(&self, bidder: &Pubkey, nonce: u64) -> Instruction {
        book_ix(
            hookwars_book::accounts::CancelClassBid {
                bidder: *bidder,
                bid: bs::class_bid_address(bidder, nonce).0,
                event_authority: events(&hookwars_book::ID),
                program: hookwars_book::ID,
            },
            hookwars_book::instruction::CancelClassBid {},
        )
    }

    pub fn match_class_ix(&self, seller: &Pubkey, bidder: &Pubkey, nonce: u64, item: &Pubkey, item_mint: &Pubkey) -> Instruction {
        let config: bs::BookConfig = self.hw.w.env.read(&bs::config_address().0);
        book_ix(
            hookwars_book::accounts::MatchClass {
                seller: *seller,
                config: bs::config_address().0,
                bid: bs::class_bid_address(bidder, nonce).0,
                bidder: *bidder,
                item: *item,
                item_mint: *item_mint,
                seller_holding: token::holding_address(item_mint, seller),
                bidder_holding: token::holding_address(item_mint, bidder),
                listing: hookwars_market::state::Listing::address(item_mint).0,
                lease: hookwars_market::state::lease_address(item).0,
                treasury: config.treasury,
                token_program: bordrless_token::ID,
                token_event_authority: token::event_authority(),
                system_program: anchor_lang::system_program::ID,
                event_authority: events(&hookwars_book::ID),
                program: hookwars_book::ID,
            },
            hookwars_book::instruction::MatchClass { reference: [3; 32] },
        )
    }

    // ---- licences

    #[allow(clippy::too_many_arguments)]
    pub fn set_offer_ix(&self, holder: &Pubkey, item: &Pubkey, item_mint: &Pubkey, price: u64, term: u32, per: u8, max_live: u16, exclusive: bool, active: bool) -> Instruction {
        market_ix(
            hookwars_market::accounts::SetLicenceOffer {
                holder: *holder,
                licence_config: lic::licence_config_address().0,
                item: *item,
                item_mint: *item_mint,
                holder_holding: token::holding_address(item_mint, holder),
                offer: lic::licence_offer_address(item).0,
                system_program: anchor_lang::system_program::ID,
                event_authority: events(&hookwars_market::ID),
                program: hookwars_market::ID,
            },
            hookwars_market::instruction::SetLicenceOffer {
                price_lamports: price,
                term_secs: term,
                per,
                max_live,
                exclusive,
                active,
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn buy_license_ix(&self, payer: &Pubkey, item: &Pubkey, item_mint: &Pubkey, token_mint: &Pubkey, holder: &Pubkey, author: &Pubkey, max_price: u64, renew: bool) -> Instruction {
        let it: hookwars_armory::state::Item = self.hw.w.env.read(item);
        let s = social_keys(holder, &hookwars_market::ID);
        let accounts = hookwars_market::accounts::BuyLicense {
            payer: *payer,
            config: hookwars_market::state::config_address().0,
            licence_config: lic::licence_config_address().0,
            offer: lic::licence_offer_address(item).0,
            license: lic::license_address(item, token_mint).0,
            item: *item,
            item_mint: *item_mint,
            template: pda::template(it.template_id).0,
            token_mint: *token_mint,
            holder: *holder,
            holder_holding: token::holding_address(item_mint, holder),
            author: *author,
            treasury: self.market_treasury,
            skills: s.skills,
            holder_profile: s.profile,
            social_caller: s.caller,
            social_event_authority: s.events,
            social_program: hookwars_social::ID,
            system_program: anchor_lang::system_program::ID,
            event_authority: events(&hookwars_market::ID),
            program: hookwars_market::ID,
        };
        if renew {
            market_ix(accounts, hookwars_market::instruction::RenewLicense { max_price, reference: [5; 32] })
        } else {
            market_ix(accounts, hookwars_market::instruction::BuyLicense { max_price, reference: [5; 32] })
        }
    }

    pub fn revoke_license_ix(&self, holder: &Pubkey, item: &Pubkey, item_mint: &Pubkey, token_mint: &Pubkey, payer: &Pubkey) -> Instruction {
        market_ix(
            hookwars_market::accounts::RevokeLicense {
                holder: *holder,
                item_mint: *item_mint,
                holder_holding: token::holding_address(item_mint, holder),
                offer: lic::licence_offer_address(item).0,
                license: lic::license_address(item, token_mint).0,
                payer: *payer,
                system_program: anchor_lang::system_program::ID,
                event_authority: events(&hookwars_market::ID),
                program: hookwars_market::ID,
            },
            hookwars_market::instruction::RevokeLicense {},
        )
    }

    pub fn expire_license_ix(&self, item: &Pubkey, token_mint: &Pubkey) -> Instruction {
        market_ix(
            hookwars_market::accounts::ExpireLicense {
                offer: lic::licence_offer_address(item).0,
                license: lic::license_address(item, token_mint).0,
                event_authority: events(&hookwars_market::ID),
                program: hookwars_market::ID,
            },
            hookwars_market::instruction::ExpireLicense {},
        )
    }

    /// A template's `registered_by`.
    pub fn template_author(&self, item: &Pubkey) -> Pubkey {
        let it: hookwars_armory::state::Item = self.hw.w.env.read(item);
        let t: hookwars_armory::state::Template = self.hw.w.env.read(&pda::template(it.template_id).0);
        t.registered_by
    }
}

/// `(wallet, base holding)` pairs.
pub fn pairs(base: &Pubkey, owners: &[Pubkey]) -> Vec<AccountMeta> {
    let mut v = Vec::new();
    for o in owners {
        v.push(AccountMeta::new(*o, false));
        v.push(AccountMeta::new(token::holding_address(base, o), false));
    }
    v
}

/// Anchor's custom error code of a craft error.
pub fn craft_code(e: hookwars_craft::error::CraftError) -> u32 {
    anchor_lang::error::ERROR_CODE_OFFSET + e as u32
}

/// Anchor's custom error code of a book error.
pub fn book_code(e: hookwars_book::error::BookError) -> u32 {
    anchor_lang::error::ERROR_CODE_OFFSET + e as u32
}

/// Whether a log line of the transaction contains `needle`.
pub fn logged(tx: &Tx, needle: &str) -> bool {
    tx.logs().iter().any(|l| l.contains(needle))
}
