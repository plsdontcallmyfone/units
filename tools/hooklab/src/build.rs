//! The build step: `cargo build-sbf` with the toolchain the units programs pin (platform-tools
//! v1.57, SBF v3), `--locked`, into a directory of the lab's choosing. A stack frame over 4,096
//! bytes fails the build, as it does for the protocol's own programs. The crate must carry its
//! `Cargo.lock`.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

/// Platform tools the units programs build with (scripts/solana/programs.sh).
pub const TOOLS_VERSION: &str = "v1.57";
/// SBF architecture.
pub const SBF_ARCH: &str = "v3";

/// What the build produced.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Built {
    /// The `.so`.
    #[serde(skip)]
    pub so: PathBuf,
    /// `cargo build-sbf --version`.
    pub toolchain: String,
    /// Platform tools and architecture flags.
    pub tools_version: String,
    pub arch: String,
    /// Cargo features the build enabled.
    pub features: Vec<String>,
}

/// The crate's library name (`[lib] name`, else the package name with `-` as `_`).
pub fn lib_name(crate_dir: &Path) -> Result<String, String> {
    let text = std::fs::read_to_string(crate_dir.join("Cargo.toml")).map_err(|e| format!("Cargo.toml: {e}"))?;
    let mut section = String::new();
    let mut package = None;
    let mut lib = None;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            section = t.to_string();
            continue;
        }
        if let Some(rest) = t.strip_prefix("name") {
            let value = rest.trim_start().strip_prefix('=').map(|v| v.trim().trim_matches('"').to_string());
            match section.as_str() {
                "[package]" => package = value,
                "[lib]" => lib = value,
                _ => {}
            }
        }
    }
    lib.or_else(|| package.map(|p| p.replace('-', "_")))
        .filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
        .ok_or_else(|| "Cargo.toml has no usable package or lib name".into())
}

/// Builds `crate_dir` into `out_dir`.
pub fn build(crate_dir: &Path, out_dir: &Path, features: &[String]) -> Result<Built, String> {
    let name = lib_name(crate_dir)?;
    std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let mut cmd = Command::new("cargo");
    cmd.args(["build-sbf", "--tools-version", TOOLS_VERSION, "--arch", SBF_ARCH])
        .arg("--manifest-path")
        .arg(crate_dir.join("Cargo.toml"))
        .arg("--sbf-out-dir")
        .arg(out_dir);
    if !features.is_empty() {
        cmd.arg("--features").arg(features.join(","));
    }
    cmd.args(["--", "--locked"]);
    // Its own target directory (never the caller's: a `cargo test` holding that lock would wait
    // on itself). HOOKLAB_CARGO_TARGET_DIR shares one across builds.
    let target = std::env::var_os("HOOKLAB_CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| out_dir.join("target"));
    cmd.env("CARGO_TARGET_DIR", target);
    let out = cmd.output().map_err(|e| format!("cargo build-sbf: {e}"))?;
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if log.contains("overflows the maximum allowed frame space") {
        return Err("a function overflows the 4,096-byte stack frame".into());
    }
    if !out.status.success() {
        let tail: Vec<&str> = log.lines().rev().take(30).collect();
        return Err(format!(
            "cargo build-sbf failed:\n{}",
            tail.into_iter().rev().collect::<Vec<_>>().join("\n")
        ));
    }
    let so = out_dir.join(format!("{name}.so"));
    if !so.exists() {
        return Err(format!("no {} after the build", so.display()));
    }
    Ok(Built {
        so,
        toolchain: version(),
        tools_version: TOOLS_VERSION.into(),
        arch: SBF_ARCH.into(),
        features: features.to_vec(),
    })
}

/// `cargo build-sbf --version`, first line.
pub fn version() -> String {
    Command::new("cargo")
        .args(["build-sbf", "--version"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.lines().next().unwrap_or("").trim().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_example_lib_name_is_read() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/hook-template");
        assert_eq!(lib_name(&dir).unwrap(), "hook_template_example");
    }
}
