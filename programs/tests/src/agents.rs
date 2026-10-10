// Changed by Hookwars: new file, helpers for hookwars_agents (09).
//! The agents program in the LiteSVM suites, on top of the armory world ([`Hw`]): it loads
//! `hookwars_agents` and the test-only `agents_caller_stub` (at the war program's id, signing
//! `["agents-caller"]`), registers template 42 Soulbound, creates the one Soulbound item and
//! initializes the config with [`TEST_AGENTS_PARAMS`].
//!
//! Badges are equipped through `launch_stub` (the armory's launch caller) until the armory accepts
//! the agents caller for badges (09 "Integration requests" 1).

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{InstructionData, ToAccountMetas};
use bordrless_token::client as token;
use hookwars_agents::constants::{link_statement, pda as apda, report_data, ED25519_PROGRAM_ID, INSTRUCTIONS_SYSVAR_ID};
use hookwars_agents::state::{AgentsParams, Passport, PolicyLimits};
use hookwars_agents::{AttestArgs, ConfigArgs, ProfileArgs};
use hookwars_common::{ids, template_id, EquipConfig, PARAM_FIELDS};
use solana_keypair::Keypair;
use solana_signer::Signer;

use crate::armory::Hw;
use crate::env::Tx;
use crate::program_bytes;

/// TEST values of every agents parameter (named TEST: none is a decision, 09 section 13).
pub const TEST_AGENTS_PARAMS: AgentsParams = AgentsParams {
    name_max_len: 32,
    uri_max_len: 200,
    handle_max_len: 32,
    passport_fee_lamports: 10_000_000,
    max_passports_per_operator: 2,
    attest_quorum: 2,
    attest_max_ttl_secs: 30 * 86_400,
    policy_day_secs: 86_400,
    bond_lamports: 1_000_000_000,
    bond_min_passport_age_secs: 600,
    bond_cancel_grace_secs: 3_600,
    treaty_hold_secs: 7 * 86_400,
    admin_timelock_secs: 600,
};

/// The agents program's event authority.
pub fn agents_events() -> Pubkey {
    Pubkey::find_program_address(&[b"__event_authority"], &ids::AGENTS_ID).0
}

/// An agents instruction.
pub fn agents_ix(accounts: impl ToAccountMetas, data: impl InstructionData) -> Instruction {
    Instruction {
        program_id: ids::AGENTS_ID,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

/// The token accounts every agents token CPI takes.
pub fn token_accs() -> hookwars_agents::accounts::TokenAccs {
    hookwars_agents::accounts::TokenAccs {
        token_program: bordrless_token::ID,
        token_event_authority: token::event_authority(),
    }
}

/// A profile.
pub fn profile(name: &str, kinds: u8) -> ProfileArgs {
    ProfileArgs {
        name: name.to_string(),
        avatar_uri: format!("https://example.invalid/{name}.png"),
        bio_uri: String::new(),
        hire_uri: String::new(),
        kinds,
        credit_agent_id: None,
    }
}

/// An ed25519 program instruction signing `message` with `key`, all parts in its own data.
pub fn ed25519_ix(key: &Keypair, message: &[u8]) -> Instruction {
    let sig = key.sign_message(message);
    let (pk_off, sig_off, msg_off) = (16u16, 48u16, 112u16);
    let mut data = vec![1u8, 0u8];
    for v in [sig_off, u16::MAX, pk_off, u16::MAX, msg_off, message.len() as u16, u16::MAX] {
        data.extend_from_slice(&v.to_le_bytes());
    }
    data.extend_from_slice(key.pubkey().as_ref());
    data.extend_from_slice(sig.as_ref());
    data.extend_from_slice(message);
    Instruction {
        program_id: ED25519_PROGRAM_ID,
        accounts: vec![],
        data,
    }
}

/// A registered agent.
pub struct Agent {
    pub operator: Keypair,
    pub key: Keypair,
    pub passport: Pubkey,
    pub badge: Pubkey,
}

/// The agents world.
pub struct Aw {
    pub hw: Hw,
    /// The Soulbound item every badge equips.
    pub soulbound: Pubkey,
    /// Receives passport fees.
    pub fee_collector: Pubkey,
    /// The two TEST verifiers.
    pub verifiers: [Keypair; 2],
}

impl Default for Aw {
    fn default() -> Self {
        Self::new()
    }
}

impl Aw {
    /// The armory world plus the agents program, template 42, the Soulbound item and the config.
    pub fn new() -> Self {
        let mut hw = Hw::new();
        // The ed25519 precompile, for link statements (09 5.2).
        let svm = std::mem::take(&mut hw.w.env.svm);
        hw.w.env.svm = svm.with_precompiles();
        for (name, id) in [
            ("hookwars_agents", ids::AGENTS_ID),
            ("agents_caller_stub", ids::WAR_ID),
        ] {
            hw.w.env
                .svm
                .add_program(id, &program_bytes(name))
                .unwrap_or_else(|e| panic!("load {name}: {e:?}"));
        }
        let deployer = hw.w.env.deployer.pubkey();
        hw.w.env.set_upgrade_authority(ids::AGENTS_ID, Some(deployer));
        // Template 42, registered from the items program (TEST: open authoring so the admin can
        // create the one item; see the integration notes in 09).
        let shape = hookwars_common::shape(template_id::SOULBOUND).expect("template 42");
        let args = hookwars_armory::RegisterTemplateArgs {
            id: template_id::SOULBOUND,
            code_hash: [42; 32],
            kind: shape.kind,
            field_count: shape.field_count,
            field_min: [0; PARAM_FIELDS],
            field_max: [0; PARAM_FIELDS],
            open_authoring: true,
            loot_enabled: false,
            forge_enabled: false,
            max_level: 1,
            loot_royalty_bps: 0,
            max_targets: 0,
            name: "Soulbound".to_string(),
        };
        let admin = hw.admin.insecure_clone();
        hw.register_with(&admin, ids::ITEMS_ID, args).ok();
        let (tx, soulbound, _) = hw.create_item(&admin, template_id::SOULBOUND, [0; PARAM_FIELDS], 0);
        tx.ok();
        let fee_collector = hw.w.env.funded(1_000_000_000).pubkey();
        let verifiers = [hw.w.env.funded(1_000_000_000), hw.w.env.funded(1_000_000_000)];
        let mut aw = Self {
            hw,
            soulbound,
            fee_collector,
            verifiers,
        };
        aw.init().ok();
        aw
    }

    /// The config args with the TEST values.
    pub fn config_args(&self) -> ConfigArgs {
        ConfigArgs {
            admin: self.hw.admin.pubkey(),
            fee_collector: self.fee_collector,
            soulbound_item: self.soulbound,
            params: TEST_AGENTS_PARAMS,
            verifiers: self.verifiers.iter().map(Signer::pubkey).collect(),
            targets: vec![bordrless_token::ID],
        }
    }

    /// `init_config` by the deployer.
    pub fn init(&mut self) -> Tx {
        let ix = agents_ix(
            hookwars_agents::accounts::InitConfig {
                authority: self.hw.admin.pubkey(),
                config: apda::config().0,
                program_data: hookwars_common::programdata_address(&ids::AGENTS_ID),
                system_program: anchor_lang::system_program::ID,
                event_authority: agents_events(),
                program: ids::AGENTS_ID,
            },
            hookwars_agents::instruction::InitConfig {
                args: self.config_args(),
            },
        );
        let admin = self.hw.admin.insecure_clone();
        self.hw.w.env.send_paid_by(&[ix], &admin, &[])
    }

    /// The `register_passport` instruction for `operator`'s next passport.
    pub fn register_ix(&self, operator: &Pubkey, key: &Pubkey, payer: &Pubkey, args: ProfileArgs) -> (Instruction, Pubkey) {
        let index = self
            .hw
            .w
            .env
            .try_read::<hookwars_agents::state::OperatorIndex>(&apda::operator(operator).0)
            .map_or(0, |o| o.next);
        let passport = apda::passport(operator, index).0;
        let ix = agents_ix(
            hookwars_agents::accounts::RegisterPassport {
                operator: *operator,
                agent_key: *key,
                payer: *payer,
                config: apda::config().0,
                fee_collector: self.fee_collector,
                operator_index: apda::operator(operator).0,
                passport,
                agent_key_record: apda::agent_key(key).0,
                badge_mint: apda::badge_mint(&passport, 0).0,
                token: token_accs(),
                system_program: anchor_lang::system_program::ID,
                event_authority: agents_events(),
                program: ids::AGENTS_ID,
            },
            hookwars_agents::instruction::RegisterPassport { index, args },
        );
        (ix, passport)
    }

    /// Registers a passport for a fresh operator and key; panics on failure.
    pub fn agent(&mut self, name: &str, kinds: u8) -> Agent {
        let operator = self.hw.w.env.funded(10_000_000_000);
        let key = self.hw.w.env.funded(10_000_000_000);
        self.agent_of(operator, key, name, kinds)
    }

    /// Registers a passport for `operator` and `key`; panics on failure.
    pub fn agent_of(&mut self, operator: Keypair, key: Keypair, name: &str, kinds: u8) -> Agent {
        let (ix, passport) = self.register_ix(&operator.pubkey(), &key.pubkey(), &operator.pubkey(), profile(name, kinds));
        self.hw.w.env.send_paid_by(&[ix], &operator, &[&key]).ok();
        let badge = self.passport(&passport).badge_mint;
        Agent {
            operator,
            key,
            passport,
            badge,
        }
    }

    /// The passport.
    pub fn passport(&self, key: &Pubkey) -> Passport {
        self.hw.w.env.read(key)
    }

    /// Equips the Soulbound item into `badge`'s slot through `launch_stub` (stand-in for the
    /// agents caller, which the armory does not accept yet).
    pub fn equip_badge_stub(&mut self, payer: &Keypair, badge: &Pubkey) -> Tx {
        let entry = Hw::entry(0, Some(self.soulbound), EquipConfig::default());
        self.hw.equip_launch(payer, badge, entry)
    }

    /// The `issue_badge` instruction.
    pub fn issue_ix(&self, payer: &Pubkey, passport: &Pubkey) -> Instruction {
        let p = self.passport(passport);
        agents_ix(
            hookwars_agents::accounts::IssueBadge {
                payer: *payer,
                config: apda::config().0,
                passport: *passport,
                agent_key: p.agent_key,
                badge_mint: p.badge_mint,
                badge_holding: token::holding_address(&p.badge_mint, &p.agent_key),
                signer: ids::AGENTS_SIGNER,
                token: token_accs(),
                system_program: anchor_lang::system_program::ID,
                event_authority: agents_events(),
                program: ids::AGENTS_ID,
            },
            hookwars_agents::instruction::IssueBadge {},
        )
    }

    /// Equips and issues `agent`'s current badge; panics on failure.
    pub fn badge(&mut self, agent: &Agent) {
        let p = self.passport(&agent.passport);
        self.equip_badge_stub(&agent.operator, &p.badge_mint).ok();
        let ix = self.issue_ix(&agent.operator.pubkey(), &agent.passport);
        self.hw.w.env.send_paid_by(&[ix], &agent.operator, &[]).ok();
    }

    /// The `set_status` instruction.
    pub fn status_ix(&self, agent: &Agent, status: u8) -> Instruction {
        let p = self.passport(&agent.passport);
        agents_ix(
            hookwars_agents::accounts::SetStatus {
                operator: p.operator,
                config: apda::config().0,
                passport: agent.passport,
                operator_index: apda::operator(&p.operator).0,
                badge_mint: p.badge_mint,
                badge_holding: token::holding_address(&p.badge_mint, &p.agent_key),
                signer: ids::AGENTS_SIGNER,
                token: token_accs(),
                event_authority: agents_events(),
                program: ids::AGENTS_ID,
            },
            hookwars_agents::instruction::SetStatus { status },
        )
    }

    /// `record(kind, value)` through the test recorder, naming `actor`.
    pub fn record_ix(&self, passport: &Pubkey, actor: &Pubkey, kind: u8, value: u64) -> Instruction {
        let inner = agents_ix(
            hookwars_agents::accounts::Record {
                caller: hookwars_agents::constants::RECORDERS[1],
                passport: *passport,
                actor: *actor,
                event_authority: agents_events(),
                program: ids::AGENTS_ID,
            },
            hookwars_agents::instruction::Record { kind, value },
        );
        let mut accounts = vec![AccountMeta::new_readonly(ids::AGENTS_ID, false)];
        accounts.extend(inner.accounts.into_iter().map(|mut m| {
            if m.pubkey == hookwars_agents::constants::RECORDERS[1] {
                m.is_signer = false;
            }
            m
        }));
        Instruction {
            program_id: ids::WAR_ID,
            accounts,
            data: agents_caller_stub::instruction::AsAgentsCaller { data: inner.data }.data(),
        }
    }

    /// The `link_social` instruction (without its ed25519 instruction).
    pub fn link_ix(&self, payer: &Pubkey, passport: &Pubkey, platform: u8, handle: &str) -> Instruction {
        agents_ix(
            hookwars_agents::accounts::LinkSocial {
                payer: *payer,
                config: apda::config().0,
                passport: *passport,
                link: apda::link(passport, platform).0,
                instructions: INSTRUCTIONS_SYSVAR_ID,
                system_program: anchor_lang::system_program::ID,
                event_authority: agents_events(),
                program: ids::AGENTS_ID,
            },
            hookwars_agents::instruction::LinkSocial {
                platform,
                handle: handle.to_string(),
                post_uri: format!("https://example.invalid/{handle}/status/1"),
            },
        )
    }

    /// The statement `agent` signs for `platform` and `handle`.
    pub fn statement(&self, agent: &Agent, platform: u8, handle: &str) -> Vec<u8> {
        let nonce = self.passport(&agent.passport).link_nonce;
        link_statement(&agent.passport, platform, handle, nonce).into_bytes()
    }

    /// `submit_attestation` args binding `agent`'s key, expiring `ttl` seconds from now.
    pub fn attest_args(&self, agent: &Agent, ttl: i64) -> AttestArgs {
        let nonce = [9u8; 32];
        AttestArgs {
            tee_kind: 0,
            measurement: [1u8; 48],
            report_data: report_data(&agent.key.pubkey(), &agent.passport, &nonce),
            nonce,
            quote_hash: [2u8; 32],
            quote_uri: "https://example.invalid/quote".to_string(),
            source_uri: "https://example.invalid/source".to_string(),
            expires_at: self.hw.w.env.now + ttl,
        }
    }

    /// The `submit_attestation` instruction.
    pub fn attest_ix(&self, agent: &Agent, args: AttestArgs) -> Instruction {
        agents_ix(
            hookwars_agents::accounts::SubmitAttestation {
                agent_key: agent.key.pubkey(),
                config: apda::config().0,
                passport: agent.passport,
                attestation: apda::attestation(&agent.passport).0,
                system_program: anchor_lang::system_program::ID,
                event_authority: agents_events(),
                program: ids::AGENTS_ID,
            },
            hookwars_agents::instruction::SubmitAttestation { args },
        )
    }

    /// The `endorse_attestation` instruction by `verifier`.
    pub fn endorse_ix(&self, passport: &Pubkey, verifier: &Pubkey) -> Instruction {
        let att = apda::attestation(passport).0;
        agents_ix(
            hookwars_agents::accounts::EndorseAttestation {
                verifier: *verifier,
                config: apda::config().0,
                passport: *passport,
                attestation: att,
                endorsement: apda::endorsement(&att, verifier).0,
                system_program: anchor_lang::system_program::ID,
                event_authority: agents_events(),
                program: ids::AGENTS_ID,
            },
            hookwars_agents::instruction::EndorseAttestation {},
        )
    }

    /// The `revoke_endorsement` instruction by `verifier`.
    pub fn revoke_ix(&self, passport: &Pubkey, verifier: &Pubkey) -> Instruction {
        let att = apda::attestation(passport).0;
        agents_ix(
            hookwars_agents::accounts::RevokeEndorsement {
                verifier: *verifier,
                config: apda::config().0,
                passport: *passport,
                attestation: att,
                endorsement: apda::endorsement(&att, verifier).0,
                event_authority: agents_events(),
                program: ids::AGENTS_ID,
            },
            hookwars_agents::instruction::RevokeEndorsement {},
        )
    }

    /// The `refresh_proof` instruction.
    pub fn refresh_ix(&self, passport: &Pubkey) -> Instruction {
        agents_ix(
            hookwars_agents::accounts::RefreshProof {
                passport: *passport,
                event_authority: agents_events(),
                program: ids::AGENTS_ID,
            },
            hookwars_agents::instruction::RefreshProof {},
        )
    }

    /// The `init_policy` instruction.
    pub fn init_policy_ix(&self, agent: &Agent, limits: PolicyLimits) -> Instruction {
        agents_ix(
            hookwars_agents::accounts::InitPolicy {
                operator: agent.operator.pubkey(),
                payer: agent.operator.pubkey(),
                config: apda::config().0,
                passport: agent.passport,
                policy: apda::policy(&agent.passport).0,
                system_program: anchor_lang::system_program::ID,
                event_authority: agents_events(),
                program: ids::AGENTS_ID,
            },
            hookwars_agents::instruction::InitPolicy { limits },
        )
    }

    /// `spend` of `inner` (an instruction the vault signs) by `agent`.
    pub fn spend_ix(&self, agent: &Agent, inner: Instruction) -> Instruction {
        let vault = apda::vault(&agent.passport).0;
        let mut accounts = hookwars_agents::accounts::Spend {
            agent_key: agent.key.pubkey(),
            config: apda::config().0,
            passport: agent.passport,
            policy: apda::policy(&agent.passport).0,
            vault,
            target_program: inner.program_id,
            event_authority: agents_events(),
            program: ids::AGENTS_ID,
        }
        .to_account_metas(None);
        accounts.extend(inner.accounts.into_iter().map(|mut m| {
            if m.pubkey == vault {
                m.is_signer = false;
            }
            m
        }));
        Instruction {
            program_id: ids::AGENTS_ID,
            accounts,
            data: hookwars_agents::instruction::Spend { data: inner.data }.data(),
        }
    }

    /// Operator policy instruction (`set_limits` or `freeze_policy`).
    pub fn operator_policy_accounts(&self, agent: &Agent) -> hookwars_agents::accounts::OperatorPolicy {
        hookwars_agents::accounts::OperatorPolicy {
            operator: agent.operator.pubkey(),
            config: apda::config().0,
            passport: agent.passport,
            policy: apda::policy(&agent.passport).0,
            event_authority: agents_events(),
            program: ids::AGENTS_ID,
        }
    }

    /// The `withdraw` accounts.
    pub fn withdraw_accounts(&self, agent: &Agent) -> hookwars_agents::accounts::Withdraw {
        hookwars_agents::accounts::Withdraw {
            operator: agent.operator.pubkey(),
            passport: agent.passport,
            policy: apda::policy(&agent.passport).0,
            vault: apda::vault(&agent.passport).0,
            token: token_accs(),
            system_program: anchor_lang::system_program::ID,
            event_authority: agents_events(),
            program: ids::AGENTS_ID,
        }
    }
}

/// An agents error's code.
pub fn agents_code(e: hookwars_agents::error::AgentsError) -> u32 {
    anchor_lang::error::ERROR_CODE_OFFSET + e as u32
}

/// The Soulbound template's error code.
pub fn soulbound_code() -> u32 {
    hookwars_items::ItemsError::SoulboundTransfer as u32 + 6000
}
