// Changed by Hookwars: new file, helpers for the armory and items programs (M2); arsenal waves B to E schemas; security review 1 and 2: propose and finalize accounts, TEST thresholds.
//! The armory in the LiteSVM suites: loads `hookwars_armory`, `hookwars_items`, and the test-only
//! `launch_stub` (at the launchpad's id: signs `["armory-caller", mint]`) and `war_stub` (at the
//! war program's id: signs `["loot-signer"]`); initializes the armory with [`TEST_PARAMS`] and
//! registers the nine templates of docs/spec/04-templates.md with [`test_schema`] ceilings.

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{InstructionData, ToAccountMetas};
use bordrless_hook::{equip_rule, slot_kind};
use bordrless_token::client as token;
use bordrless_token::instructions::SlotInit;
use hookwars_armory::cpi::{ARMORY_SIGNER, MINTER};
use hookwars_armory::state::{ArmoryConfig, ArmoryParams, Item};
use hookwars_armory::{LaunchEquip, RegisterTemplateArgs};
use hookwars_common::{ids, pda, template_id, EquipConfig, Params, PARAM_FIELDS};
use solana_keypair::Keypair;
use solana_signer::Signer;

use crate::env::Tx;
use crate::fixture::World;
use crate::program_bytes;
use crate::slots::item_slot;

/// Test values of every armory parameter (named TEST: none is a decision, 00 section 6).
pub const TEST_PARAMS: ArmoryParams = ArmoryParams {
    max_royalty_bps: 2_000,
    vote_period_secs: 3_600,
    vote_quorum_bps: 1_000,
    min_notice_secs: 60,
    max_notice_secs: 86_400,
    forge_gain_bps: 5_000,
    min_twap_secs: 60,
    max_pool_item_cut_bps: 300,
    max_pool_item_discount_bps: 10_000,
    max_item_reads: 4,
    admin_timelock_secs: 600,
    settle_bounty_bps: 50,
    // Security review 1, M-2: TEST threshold, 1% of the supply.
    proposal_min_bps: 100,
};

/// `p` padded with zeros.
pub fn params(v: &[u32]) -> Params {
    let mut a = [0u32; PARAM_FIELDS];
    a[..v.len()].copy_from_slice(v);
    a
}

/// TEST floors and ceilings per template: (field_min, field_max, max_targets, forge_enabled).
pub fn test_schema(id: u16) -> (Params, Params, u8, bool) {
    match id {
        template_id::RAID => (params(&[0, 0, 0]), params(&[5_000, 300, 1_000]), 3, true),
        template_id::SHIELD => (params(&[0, 60, 0]), params(&[300, 3_600, 1]), 3, true),
        template_id::WALL => (params(&[100]), params(&[10_000]), 0, true),
        template_id::SPY => (params(&[1, 60, 1, 0]), params(&[2, 3_600, 5_000, 300]), 1, true),
        template_id::TREATY => (params(&[0, 0, 0]), params(&[300, 300, 1]), 1, false),
        template_id::TRIBUTE => (params(&[0]), params(&[300]), 1, false),
        template_id::HALF_LIFE => (
            params(&[0, 3_600, 1]),
            params(&[200_000, 86_400, 16]),
            0,
            true,
        ),
        template_id::TRANSFER_FEE => (params(&[0, 100]), params(&[300, 10_000]), 1, true),
        template_id::WAR_ORDERS => (
            params(&[1, 0, 60, 100, 60, 60, 60, 0, 0, 0, 0]),
            params(&[1_000_000, 1_000, 86_400, 5_000, 86_400, 86_400, 86_400, 1_000, 1, 1_000, 100]),
            0,
            false,
        ),
        // Hookwars M3b: the arsenal's wave A and Composite (TEST ceilings).
        template_id::SIZE_TIERS => (
            params(&[0, 0, 0, 0, 0]),
            params(&[4_000_000_000, 4_000_000_000, 300, 300, 300]),
            0,
            true,
        ),
        template_id::SIDE_SKEW => (params(&[0, 0]), params(&[300, 300]), 0, true),
        template_id::LAUNCH_DECAY => (params(&[0, 0, 60]), params(&[300, 300, 86_400]), 0, true),
        template_id::MAX_TRANSACTION => (params(&[1]), params(&[10_000]), 0, true),
        template_id::DUST_GUARD => (params(&[0]), params(&[u32::MAX]), 0, true),
        template_id::SELL_BURN => (params(&[0]), params(&[300]), 0, true),
        // Hookwars arsenal waves B and C (TEST floors and ceilings).
        template_id::VELOCITY_FEE => (params(&[300, 0, 1, 0]), params(&[86_400, 1_000, 300, 300]), 0, true),
        template_id::IMPACT_FEE => (params(&[1, 0]), params(&[1_000, 300]), 0, true),
        template_id::VOLATILITY_FEE => (params(&[300, 300, 0, 0]), params(&[86_400, 86_400, 10_000, 300]), 0, true),
        template_id::RUSH_HOUR => (params(&[0, 1, 0, 0]), params(&[23, 24, 300, 300]), 0, true),
        template_id::COOLDOWN => (params(&[1]), params(&[86_400]), 0, true),
        template_id::DAILY_SELL_CAP => (params(&[1]), params(&[10_000]), 0, true),
        template_id::FLASH_GUARD => (params(&[1]), params(&[1_000]), 0, true),
        template_id::DUMP_BRAKE => (params(&[300, 300, 0, 0]), params(&[86_400, 86_400, 10_000, 300]), 0, true),
        template_id::STREAK => (params(&[1]), params(&[365]), 0, true),
        template_id::RANK_BADGE => (params(&[1, 1, 0]), params(&[u32::MAX, 1_000_000, 255]), 0, true),
        template_id::GUILD_TAG => (params(&[0]), params(&[1]), 0, true),
        template_id::COMPOSITE => (
            params(&[1, 1]),
            params(&[hookwars_common::MAX_MODULES as u32, 1]),
            6,
            false,
        ),
        // Arsenal waves D and E (TEST ceilings).
        id if hookwars_common::arsenal2::is(id) => crate::arsenal2::test_schema(id),
        _ => panic!("no template {id}"),
    }
}

/// The armory's event authority.
pub fn armory_events() -> Pubkey {
    Pubkey::find_program_address(&[b"__event_authority"], &ids::ARMORY_ID).0
}

/// Token accounts every armory path takes.
pub fn token_accounts() -> hookwars_armory::accounts::TokenAccounts {
    hookwars_armory::accounts::TokenAccounts {
        token_program: bordrless_token::ID,
        token_event_authority: token::event_authority(),
        system_program: anchor_lang::system_program::ID,
    }
}

/// An armory instruction.
pub fn armory_ix(accounts: impl ToAccountMetas, data: impl InstructionData) -> Instruction {
    Instruction {
        program_id: ids::ARMORY_ID,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

/// `ix` (an armory instruction whose `launch_caller` is `["armory-caller", mint]`) forwarded by
/// `launch_stub`, which signs as that PDA.
pub fn as_launch(mint: &Pubkey, ix: Instruction) -> Instruction {
    let caller = pda::armory_caller(mint).0;
    let mut accounts = vec![
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(ids::ARMORY_ID, false),
    ];
    accounts.extend(ix.accounts.into_iter().map(|mut m| {
        if m.pubkey == caller {
            m.is_signer = false;
        }
        m
    }));
    Instruction {
        program_id: ids::LAUNCH_ID,
        accounts,
        data: launch_stub::instruction::AsArmoryCaller { data: ix.data }.data(),
    }
}

/// `ix` (an armory instruction whose `loot_signer` is `["loot-signer"]`) forwarded by `war_stub`.
pub fn as_war(ix: Instruction) -> Instruction {
    let signer = pda::loot_signer().0;
    let mut accounts = vec![AccountMeta::new_readonly(ids::ARMORY_ID, false)];
    accounts.extend(ix.accounts.into_iter().map(|mut m| {
        if m.pubkey == signer {
            m.is_signer = false;
        }
        m
    }));
    Instruction {
        program_id: ids::WAR_ID,
        accounts,
        data: war_stub::instruction::AsLootSigner { data: ix.data }.data(),
    }
}

/// The test slot table: 0 War (vote), 1 Pool (vote, 12 data bytes, touch), 2 Fee (vote, cut up to
/// 300 bps), 3 Relation (performance).
pub fn test_slots() -> Vec<SlotInit> {
    vec![
        item_slot(slot_kind::WAR, equip_rule::VOTE, 0, 0, false),
        item_slot(slot_kind::POOL, equip_rule::VOTE, 0, 12, true),
        item_slot(slot_kind::FEE, equip_rule::VOTE, 300, 0, false),
        item_slot(slot_kind::RELATION, equip_rule::PERFORMANCE, 0, 0, false),
    ]
}

/// The armory world.
pub struct Hw {
    /// The world (DEX, bridge, launchpad configured; the launchpad's id then runs `launch_stub`).
    pub w: World,
    /// The armory's admin (the deployer).
    pub admin: Keypair,
}

impl Default for Hw {
    fn default() -> Self {
        Self::new()
    }
}

impl Hw {
    /// Loads the programs, initializes the armory and registers the nine templates.
    pub fn new() -> Self {
        let mut hw = Self::bare();
        hw.init().ok();
        for id in 1..=9u16 {
            hw.register(id).ok();
        }
        hw
    }

    /// Loads the programs only.
    pub fn bare() -> Self {
        let mut w = World::new();
        for (name, id) in [
            ("hookwars_armory", ids::ARMORY_ID),
            ("hookwars_items", ids::ITEMS_ID),
            ("launch_stub", ids::LAUNCH_ID),
            ("war_stub", ids::WAR_ID),
        ] {
            w.env
                .svm
                .add_program(id, &program_bytes(name))
                .unwrap_or_else(|e| panic!("load {name}: {e:?}"));
        }
        let deployer = w.env.deployer.pubkey();
        w.env.set_upgrade_authority(ids::ARMORY_ID, Some(deployer));
        w.env
            .set_upgrade_authority(ids::ITEMS_ID, Some(ids::PROTOCOL_AUTHORITY));
        let admin = w.env.deployer.insecure_clone();
        Self { w, admin }
    }

    /// `init` by the deployer with [`TEST_PARAMS`].
    pub fn init(&mut self) -> Tx {
        let ix = armory_ix(
            hookwars_armory::accounts::Init {
                authority: self.admin.pubkey(),
                config: pda::config().0,
                program_data: hookwars_common::programdata_address(&ids::ARMORY_ID),
                system_program: anchor_lang::system_program::ID,
            },
            hookwars_armory::instruction::Init {
                admin: self.admin.pubkey(),
                params: TEST_PARAMS,
            },
        );
        let admin = self.admin.insecure_clone();
        self.w.env.send_paid_by(&[ix], &admin, &[])
    }

    /// The config.
    pub fn config(&self) -> ArmoryConfig {
        self.w.env.read(&pda::config().0)
    }

    /// Registration args of template `id` with the TEST schema.
    pub fn template_args(id: u16) -> RegisterTemplateArgs {
        let s = hookwars_common::shape(id).expect("template");
        let (field_min, field_max, max_targets, forge) = test_schema(id);
        RegisterTemplateArgs {
            id,
            code_hash: [id as u8; 32],
            kind: s.kind,
            field_count: s.field_count,
            field_min,
            field_max,
            open_authoring: true,
            loot_enabled: true,
            forge_enabled: forge,
            max_level: 5,
            loot_royalty_bps: 500,
            max_targets,
            name: format!("Template {id}"),
        }
    }

    /// `register_template` of `args` with `program`, signed by `admin`.
    pub fn register_with(
        &mut self,
        admin: &Keypair,
        program: Pubkey,
        args: RegisterTemplateArgs,
    ) -> Tx {
        let ix = armory_ix(
            hookwars_armory::accounts::RegisterTemplate {
                admin: admin.pubkey(),
                config: pda::config().0,
                template: pda::template(args.id).0,
                template_program: program,
                programdata: hookwars_common::programdata_address(&program),
                system_program: anchor_lang::system_program::ID,
                event_authority: armory_events(),
                program: ids::ARMORY_ID,
            },
            hookwars_armory::instruction::RegisterTemplate { args },
        );
        self.w.env.send_paid_by(&[ix], admin, &[])
    }

    /// Registers template `id` from the items program with the TEST schema.
    pub fn register(&mut self, id: u16) -> Tx {
        let admin = self.admin.insecure_clone();
        self.register_with(&admin, ids::ITEMS_ID, Self::template_args(id))
    }

    /// `create_item` by `author`; returns the transaction, the item and its mint.
    pub fn create_item(
        &mut self,
        author: &Keypair,
        template_id: u16,
        p: Params,
        royalty_bps: u16,
    ) -> (Tx, Pubkey, Pubkey) {
        let n = self.config().items_minted;
        let item_mint = pda::item_mint(n).0;
        let item = pda::item(&item_mint).0;
        let ix = armory_ix(
            hookwars_armory::accounts::CreateItem {
                author: author.pubkey(),
                config: pda::config().0,
                template: pda::template(template_id).0,
                minter: MINTER,
                item_mint,
                item,
                recipient_holding: token::holding_address(&item_mint, &author.pubkey()),
                armory_signer: ARMORY_SIGNER,
                items_program: ids::ITEMS_ID,
                token: token_accounts(),
                system_program: anchor_lang::system_program::ID,
                event_authority: armory_events(),
                program: ids::ARMORY_ID,
            },
            hookwars_armory::instruction::CreateItem {
                template_id,
                params: p,
                royalty_bps,
            },
        );
        let tx = self.w.env.send_paid_by(&[ix], author, &[]);
        (tx, item, item_mint)
    }

    /// An item created by a fresh funded author; panics if creation fails.
    pub fn item(&mut self, template_id: u16, p: Params, royalty_bps: u16) -> (Keypair, Pubkey, Pubkey) {
        let author = self.w.env.funded(10_000_000_000);
        let (tx, item, mint) = self.create_item(&author, template_id, p, royalty_bps);
        tx.ok();
        (author, item, mint)
    }

    /// The `Item`.
    pub fn read_item(&self, item: &Pubkey) -> Item {
        self.w.env.read(item)
    }

    /// The `mint_loot` instruction (not forwarded).
    pub fn mint_loot_ix(&self, payer: &Pubkey, owner: &Pubkey, template_id: u16, p: Params) -> (Instruction, Pubkey) {
        let n = self.config().items_minted;
        let item_mint = pda::item_mint(n).0;
        let item = pda::item(&item_mint).0;
        let ix = armory_ix(
            hookwars_armory::accounts::MintLoot {
                loot_signer: pda::loot_signer().0,
                payer: *payer,
                owner: *owner,
                config: pda::config().0,
                template: pda::template(template_id).0,
                minter: MINTER,
                item_mint,
                item,
                recipient_holding: token::holding_address(&item_mint, owner),
                armory_signer: ARMORY_SIGNER,
                items_program: ids::ITEMS_ID,
                token: token_accounts(),
                system_program: anchor_lang::system_program::ID,
                event_authority: armory_events(),
                program: ids::ARMORY_ID,
            },
            hookwars_armory::instruction::MintLoot {
                template_id,
                params: p,
            },
        );
        (ix, item)
    }

    /// The equip accounts for moving `slot` of `mint` from `old` to `new`.
    pub fn equip_accounts(
        &self,
        payer: &Pubkey,
        mint: &Pubkey,
        slot: u8,
        old: Option<Pubkey>,
        new: Option<Pubkey>,
    ) -> hookwars_armory::accounts::EquipCtx {
        let equip_state = pda::equip_state(mint, slot).0;
        let vault = pda::holding(mint, &equip_state);
        let old_vault = old.and_then(|o| {
            let i: Item = self.read_item(&o);
            i.manifest.token_cuts().then_some(vault)
        });
        let mut a = hookwars_armory::accounts::EquipCtx {
            payer: *payer,
            token_mint: *mint,
            slot_authority: pda::slot_authority(mint).0,
            equip_state,
            old_item: old,
            old_equip_vault: old_vault,
            new_item: None,
            new_template: None,
            template_program: None,
            template_programdata: None,
            registry: None,
            new_equip_vault: None,
            royalty_owner: None,
            royalty_holding_token: None,
            quote_mint: None,
            royalty_holding_quote: None,
            new_composite: None,
            armory_signer: ARMORY_SIGNER,
            items_program: ids::ITEMS_ID,
            token: token_accounts(),
        };
        if let Some(n) = new {
            let i: Item = self.read_item(&n);
            let owner = pda::royalty_owner(&n).0;
            a.new_item = Some(n);
            a.new_template = Some(pda::template(i.template_id).0);
            a.template_program = Some(ids::ITEMS_ID);
            a.template_programdata = Some(hookwars_common::programdata_address(&ids::ITEMS_ID));
            a.registry = Some(
                Pubkey::find_program_address(
                    &[bordrless_hook::HOOK_ACCOUNTS_SEED, mint.as_ref(), n.as_ref()],
                    &ids::ITEMS_ID,
                )
                .0,
            );
            if i.manifest.token_cuts() {
                a.new_equip_vault = Some(vault);
            }
            if i.template_id == template_id::COMPOSITE {
                a.new_composite = Some(hookwars_common::composite::CompositeItem::address(&n).0);
            }
            if i.manifest.token_cuts() || i.manifest.pool_cuts() {
                a.royalty_owner = Some(owner);
                a.royalty_holding_token = Some(pda::holding(mint, &owner));
                a.quote_mint = Some(self.w.sol);
                a.royalty_holding_quote = Some(pda::holding(&self.w.sol, &owner));
            }
        }
        a
    }

    /// A fresh slot mint (test slot table, armory as slot authority) owned by `owner`, not yet
    /// minted.
    pub fn slot_mint(&mut self, owner: &Keypair, slots: Vec<SlotInit>) -> Pubkey {
        let mint = Keypair::new();
        self.w.create_slot_mint(owner, &mint, slots, true).ok();
        mint.pubkey()
    }

    /// `equip_launch` of one slot, forwarded by `launch_stub`.
    pub fn equip_launch(
        &mut self,
        payer: &Keypair,
        mint: &Pubkey,
        entry: LaunchEquip,
    ) -> Tx {
        let equip = self.equip_accounts(&payer.pubkey(), mint, entry.slot, None, entry.item);
        let ix = armory_ix(
            hookwars_armory::accounts::EquipLaunch {
                launch_caller: pda::armory_caller(mint).0,
                config: pda::config().0,
                slot_state: pda::slot_state(mint, entry.slot).0,
                equip,
                system_program: anchor_lang::system_program::ID,
                event_authority: armory_events(),
                program: ids::ARMORY_ID,
            },
            hookwars_armory::instruction::EquipLaunch { entry },
        );
        self.w.env.send_paid_by(&[as_launch(mint, ix)], payer, &[])
    }

    /// A launch entry.
    pub fn entry(slot: u8, item: Option<Pubkey>, config: EquipConfig) -> LaunchEquip {
        LaunchEquip {
            slot,
            item,
            config,
            notice_secs: 600,
            rule: None,
        }
    }

    /// `propose` by `proposer`; returns the transaction and the proposal.
    pub fn propose(
        &mut self,
        proposer: &Keypair,
        mint: &Pubkey,
        slot: u8,
        item: Option<Pubkey>,
        config: EquipConfig,
    ) -> (Tx, Pubkey) {
        let st: hookwars_armory::state::SlotState =
            self.w.env.read(&pda::slot_state(mint, slot).0);
        let proposal = pda::proposal(mint, slot, st.next_nonce).0;
        let template = item.map(|i| pda::template(self.read_item(&i).template_id).0);
        let ix = armory_ix(
            hookwars_armory::accounts::Propose {
                proposer: proposer.pubkey(),
                config: pda::config().0,
                token_mint: *mint,
                slot_state: pda::slot_state(mint, slot).0,
                proposal,
                item,
                template,
                template_program: item.map(|_| ids::ITEMS_ID),
                template_programdata: item
                    .map(|_| hookwars_common::programdata_address(&ids::ITEMS_ID)),
                proposer_holding: token::holding_address(mint, &proposer.pubkey()),
                slot_authority: pda::slot_authority(mint).0,
                token: token_accounts(),
                system_program: anchor_lang::system_program::ID,
                event_authority: armory_events(),
                program: ids::ARMORY_ID,
            },
            hookwars_armory::instruction::Propose {
                slot,
                item,
                equip_config: config,
            },
        );
        (self.w.env.send_paid_by(&[ix], proposer, &[]), proposal)
    }

    /// `vote`.
    pub fn vote(&mut self, voter: &Keypair, mint: &Pubkey, proposal: &Pubkey, support: bool, amount: u64) -> Tx {
        let ix = armory_ix(
            hookwars_armory::accounts::Vote {
                voter: voter.pubkey(),
                proposal: *proposal,
                vote_lock: pda::vote_lock(proposal, &voter.pubkey()).0,
                holding: token::holding_address(mint, &voter.pubkey()),
                token_mint: *mint,
                slot_authority: pda::slot_authority(mint).0,
                token: token_accounts(),
                system_program: anchor_lang::system_program::ID,
                event_authority: armory_events(),
                program: ids::ARMORY_ID,
            },
            hookwars_armory::instruction::Vote { support, amount },
        );
        self.w.env.send_paid_by(&[ix], voter, &[])
    }

    /// `finalize` (no launch: eligible is the whole supply; the launch address is always passed,
    /// security review 1, M-1).
    pub fn finalize(&mut self, proposal: &Pubkey) -> Tx {
        let p: hookwars_armory::state::Proposal = self.w.env.read(proposal);
        let ix = armory_ix(
            hookwars_armory::accounts::Finalize {
                config: pda::config().0,
                proposal: *proposal,
                slot_state: pda::slot_state(&p.mint, p.slot).0,
                token_mint: p.mint,
                launch: Some(pda::launch(&p.mint).0),
                pool_base_vault: None,
                launch_holding: None,
                event_authority: armory_events(),
                program: ids::ARMORY_ID,
            },
            hookwars_armory::instruction::Finalize {},
        );
        let payer = self.w.env.payer.insecure_clone();
        self.w.env.send(&[ix], &[&payer])
    }

    /// `execute` paid by `payer`.
    pub fn execute(&mut self, payer: &Keypair, proposal: &Pubkey) -> Tx {
        let ix = self.execute_ix(&payer.pubkey(), proposal);
        self.w.env.send_paid_by(&[ix], payer, &[])
    }

    /// The `execute` instruction.
    pub fn execute_ix(&self, payer: &Pubkey, proposal: &Pubkey) -> Instruction {
        let p: hookwars_armory::state::Proposal = self.w.env.read(proposal);
        let m: bordrless_token::state::Mint = self.w.env.read(&p.mint);
        let current = m.slots[usize::from(p.slot)].item;
        let old = (current != Pubkey::default()).then_some(current);
        let equip = self.equip_accounts(payer, &p.mint, p.slot, old, p.item);
        let ix = armory_ix(
            hookwars_armory::accounts::Execute {
                config: pda::config().0,
                proposal: *proposal,
                slot_state: pda::slot_state(&p.mint, p.slot).0,
                equip,
                event_authority: armory_events(),
                program: ids::ARMORY_ID,
            },
            hookwars_armory::instruction::Execute {},
        );
        let mut ix = ix;
        ix.accounts.extend(self.refresh_tail(&p.mint, p.slot));
        ix
    }

    /// Security review 2, L-D: the accounts an equip of slot `slot` appends for the pool registry
    /// refresh (`[launch program, launch, pool registry]`, no item registries: the stand-in
    /// launchpad of these suites makes no slot launches). Empty for other slot kinds.
    pub fn refresh_tail(&self, mint: &Pubkey, slot: u8) -> Vec<AccountMeta> {
        let m: bordrless_token::state::Mint = self.w.env.read(mint);
        let kind = m.slots[usize::from(slot)].kind;
        if kind != slot_kind::POOL && kind != slot_kind::RELATION {
            return vec![];
        }
        let launch = pda::launch(mint).0;
        let registry = self
            .w
            .env
            .account(&launch)
            .filter(|a| a.data.len() >= 106)
            .map(|a| {
                let pool = Pubkey::new_from_array(a.data[74..106].try_into().unwrap());
                bordrless_hook::hook_accounts_address(&ids::LAUNCH_ID, &pool).0
            })
            .unwrap_or_default();
        vec![
            AccountMeta::new_readonly(ids::LAUNCH_ID, false),
            AccountMeta::new_readonly(launch, false),
            AccountMeta::new(registry, false),
        ]
    }

    /// The `forge` instruction of items `a` and `b` by `forger` (who must hold both).
    pub fn forge_ix(&self, forger: &Pubkey, a: &Pubkey, b: &Pubkey) -> (Instruction, Pubkey) {
        let ia = self.read_item(a);
        let ib = self.read_item(b);
        let n = self.config().items_minted;
        let item_mint = pda::item_mint(n).0;
        let item = pda::item(&item_mint).0;
        let ix = armory_ix(
            hookwars_armory::accounts::Forge {
                forger: *forger,
                config: pda::config().0,
                forge_counter: pda::forge_counter(forger).0,
                item_a: *a,
                item_b: *b,
                mint_a: ia.item_mint,
                mint_b: ib.item_mint,
                holding_a: token::holding_address(&ia.item_mint, forger),
                holding_b: token::holding_address(&ib.item_mint, forger),
                template: pda::template(ia.template_id).0,
                minter: MINTER,
                item_mint,
                item,
                recipient_holding: token::holding_address(&item_mint, forger),
                armory_signer: ARMORY_SIGNER,
                items_program: ids::ITEMS_ID,
                token: token_accounts(),
                system_program: anchor_lang::system_program::ID,
                event_authority: armory_events(),
                program: ids::ARMORY_ID,
            },
            hookwars_armory::instruction::Forge {},
        );
        (ix, item)
    }

    /// Gives `to` the item token of `item_mint` held by `from`.
    pub fn give_item(&mut self, from: &Keypair, item_mint: &Pubkey, to: &Pubkey) {
        let ixs = [
            token::create_holding(from.pubkey(), *item_mint, *to),
            token::transfer(
                from.pubkey(),
                token::holding_address(item_mint, &from.pubkey()),
                token::holding_address(item_mint, to),
                *item_mint,
                None,
                vec![],
                1,
            ),
        ];
        self.w.env.send_paid_by(&ixs, from, &[]).ok();
    }

    /// Mints `amount` of slot mint `mint` (authority `owner`) to `to` (holding created).
    pub fn mint_to(&mut self, owner: &Keypair, mint: &Pubkey, to: &Pubkey, amount: u64) {
        let ixs = [
            token::create_holding(owner.pubkey(), *mint, *to),
            self.w.slot_mint_ix(&owner.pubkey(), mint, to, amount),
        ];
        self.w.env.send_paid_by(&ixs, owner, &[]).ok();
    }

    /// The item equipped in `slot` of `mint`.
    pub fn slot_item(&self, mint: &Pubkey, slot: u8) -> Pubkey {
        let m: bordrless_token::state::Mint = self.w.env.read(mint);
        m.slots[usize::from(slot)].item
    }
}

/// An armory error's code.
pub fn armory_code(e: hookwars_armory::error::ArmoryError) -> u32 {
    anchor_lang::error::ERROR_CODE_OFFSET + e as u32
}

/// An items error's code.
pub fn items_code(e: hookwars_items::ItemsError) -> u32 {
    anchor_lang::error::ERROR_CODE_OFFSET + e as u32
}
