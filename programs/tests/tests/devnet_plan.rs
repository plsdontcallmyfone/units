// Changed by Hookwars: new file. Builds the devnet init plan from the same instruction builders the
// suites use, runs it in LiteSVM to prove it lands in order, and writes it as JSON for
// scripts/devnet/send-plan.mjs. Ignored by default: run it explicitly (docs/DEVNET.md).
// Integration pass 3: craft, book and the social skill table join the plan.
//
//   cargo test -p bordrless-program-tests --test devnet_plan -- --ignored --nocapture
//
// Every parameter value is a TEST value taken from the suites' own constants (armory TEST_PARAMS,
// war TEST_PARAMS, TEST_MARKET, TEST_SOCIAL, TEST_AGENTS_PARAMS, the fixture's launch policy and
// the template TEST schemas). None of them is an owner decision (docs/spec/00 section 6).

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::Instruction;
use bordrless_program_tests::agents::{agents_events, agents_ix, TEST_AGENTS_PARAMS};
use bordrless_program_tests::armory::{armory_events, armory_ix, test_schema, token_accounts, Hw, TEST_PARAMS};
use bordrless_program_tests::env::Env;
use bordrless_program_tests::expansion::{market_ix, social_config, social_ix, TEST_MARKET, TEST_SOCIAL};
use bordrless_program_tests::fixture::policy_launch_config;
use bordrless_program_tests::program_bytes;
use bordrless_token::client as token;
use hookwars_armory::cpi::{ARMORY_SIGNER, MINTER};
use hookwars_armory::RegisterTemplateArgs;
use hookwars_common::{ids, pda, shape, template_id as t, PARAM_FIELDS};
use solana_keypair::Keypair;
use solana_signer::Signer;
use std::path::PathBuf;

/// One plan step: the instruction, a label, and the account whose existence means the step is done.
struct Step {
    label: String,
    ix: Instruction,
    creates: Option<Pubkey>,
    /// Which keypair signs besides the fee payer (the deployer pays every step).
    signers: Vec<&'static str>,
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_keypair(path: &PathBuf) -> Keypair {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let bytes: Vec<u8> = text
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|s| s.trim().parse::<u8>().expect("keypair byte"))
        .collect();
    Keypair::try_from(bytes.as_slice()).expect("keypair")
}

/// Template names from docs/spec/08-arsenal.md section 6 (ids 1 to 41) and 09, 10 (42 to 45).
fn template_names() -> Vec<(u16, String)> {
    let doc = std::fs::read_to_string(root().join("docs/spec/08-arsenal.md")).expect("spec 08");
    let start = doc.find("## 6. Template list").expect("section 6");
    let mut out = Vec::new();
    for line in doc[start..].lines().skip(1) {
        if line.starts_with("## ") {
            break;
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if cells.len() > 3 {
            if let Ok(id) = cells[1].parse::<u16>() {
                out.push((id, cells[2].to_string()));
            }
        }
    }
    for (id, name) in [(42u16, "Soulbound"), (43, "Coalition"), (44, "Boss"), (45, "Rivalry")] {
        if !out.iter().any(|(i, _)| *i == id) {
            out.push((id, name.to_string()));
        }
    }
    out
}

fn padded(v: &[u32]) -> [u32; PARAM_FIELDS] {
    let mut a = [0u32; PARAM_FIELDS];
    a[..v.len()].copy_from_slice(v);
    a
}

/// Registration args for template `id`: the suites' TEST schema, the real name, and the sha256 of
/// the items program binary that is deployed (`code_hash`).
fn register_args(id: u16, name: &str, code_hash: [u8; 32]) -> RegisterTemplateArgs {
    let s = shape(id).expect("shape");
    match id {
        t::SOULBOUND => RegisterTemplateArgs {
            id,
            code_hash,
            kind: s.kind,
            field_count: s.field_count,
            field_min: [0; PARAM_FIELDS],
            field_max: [0; PARAM_FIELDS],
            open_authoring: true,
            loot_enabled: false,
            forge_enabled: false,
            max_level: 1,
            loot_royalty_bps: 0,
            max_targets: 0,
            name: name.to_string(),
        },
        t::COALITION | t::BOSS | t::RIVALRY => {
            let (fields, max) = match id {
                t::COALITION => (2u8, padded(&[u32::MAX, 10_000])),
                t::BOSS => (1, padded(&[u32::MAX])),
                _ => (3, padded(&[u32::MAX, u32::MAX, 10_000])),
            };
            RegisterTemplateArgs {
                id,
                code_hash,
                kind: s.kind,
                field_count: fields,
                field_min: [0; PARAM_FIELDS],
                field_max: max,
                open_authoring: true,
                loot_enabled: false,
                forge_enabled: false,
                max_level: 1,
                loot_royalty_bps: 0,
                max_targets: 1,
                name: name.to_string(),
            }
        }
        _ => {
            let mut a = Hw::template_args(id);
            let (field_min, field_max, max_targets, forge) = test_schema(id);
            a.field_min = field_min;
            a.field_max = field_max;
            a.max_targets = max_targets;
            a.forge_enabled = forge;
            a.code_hash = code_hash;
            a.name = name.to_string();
            a
        }
    }
}

fn plan(deployer: Pubkey, code_hash: [u8; 32]) -> Vec<Step> {
    let sol = bordrless_swap::constants::BRIDGED_SOL_MINT;
    let mut steps = vec![
        Step {
            label: "swap init_config (TEST, fixture policy)".into(),
            ix: bordrless_swap::client::init_config(
                deployer,
                bordrless_swap::instructions::ConfigArgs {
                    admin: deployer,
                    protocol_fee_bps: bordrless_core::policy::PROTOCOL_FEE_BPS,
                    launch_protocol_share_bps: bordrless_core::policy::LAUNCH_PROTOCOL_SHARE_BPS,
                    fee_collector: deployer,
                    treasury: deployer,
                    pool_creation_fee_lamports: 0,
                    paused: false,
                },
            ),
            creates: Some(bordrless_launch::constants::DEX_CONFIG),
            signers: vec![],
        },
        Step {
            label: "bridge init_config".into(),
            ix: bordrless_bridge::client::init_config(
                deployer,
                bordrless_bridge::instructions::ConfigArgs { admin: deployer, paused: false },
            ),
            creates: Some(Pubkey::find_program_address(&[b"config"], &bordrless_bridge::ID).0),
            signers: vec![],
        },
        Step {
            label: "launch init_config (TEST, fixture policy)".into(),
            ix: bordrless_launch::client::init_config(deployer, policy_launch_config(deployer, deployer, sol)),
            creates: Some(bordrless_launch::constants::CONFIG_ADDRESS),
            signers: vec![],
        },
        Step {
            label: "bridge register_sol (bridged SOL wrapper)".into(),
            ix: bordrless_bridge::client::register_sol(deployer),
            creates: Some(sol),
            signers: vec![],
        },
        Step {
            label: "armory init (TEST_PARAMS)".into(),
            ix: armory_ix(
                hookwars_armory::accounts::Init {
                    authority: deployer,
                    config: pda::config().0,
                    program_data: hookwars_common::programdata_address(&ids::ARMORY_ID),
                    system_program: anchor_lang::system_program::ID,
                },
                hookwars_armory::instruction::Init { admin: deployer, params: TEST_PARAMS },
            ),
            creates: Some(pda::config().0),
            signers: vec![],
        },
    ];
    for (id, name) in template_names() {
        if shape(id).is_none() {
            continue;
        }
        let args = register_args(id, &name, code_hash);
        steps.push(Step {
            label: format!("armory register_template {id} {name}"),
            ix: armory_ix(
                hookwars_armory::accounts::RegisterTemplate {
                    admin: deployer,
                    config: pda::config().0,
                    template: pda::template(id).0,
                    template_program: ids::ITEMS_ID,
                    programdata: hookwars_common::programdata_address(&ids::ITEMS_ID),
                    system_program: anchor_lang::system_program::ID,
                    event_authority: armory_events(),
                    program: ids::ARMORY_ID,
                },
                hookwars_armory::instruction::RegisterTemplate { args },
            ),
            creates: Some(pda::template(id).0),
            signers: vec![],
        });
    }
    // The one Soulbound item every agent badge equips (item number 0 on a fresh armory).
    let item_mint = pda::item_mint(0).0;
    let soulbound = pda::item(&item_mint).0;
    steps.push(Step {
        label: "armory create_item Soulbound (item 0, the badge item)".into(),
        ix: armory_ix(
            hookwars_armory::accounts::CreateItem {
                author: deployer,
                config: pda::config().0,
                template: pda::template(t::SOULBOUND).0,
                minter: MINTER,
                item_mint,
                item: soulbound,
                recipient_holding: token::holding_address(&item_mint, &deployer),
                armory_signer: ARMORY_SIGNER,
                items_program: ids::ITEMS_ID,
                token: token_accounts(),
                system_program: anchor_lang::system_program::ID,
                event_authority: armory_events(),
                program: ids::ARMORY_ID,
            },
            hookwars_armory::instruction::CreateItem {
                template_id: t::SOULBOUND,
                params: [0; PARAM_FIELDS],
                royalty_bps: 0,
            },
        ),
        creates: Some(soulbound),
        signers: vec![],
    });
    steps.push(Step {
        label: "war init_config (TEST_PARAMS; randomness = randomness_stub id, TEST only)".into(),
        ix: hookwars_war::client::init_config(
            deployer,
            hookwars_war::instructions::ConfigArgs {
                admin: deployer,
                protocol_treasury: deployer,
                randomness_program: randomness_stub::ID,
                treaty_template_id: Some(t::TREATY),
                params: bordrless_program_tests::war::TEST_PARAMS,
            },
        ),
        creates: Some(Pubkey::find_program_address(&[b"war-config"], &ids::WAR_ID).0),
        signers: vec![],
    });
    steps.push(Step {
        label: "market init (TEST_MARKET)".into(),
        ix: market_ix(
            hookwars_market::accounts::Init {
                authority: deployer,
                config: hookwars_market::state::config_address().0,
                program_data: hookwars_common::programdata_address(&hookwars_market::ID),
                system_program: anchor_lang::system_program::ID,
            },
            hookwars_market::instruction::Init { admin: deployer, treasury: deployer, params: TEST_MARKET },
        ),
        creates: Some(hookwars_market::state::config_address().0),
        signers: vec![],
    });
    steps.push(Step {
        label: "social init (TEST_SOCIAL)".into(),
        ix: social_ix(
            hookwars_social::accounts::Init {
                authority: deployer,
                config: social_config(),
                program_data: hookwars_common::programdata_address(&hookwars_social::ID),
                system_program: anchor_lang::system_program::ID,
            },
            hookwars_social::instruction::Init { admin: deployer, params: TEST_SOCIAL },
        ),
        creates: Some(social_config()),
        signers: vec![],
    });
    steps.push(Step {
        label: "agents init_config (TEST_AGENTS_PARAMS; verifiers = deployer and protocol authority, quorum 2, TEST only)".into(),
        ix: agents_ix(
            hookwars_agents::accounts::InitConfig {
                authority: deployer,
                config: hookwars_agents::constants::pda::config().0,
                program_data: hookwars_common::programdata_address(&ids::AGENTS_ID),
                system_program: anchor_lang::system_program::ID,
                event_authority: agents_events(),
                program: ids::AGENTS_ID,
            },
            hookwars_agents::instruction::InitConfig {
                args: hookwars_agents::ConfigArgs {
                    admin: deployer,
                    fee_collector: deployer,
                    soulbound_item: soulbound,
                    params: TEST_AGENTS_PARAMS,
                    verifiers: vec![deployer, ids::PROTOCOL_AUTHORITY],
                    targets: vec![bordrless_token::ID],
                },
            },
        ),
        creates: Some(hookwars_agents::constants::pda::config().0),
        signers: vec![],
    });
    // Integration pass 3 (13): the economy, wired to the real programs: craft's output is the
    // armory and its callers the armory and items; social counts calls from craft, book, market and
    // the armory.
    use bordrless_program_tests::economy::{book_ix, craft_ix, test_skills, TEST_BOOK, TEST_CRAFT};
    steps.push(Step {
        label: "craft init (TEST_CRAFT; output the armory, callers armory and items)".into(),
        ix: craft_ix(
            hookwars_craft::accounts::Init {
                authority: deployer,
                config: hookwars_craft::state::config_address().0,
                program_data: hookwars_common::programdata_address(&hookwars_craft::ID),
                system_program: anchor_lang::system_program::ID,
            },
            hookwars_craft::instruction::Init {
                admin: deployer,
                treasury: deployer,
                season_pool: deployer,
                output_program: ids::ARMORY_ID,
                callers: vec![ids::ARMORY_ID, ids::ITEMS_ID],
                params: TEST_CRAFT,
            },
        ),
        creates: Some(hookwars_craft::state::config_address().0),
        signers: vec![],
    });
    steps.push(Step {
        label: "book init (TEST_BOOK)".into(),
        ix: book_ix(
            hookwars_book::accounts::Init {
                authority: deployer,
                config: hookwars_book::state::config_address().0,
                program_data: hookwars_common::programdata_address(&hookwars_book::ID),
                system_program: anchor_lang::system_program::ID,
            },
            hookwars_book::instruction::Init { admin: deployer, treasury: deployer, params: TEST_BOOK },
        ),
        creates: Some(hookwars_book::state::config_address().0),
        signers: vec![],
    });
    steps.push(Step {
        label: "social init_skills (TEST skills; callers craft, book, market, armory)".into(),
        ix: social_ix(
            hookwars_social::accounts::InitSkills {
                admin: deployer,
                config: social_config(),
                skills: hookwars_social::skills_address().0,
                system_program: anchor_lang::system_program::ID,
            },
            hookwars_social::instruction::InitSkills {
                skills: test_skills(),
                callers: vec![hookwars_craft::ID, hookwars_book::ID, hookwars_market::ID, ids::ARMORY_ID],
            },
        ),
        creates: Some(hookwars_social::skills_address().0),
        signers: vec![],
    });
    steps
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn to_json(steps: &[Step], deployer: &Pubkey, code_hash: &[u8; 32]) -> String {
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str("  \"note\": \"Generated by programs/tests/tests/devnet_plan.rs. TEST parameter values from the suites; not owner decisions.\",\n");
    s.push_str(&format!("  \"deployer\": \"{deployer}\",\n"));
    s.push_str(&format!("  \"items_code_hash\": \"{}\",\n", hex(code_hash)));
    s.push_str("  \"steps\": [\n");
    for (i, st) in steps.iter().enumerate() {
        s.push_str("    {\n");
        s.push_str(&format!("      \"step\": {},\n", i + 1));
        s.push_str(&format!("      \"label\": \"{}\",\n", st.label.replace('"', "'")));
        s.push_str(&format!("      \"program\": \"{}\",\n", st.ix.program_id));
        match st.creates {
            Some(k) => s.push_str(&format!("      \"creates\": \"{k}\",\n")),
            None => s.push_str("      \"creates\": null,\n"),
        }
        s.push_str(&format!(
            "      \"signers\": [{}],\n",
            st.signers.iter().map(|x| format!("\"{x}\"")).collect::<Vec<_>>().join(", ")
        ));
        s.push_str("      \"accounts\": [\n");
        for (j, m) in st.ix.accounts.iter().enumerate() {
            s.push_str(&format!(
                "        {{\"pubkey\": \"{}\", \"is_signer\": {}, \"is_writable\": {}}}{}\n",
                m.pubkey,
                m.is_signer,
                m.is_writable,
                if j + 1 < st.ix.accounts.len() { "," } else { "" }
            ));
        }
        s.push_str("      ],\n");
        s.push_str(&format!("      \"data_hex\": \"{}\"\n", hex(&st.ix.data)));
        s.push_str(if i + 1 < steps.len() { "    },\n" } else { "    }\n" });
    }
    s.push_str("  ]\n}\n");
    s
}

#[test]
#[ignore = "generates scripts/devnet/init-plan.json; run explicitly (docs/DEVNET.md)"]
fn devnet_init_plan() {
    let deployer_kp = read_keypair(&root().join("keys/devnet/deployer-keypair.json"));
    let deployer = deployer_kp.pubkey();
    let items_so = program_bytes("hookwars_items");
    let code_hash = hookwars_agents::constants::sha256(&items_so);
    let steps = plan(deployer, code_hash);

    // Prove the plan lands, in order, on a fresh SVM with every deployed program, the real deployer
    // as upgrade authority of the protocol programs and the protocol authority over the hooks.
    let mut env = Env::new();
    for (name, id) in [
        ("hookwars_armory", ids::ARMORY_ID),
        ("hookwars_items", ids::ITEMS_ID),
        ("hookwars_war", ids::WAR_ID),
        ("hookwars_market", hookwars_market::ID),
        ("hookwars_social", hookwars_social::ID),
        ("hookwars_agents", ids::AGENTS_ID),
        ("hookwars_craft", hookwars_craft::ID),
        ("hookwars_book", hookwars_book::ID),
    ] {
        env.svm.add_program(id, &program_bytes(name)).unwrap_or_else(|e| panic!("load {name}: {e:?}"));
    }
    env.fund(deployer, 100_000_000_000);
    for id in [
        bordrless_token::ID,
        bordrless_swap::ID,
        bordrless_bridge::ID,
        bordrless_launch::ID,
        bordrless_kit::ID,
        bordrless_companion::ID,
        ids::ARMORY_ID,
        ids::WAR_ID,
        hookwars_market::ID,
        hookwars_social::ID,
        ids::AGENTS_ID,
        hookwars_craft::ID,
        hookwars_book::ID,
    ] {
        env.set_upgrade_authority(id, Some(deployer));
    }
    for id in [ids::ITEMS_ID, half_life::ID, tax_hook::ID] {
        env.set_upgrade_authority(id, Some(ids::PROTOCOL_AUTHORITY));
    }
    let before = env.lamports(&deployer);
    for (i, st) in steps.iter().enumerate() {
        let tx = env.send_paid_by(&[st.ix.clone()], &deployer_kp, &[]);
        tx.ok();
        println!("plan step {} ok: {} ({} CU)", i + 1, st.label, tx.cu());
    }

    let spent = before - env.lamports(&deployer);
    println!("init plan cost to the deployer: {spent} lamports (rent of every config, template and item account, plus LiteSVM fees)");
    let json = to_json(&steps, &deployer, &code_hash);
    let out = root().join("scripts/devnet/init-plan.json");
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
    std::fs::write(&out, json).unwrap();
    println!("wrote {} ({} steps)", out.display(), steps.len());

    // Every TEST parameter set the plan uses, as the Rust constants print themselves (Debug), so
    // the readable file cannot drift from what init sends.
    let esc = |x: String| x.replace('\\', "\\\\").replace('"', "\\\"");
    let sol = bordrless_swap::constants::BRIDGED_SOL_MINT;
    let mut templates = String::new();
    for (i, (id, name)) in template_names().into_iter().filter(|(id, _)| shape(*id).is_some()).enumerate() {
        let a = register_args(id, &name, code_hash);
        templates.push_str(&format!(
            "{}    {{\"id\": {id}, \"name\": \"{}\", \"field_count\": {}, \"field_min\": {:?}, \"field_max\": {:?}, \"max_targets\": {}, \"forge_enabled\": {}, \"loot_enabled\": {}, \"max_level\": {}, \"loot_royalty_bps\": {}}}",
            if i == 0 { "" } else { ",\n" },
            esc(a.name.clone()), a.field_count, a.field_min, a.field_max, a.max_targets, a.forge_enabled, a.loot_enabled, a.max_level, a.loot_royalty_bps
        ));
    }
    let params = format!(
        "{{\n  \"note\": \"TEST values copied from the test suites' constants by programs/tests/tests/devnet_plan.rs. None is an owner decision (docs/spec/00 section 6).\",\n  \"launch_config\": \"{}\",\n  \"armory\": \"{}\",\n  \"war\": \"{}\",\n  \"market\": \"{}\",\n  \"social\": \"{}\",\n  \"agents\": \"{}\",\n  \"templates\": [\n{}\n  ]\n}}\n",
        esc(format!("{:?}", policy_launch_config(deployer, deployer, sol))),
        esc(format!("{:?}", TEST_PARAMS)),
        esc(format!("{:?}", bordrless_program_tests::war::TEST_PARAMS)),
        esc(format!("{:?}", TEST_MARKET)),
        esc(format!("{:?}", TEST_SOCIAL)),
        esc(format!("{:?}", TEST_AGENTS_PARAMS)),
        templates
    );
    let pout = root().join("scripts/devnet/params.test.json");
    std::fs::write(&pout, params).unwrap();
    println!("wrote {}", pout.display());
}
