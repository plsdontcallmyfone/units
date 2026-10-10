//! The whole pipeline on the starter template and its four broken builds. These build with
//! `cargo build-sbf` (minutes), so they are ignored by default:
//!
//!   SBF_OUT_DIR=<the protocol's .so files> cargo test -p hooklab --test pipeline -- --ignored
//!
//! Each fixture is the starter with one feature on: the lab must name what is wrong with it.

use std::path::{Path, PathBuf};

use anchor_lang::prelude::Pubkey;
use bordrless_program_tests::armory::Hw;
use hooklab::manifest::Manifest;
use hooklab::{build, executable_hash, register, report, suite, Job};
use serde_json::Value;
use solana_keypair::Keypair;
use solana_signer::Signer;

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/hook-template")
}

fn manifest_text() -> String {
    std::fs::read_to_string(example_dir().join("hooklab.json")).unwrap()
}

fn out_dir(tag: &str) -> PathBuf {
    let base = std::env::var_os("HOOKLAB_OUT").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join(format!("hooklab-test-{tag}"))
}

/// Builds the starter with `features` (one build at a time: they share a target directory).
fn so(feature: Option<&str>) -> Vec<u8> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if std::env::var_os("HOOKLAB_CARGO_TARGET_DIR").is_none() {
        std::env::set_var("HOOKLAB_CARGO_TARGET_DIR", out_dir("target"));
    }
    let tag = feature.unwrap_or("clean");
    let features: Vec<String> = feature.into_iter().map(String::from).collect();
    let built = build::build(&example_dir(), &out_dir(tag), &features).unwrap_or_else(|e| panic!("{tag}: {e}"));
    std::fs::read(built.so).unwrap()
}

fn settings() -> suite::Settings {
    suite::Settings {
        seed: 7,
        random_sets: 2,
        ops: 40,
    }
}

fn classes(r: &suite::SuiteResult) -> Vec<String> {
    let mut c: Vec<String> = r.violations.iter().map(|v| v.class.clone()).collect();
    c.sort();
    c.dedup();
    c
}

fn manifest() -> Manifest {
    Manifest::parse(&manifest_text()).unwrap()
}

#[test]
#[ignore = "builds with cargo build-sbf"]
fn the_starter_template_passes_and_its_report_verifies() {
    let bytes = so(None);
    let admin = Pubkey::new_unique();
    let body = hooklab::check(Job {
        manifest_text: &manifest_text(),
        so: &bytes,
        build: None,
        submission: "ab".repeat(32),
        settings: settings(),
        template_id: Some(100),
        admin: Some(admin),
    });
    assert_eq!(body["verdict"], "pass", "{}", serde_json::to_string_pretty(&body).unwrap());
    let sets = body["suite"]["sets"].as_array().unwrap();
    assert!(sets.len() >= 3);
    // At the maximum cut (500 bps) the equip vault collected something; at the minimum (0) nothing.
    assert_eq!(sets[0]["cut_total"], 0);
    assert!(sets[1]["cut_total"].as_u64().unwrap() > 0);
    assert!(sets.iter().all(|s| s["closed_holdings"].as_u64().unwrap() >= 1));
    let cu = body["suite"]["max_cu_per_call"].as_u64().unwrap();
    println!("starter: {cu} CU per call (transaction minus the empty-slot transfer)");
    assert_eq!(body["code_hash"], hooklab::hex(&executable_hash(&bytes)));
    let reg = &body["register_template"];
    assert_eq!(reg["args"]["id"], 100);
    assert_eq!(reg["accounts"][0]["pubkey"], admin.to_string());
    let key = Keypair::new();
    let signed = report::sign(body, &key);
    assert!(report::verify(&signed, Some(&key.pubkey().to_string())).is_ok());
    // The author is never in the report (no `author` field; `open_authoring` is a registry flag).
    assert!(!serde_json::to_string(&signed).unwrap().contains("\"author\""));
}

#[test]
#[ignore = "builds with cargo build-sbf"]
fn an_over_cut_is_named() {
    let r = suite::run(&manifest(), &so(Some("overcut")), settings()).unwrap();
    assert!(classes(&r).contains(&"over_cut".to_string()), "{:?}", r.violations);
    // Only near the top of the range: over 400 bps the extra 100 passes the 500 bound.
    assert!(r.violations.iter().filter(|v| v.class == "over_cut").all(|v| v.params[0] > 400));
    assert!(r.violations.iter().any(|v| v.params == vec![500]));
}

#[test]
#[ignore = "builds with cargo build-sbf"]
fn a_refusal_the_manifest_denies_is_named() {
    let r = suite::run(&manifest(), &so(Some("refuse")), settings()).unwrap();
    assert!(classes(&r).contains(&"refusal".to_string()), "{:?}", r.violations);
    assert!(r.sets.iter().any(|s| s.refusals > 0));
}

#[test]
#[ignore = "builds with cargo build-sbf"]
fn a_bad_write_is_named() {
    let r = suite::run(&manifest(), &so(Some("badwrite")), settings()).unwrap();
    assert_eq!(classes(&r).first().map(String::as_str), Some("bad_answer"), "{:?}", r.violations);
    assert!(r.violations.iter().any(|v| v.detail.contains("SlotDataLength")));
}

#[test]
#[ignore = "builds with cargo build-sbf"]
fn compute_over_the_declared_bound_is_named() {
    let r = suite::run(&manifest(), &so(Some("cuburn")), settings()).unwrap();
    let c = classes(&r);
    assert!(
        c.contains(&"cu_over_declared".to_string()) || c.contains(&"cu_blowup".to_string()),
        "{:?}",
        r.violations
    );
}

#[test]
#[ignore = "builds with cargo build-sbf"]
fn the_armory_refuses_an_external_template_id_today() {
    // Integration gap (README "Gaps"): register_template takes only ids in hookwars_common::shape,
    // so the instruction a passing report carries is refused until the armory accepts external
    // templates. This test fails when that changes, so the README gets updated with it.
    let bytes = so(None);
    let m = manifest();
    let program = m.program().unwrap();
    let mut hw = Hw::new();
    hw.w.env.svm.add_program(program, &bytes).unwrap();
    hw.w.env.set_upgrade_authority(program, None);
    let args = register::args(&m, 1000, executable_hash(&bytes)).unwrap();
    let ix = register::instruction(hw.admin.pubkey(), program, args);
    let admin = hw.admin.insecure_clone();
    let tx = hw.w.env.send_paid_by(&[ix], &admin, &[]);
    let logs = tx.logs().join("\n");
    assert!(tx.result.is_err());
    assert!(logs.contains("InvalidSchema"), "{logs}");
}

#[test]
#[ignore = "builds with cargo build-sbf"]
fn the_cli_writes_a_report_the_cli_verifies() {
    let bin = env!("CARGO_BIN_EXE_hooklab");
    let so_path = out_dir("clean").join("hook_template_example.so");
    if !so_path.exists() {
        so(None);
    }
    let dir = out_dir("cli");
    std::fs::create_dir_all(&dir).unwrap();
    let key = Keypair::new();
    let key_path = dir.join("key.json");
    std::fs::write(&key_path, serde_json::to_string(&key.to_bytes().to_vec()).unwrap()).unwrap();
    let report_path = dir.join("report.json");
    let st = std::process::Command::new(bin)
        .args(["check"])
        .arg(example_dir())
        .arg("--so")
        .arg(&so_path)
        .args(["--ops", "16", "--random-sets", "1", "--key"])
        .arg(&key_path)
        .arg("--report")
        .arg(&report_path)
        .status()
        .unwrap();
    assert!(st.success());
    let st = std::process::Command::new(bin)
        .arg("verify")
        .arg(&report_path)
        .args(["--signer", &key.pubkey().to_string()])
        .status()
        .unwrap();
    assert!(st.success());
    let v: Value = serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    assert_eq!(v["body"]["verdict"], "pass");
    // No template id or admin given: no instruction.
    assert!(v["body"]["register_template"].is_null());
}

// ---- security review 3 M-9: the build and the signature never share a process ----------------------

#[test]
fn check_no_build_refuses_to_build_and_a_sandboxed_build_failure_is_a_signed_fail() {
    let bin = env!("CARGO_BIN_EXE_hooklab");
    let dir = out_dir("m9");
    std::fs::create_dir_all(&dir).unwrap();
    let key = Keypair::new();
    let key_path = dir.join("key.json");
    std::fs::write(&key_path, serde_json::to_string(&key.to_bytes().to_vec()).unwrap()).unwrap();
    // `--no-build` without a built `.so`: a usage error, never a build next to the key.
    let st = std::process::Command::new(bin)
        .arg("check")
        .arg(example_dir())
        .arg("--no-build")
        .arg("--key")
        .arg(&key_path)
        .status()
        .unwrap();
    assert_eq!(st.code(), Some(2));
    // The sandboxed stage failed: the keyed stage signs a fail with the reason.
    let err = dir.join("build-error.txt");
    std::fs::write(&err, "cargo build-sbf failed: a test reason").unwrap();
    let report_path = dir.join("report.json");
    let st = std::process::Command::new(bin)
        .arg("check")
        .arg(example_dir())
        .arg("--no-build")
        .arg("--build-error")
        .arg(&err)
        .arg("--key")
        .arg(&key_path)
        .arg("--report")
        .arg(&report_path)
        .args(["--submission", &"ab".repeat(32)])
        .status()
        .unwrap();
    assert_eq!(st.code(), Some(1));
    let v: Value = serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    assert!(report::verify(&v, Some(&key.pubkey().to_string())).is_ok());
    assert_eq!(v["body"]["verdict"], "fail");
    assert!(v["body"]["problems"][0].as_str().unwrap().contains("a test reason"));
}

#[test]
#[ignore = "builds with cargo build-sbf"]
fn the_two_stage_cli_builds_without_a_key_then_checks_and_signs() {
    let bin = env!("CARGO_BIN_EXE_hooklab");
    let dir = out_dir("m9-two-stage");
    std::fs::create_dir_all(&dir).unwrap();
    if std::env::var_os("HOOKLAB_CARGO_TARGET_DIR").is_none() {
        std::env::set_var("HOOKLAB_CARGO_TARGET_DIR", out_dir("target"));
    }
    let so_out = dir.join("template.so");
    let st = std::process::Command::new(bin)
        .arg("build")
        .arg(example_dir())
        .arg("--so-out")
        .arg(&so_out)
        .arg("--out-dir")
        .arg(dir.join("out"))
        .status()
        .unwrap();
    assert!(st.success());
    let meta = PathBuf::from(format!("{}.json", so_out.display()));
    assert!(so_out.exists() && meta.exists());
    let key = Keypair::new();
    let key_path = dir.join("key.json");
    std::fs::write(&key_path, serde_json::to_string(&key.to_bytes().to_vec()).unwrap()).unwrap();
    let report_path = dir.join("report.json");
    let st = std::process::Command::new(bin)
        .arg("check")
        .arg(example_dir())
        .arg("--no-build")
        .arg("--so")
        .arg(&so_out)
        .arg("--build-meta")
        .arg(&meta)
        .args(["--ops", "16", "--random-sets", "1", "--key"])
        .arg(&key_path)
        .arg("--report")
        .arg(&report_path)
        .status()
        .unwrap();
    assert!(st.success());
    let v: Value = serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    assert!(report::verify(&v, Some(&key.pubkey().to_string())).is_ok());
    assert_eq!(v["body"]["verdict"], "pass");
    assert_eq!(v["body"]["build"]["tools_version"], "v1.57");
}
