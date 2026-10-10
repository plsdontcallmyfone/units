//! units Hook Lab (tools/hooklab/README.md): the pipeline an external hook template goes through
//! before the armory admin registers it. Manifest checks, a pinned build, the LiteSVM property
//! suite against the real token program, then a signed report that carries the
//! `register_template` instruction when the template passed.

pub mod build;
pub mod manifest;
pub mod register;
pub mod report;
pub mod suite;

use std::path::Path;

use anchor_lang::prelude::Pubkey;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// sha256.
pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// Lowercase hex.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The executable hash `solana-verify get-executable-hash` prints: sha256 of the program with its
/// trailing zero bytes removed. This is the `code_hash` the armory stores (02 section 3.1).
pub fn executable_hash(so: &[u8]) -> [u8; 32] {
    let end = so.iter().rposition(|b| *b != 0).map_or(0, |i| i + 1);
    sha256(&so[..end])
}

/// A hash of a source tree: every regular file under `dir` except `target/` and dot-entries,
/// sorted by path, as `path NUL len NUL bytes`. Symlinks are refused.
pub fn tree_hash(dir: &Path) -> Result<[u8; 32], String> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) -> Result<(), String> {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .map_err(|e| format!("{}: {e}", dir.display()))?
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?;
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name == "target" {
                continue;
            }
            let ty = e.file_type().map_err(|e| e.to_string())?;
            let path = e.path();
            if ty.is_symlink() {
                return Err(format!("symlink {}", path.display()));
            } else if ty.is_dir() {
                walk(base, &path, out)?;
            } else if ty.is_file() {
                let rel = path.strip_prefix(base).map_err(|e| e.to_string())?;
                let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
                out.push((rel.to_string_lossy().replace('\\', "/"), bytes));
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    walk(dir, dir, &mut files)?;
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut h = Sha256::new();
    for (p, b) in files {
        h.update(p.as_bytes());
        h.update([0]);
        h.update((b.len() as u64).to_le_bytes());
        h.update([0]);
        h.update(&b);
    }
    Ok(h.finalize().into())
}

/// Everything `check` needs.
pub struct Job<'a> {
    pub manifest_text: &'a str,
    pub so: &'a [u8],
    pub build: Option<build::Built>,
    pub submission: String,
    pub settings: suite::Settings,
    /// The template id and the armory admin to put in the `register_template` instruction (no
    /// instruction without both).
    pub template_id: Option<u16>,
    pub admin: Option<Pubkey>,
}

/// Runs the checks and the suite; returns the unsigned report body.
pub fn check(job: Job) -> Value {
    let manifest = match manifest::Manifest::parse(job.manifest_text) {
        Ok(m) => m,
        Err(e) => return failure(&job.submission, vec![e]),
    };
    let problems = manifest.check();
    if !problems.is_empty() {
        return failure(&job.submission, problems);
    }
    let code_hash = executable_hash(job.so);
    let suite = match suite::run(&manifest, job.so, job.settings.clone()) {
        Ok(s) => s,
        Err(e) => return failure(&job.submission, vec![e]),
    };
    let passed = suite.passed();
    let register = match (passed, job.template_id, job.admin) {
        (true, Some(id), Some(admin)) => register::args(&manifest, id, code_hash)
            .and_then(|args| {
                // Protocol pass 4a: the external registration and its admin queue entry.
                let program = manifest.program().expect("checked");
                let m = register::armory_manifest(&manifest)?;
                let ix = register::instruction(admin, program, args.clone(), m);
                let queue = register::queue_instruction(admin, program, &ix);
                Ok(register::to_json_queued(&ix, &args, &m, &queue))
            })
            .unwrap_or(Value::Null),
        _ => Value::Null,
    };
    json!({
        "schema": 1,
        "submission": job.submission,
        "verdict": if passed { "pass" } else { "fail" },
        "manifest": manifest,
        "manifest_sha256": hex(&sha256(job.manifest_text.as_bytes())),
        "program_id": manifest.program_id,
        "so_len": job.so.len(),
        "code_hash": hex(&code_hash),
        "build": job.build,
        "suite": suite,
        "register_template": register,
        "requires": [
            "the program is deployed at program_id with no upgrade authority (the armory refuses an upgradeable template)",
            "the deployed executable hash equals code_hash",
        ],
    })
}

fn failure(submission: &str, problems: Vec<String>) -> Value {
    json!({
        "schema": 1,
        "submission": submission,
        "verdict": "fail",
        "problems": problems,
        "register_template": Value::Null,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executable_hash_ignores_trailing_zeros() {
        assert_eq!(executable_hash(&[1, 2, 0, 0]), sha256(&[1, 2]));
        assert_eq!(executable_hash(&[0, 1, 2]), sha256(&[0, 1, 2]));
    }

    #[test]
    fn the_tree_hash_is_stable_and_skips_target() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/hook-template");
        assert_eq!(tree_hash(&dir).unwrap(), tree_hash(&dir).unwrap());
    }

    #[test]
    fn a_bad_manifest_fails_before_the_suite() {
        let body = check(Job {
            manifest_text: "{}",
            so: &[],
            build: None,
            submission: "00".into(),
            settings: suite::Settings { seed: 1, random_sets: 0, ops: 1 },
            template_id: Some(1),
            admin: Some(Pubkey::new_unique()),
        });
        assert_eq!(body["verdict"], "fail");
        assert!(body["register_template"].is_null());
    }
}
