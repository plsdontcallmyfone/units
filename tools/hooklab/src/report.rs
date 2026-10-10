//! The signed report. The body is canonical JSON (serde_json's sorted maps, no whitespace); the
//! lab key signs `DOMAIN || 0x00 || sha256(body)`, so a report signature can never be replayed as
//! any other units statement. The body names the submission by its hash only: never its author.

use std::str::FromStr;

use serde_json::{json, Value};
use solana_keypair::Keypair;
use solana_signature::Signature;
use solana_signer::Signer;

use crate::sha256;

/// Domain separator of every report signature.
pub const DOMAIN: &str = "units:hooklab:report:v1";

/// The bytes the lab key signs for `body`.
pub fn signing_bytes(body: &Value) -> Vec<u8> {
    let canonical = serde_json::to_string(body).expect("json");
    let mut v = DOMAIN.as_bytes().to_vec();
    v.push(0);
    v.extend_from_slice(&sha256(canonical.as_bytes()));
    v
}

/// Signs `body` with `key`: `{ body, signer, signature }`.
pub fn sign(body: Value, key: &Keypair) -> Value {
    let sig = key.sign_message(&signing_bytes(&body));
    json!({
        "domain": DOMAIN,
        "body": body,
        "signer": key.pubkey().to_string(),
        "signature": sig.to_string(),
    })
}

/// Checks a signed report; with `signer`, also that this key signed it. Returns the body.
pub fn verify<'a>(report: &'a Value, signer: Option<&str>) -> Result<&'a Value, String> {
    if report.get("domain").and_then(Value::as_str) != Some(DOMAIN) {
        return Err("wrong domain".into());
    }
    let body = report.get("body").ok_or("no body")?;
    let who = report.get("signer").and_then(Value::as_str).ok_or("no signer")?;
    if let Some(expected) = signer {
        if expected != who {
            return Err(format!("signed by {who}, not {expected}"));
        }
    }
    let key = anchor_lang::prelude::Pubkey::from_str(who).map_err(|_| "bad signer")?;
    let sig = report
        .get("signature")
        .and_then(Value::as_str)
        .and_then(|s| Signature::from_str(s).ok())
        .ok_or("bad signature encoding")?;
    if !sig.verify(key.as_ref(), &signing_bytes(body)) {
        return Err("signature does not match the body".into());
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_signed_report_verifies_and_any_change_breaks_it() {
        let key = Keypair::new();
        let signed = sign(json!({"verdict": "pass", "submission": "ab"}), &key);
        assert!(verify(&signed, None).is_ok());
        assert!(verify(&signed, Some(&key.pubkey().to_string())).is_ok());
        assert!(verify(&signed, Some(&Keypair::new().pubkey().to_string())).is_err());
        let mut tampered = signed.clone();
        tampered["body"]["verdict"] = json!("fail");
        assert!(verify(&tampered, None).is_err());
        let mut other_domain = signed.clone();
        other_domain["domain"] = json!("units:memo:v1");
        assert!(verify(&other_domain, None).is_err());
    }

    #[test]
    fn the_signed_bytes_are_domain_separated() {
        let b = signing_bytes(&json!({"a": 1}));
        assert!(b.starts_with(b"units:hooklab:report:v1\0"));
        assert_eq!(b.len(), DOMAIN.len() + 1 + 32);
    }
}
