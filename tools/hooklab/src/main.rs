//! `hooklab`: the Hook Lab command line (tools/hooklab/README.md).
//!
//!   hooklab manifest <crate-dir>
//!   hooklab check <crate-dir> [--so FILE] [--features a,b] [--out-dir DIR] [--seed N]
//!                 [--random-sets N] [--ops N] [--key FILE] [--report FILE]
//!                 [--submission HEX] [--template-id N] [--admin PUBKEY]
//!   hooklab verify <report.json> [--signer PUBKEY]
//!
//! Exit codes: 0 pass (or verified), 1 the template failed (or the report does not verify),
//! 2 usage or I/O error.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;

use anchor_lang::prelude::Pubkey;
use hooklab::{build, hex, manifest::Manifest, report, suite, tree_hash, Job};
use serde_json::Value;
use solana_keypair::Keypair;
use solana_signer::Signer;

const USAGE: &str = "usage:
  hooklab manifest <crate-dir>
  hooklab check <crate-dir> [--so FILE] [--features a,b] [--out-dir DIR] [--seed N] [--random-sets N] [--ops N] [--key FILE] [--report FILE] [--submission HEX] [--template-id N] [--admin PUBKEY]
  hooklab verify <report.json> [--signer PUBKEY]";

/// Default settings: two bound sets plus this many random ones, this many operations each.
const RANDOM_SETS: u32 = 4;
const OPS: u32 = 48;

struct Args {
    positional: Vec<String>,
    flags: BTreeMap<String, String>,
}

fn parse(argv: &[String]) -> Result<Args, String> {
    let mut positional = Vec::new();
    let mut flags = BTreeMap::new();
    let mut i = 0;
    while i < argv.len() {
        let a = &argv[i];
        if let Some(name) = a.strip_prefix("--") {
            let value = argv.get(i + 1).ok_or(format!("--{name} needs a value"))?;
            flags.insert(name.to_string(), value.clone());
            i += 2;
        } else {
            positional.push(a.clone());
            i += 1;
        }
    }
    Ok(Args { positional, flags })
}

fn num<T: FromStr>(a: &Args, name: &str, default: T) -> Result<T, String> {
    a.flags
        .get(name)
        .map(|v| v.parse().map_err(|_| format!("--{name}: not a number")))
        .unwrap_or(Ok(default))
}

fn read_key(path: &str) -> Result<Keypair, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let bytes: Vec<u8> = serde_json::from_str(&text).map_err(|_| format!("{path}: not a keypair file"))?;
    Keypair::try_from(bytes.as_slice()).map_err(|_| format!("{path}: not a keypair"))
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let result = (|| -> Result<bool, String> {
        let (cmd, rest) = argv.split_first().ok_or(USAGE)?;
        let a = parse(rest)?;
        match cmd.as_str() {
            "manifest" => manifest_cmd(&a),
            "check" => check_cmd(&a),
            "verify" => verify_cmd(&a),
            _ => Err(USAGE.into()),
        }
    })();
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("hooklab: {e}");
            ExitCode::from(2)
        }
    }
}

fn crate_dir(a: &Args) -> Result<PathBuf, String> {
    a.positional.first().map(PathBuf::from).ok_or_else(|| USAGE.to_string())
}

fn manifest_cmd(a: &Args) -> Result<bool, String> {
    let dir = crate_dir(a)?;
    let text = std::fs::read_to_string(dir.join("hooklab.json")).map_err(|e| format!("hooklab.json: {e}"))?;
    let problems = match Manifest::parse(&text) {
        Ok(m) => m.check(),
        Err(e) => vec![e],
    };
    for p in &problems {
        println!("problem: {p}");
    }
    if problems.is_empty() {
        println!("manifest ok");
    }
    Ok(problems.is_empty())
}

fn check_cmd(a: &Args) -> Result<bool, String> {
    let dir = crate_dir(a)?;
    let manifest_text =
        std::fs::read_to_string(dir.join("hooklab.json")).map_err(|e| format!("hooklab.json: {e}"))?;
    let features: Vec<String> = a
        .flags
        .get("features")
        .map(|f| f.split(',').filter(|s| !s.is_empty()).map(String::from).collect())
        .unwrap_or_default();
    let submission = match a.flags.get("submission") {
        Some(h) if h.len() == 64 && h.chars().all(|c| c.is_ascii_hexdigit()) => h.to_lowercase(),
        Some(_) => return Err("--submission: 64 hex characters".into()),
        None => hex(&tree_hash(&dir)?),
    };
    let (so, built) = match a.flags.get("so") {
        Some(p) => (std::fs::read(p).map_err(|e| format!("{p}: {e}"))?, None),
        None => {
            let out = a
                .flags
                .get("out-dir")
                .map(PathBuf::from)
                .unwrap_or_else(|| std::env::temp_dir().join(format!("hooklab-{}", &submission[..16])));
            match build::build(&dir, &out, &features) {
                Ok(b) => (std::fs::read(&b.so).map_err(|e| e.to_string())?, Some(b)),
                Err(e) => {
                    let body = serde_json::json!({
                        "schema": 1,
                        "submission": submission,
                        "verdict": "fail",
                        "problems": [format!("build: {e}")],
                        "register_template": Value::Null,
                    });
                    return finish(a, body);
                }
            }
        }
    };
    let admin = a
        .flags
        .get("admin")
        .map(|s| Pubkey::from_str(s).map_err(|_| "--admin: not a public key".to_string()))
        .transpose()?;
    let template_id = a
        .flags
        .get("template-id")
        .map(|s| s.parse::<u16>().map_err(|_| "--template-id: not a number".to_string()))
        .transpose()?;
    let body = hooklab::check(Job {
        manifest_text: &manifest_text,
        so: &so,
        build: built,
        submission,
        settings: suite::Settings {
            seed: num(a, "seed", 1)?,
            random_sets: num(a, "random-sets", RANDOM_SETS)?,
            ops: num(a, "ops", OPS)?,
        },
        template_id,
        admin,
    });
    finish(a, body)
}

fn finish(a: &Args, body: Value) -> Result<bool, String> {
    let passed = body["verdict"] == "pass";
    let key = match a.flags.get("key") {
        Some(p) => read_key(p)?,
        None => Keypair::new(),
    };
    let signed = report::sign(body, &key);
    let text = serde_json::to_string_pretty(&signed).map_err(|e| e.to_string())?;
    match a.flags.get("report") {
        Some(p) => std::fs::write(p, &text).map_err(|e| format!("{p}: {e}"))?,
        None => println!("{text}"),
    }
    let b = &signed["body"];
    eprintln!(
        "verdict {} ({} violations, signer {})",
        b["verdict"].as_str().unwrap_or("?"),
        b["suite"]["violations"].as_array().map_or(0, |v| v.len()),
        key.pubkey()
    );
    for v in b["suite"]["violations"].as_array().into_iter().flatten() {
        eprintln!("  {}: {}", v["class"].as_str().unwrap_or(""), v["detail"].as_str().unwrap_or(""));
    }
    for p in b["problems"].as_array().into_iter().flatten() {
        eprintln!("  problem: {}", p.as_str().unwrap_or(""));
    }
    Ok(passed)
}

fn verify_cmd(a: &Args) -> Result<bool, String> {
    let path = a.positional.first().ok_or(USAGE)?;
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let value: Value = serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
    match report::verify(&value, a.flags.get("signer").map(String::as_str)) {
        Ok(body) => {
            println!("verified: verdict {}", body["verdict"].as_str().unwrap_or("?"));
            Ok(true)
        }
        Err(e) => {
            println!("not verified: {e}");
            Ok(false)
        }
    }
}
