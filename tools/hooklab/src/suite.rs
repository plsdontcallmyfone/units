//! The property suite: the submitted program, loaded at its id into LiteSVM next to the real token
//! program, equipped in an item slot of a fresh slot mint (the armory stub signs as the mint's slot
//! authority), then driven by seeded random transfers and burns. Every answer goes through the
//! token program's own checks; the lab adds the ones the token program cannot make (a refusal the
//! manifest denies, compute over the declared bound, conservation, holdings that cannot close).

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{Discriminator, Space};
use bordrless_hook::{equip_rule, slot_kind};
use bordrless_program_tests::env::Tx;
use bordrless_program_tests::fixture::World;
use bordrless_program_tests::slots::{as_armory, authority_of, item_slot};
use bordrless_token::client as token;
use bordrless_token::slots::{is_called, SlotOp};
use bordrless_token::state::Mint;
use hookwars_armory::state::Item;
use hookwars_common::{ids, PARAM_FIELDS};
use serde::Serialize;
use solana_account::Account;
use solana_keypair::Keypair;
use solana_signer::Signer;

use crate::manifest::Manifest;

/// Byte offset of `params` in an armory `Item` (discriminator 8, version 1, bump 1, item_mint 32,
/// template_id 2).
pub const ITEM_PARAMS_OFFSET: usize = 44;
/// The slot the template is equipped in.
const SLOT: u8 = 0;
/// Wallets in each run.
const WALLETS: usize = 4;
/// Tokens minted to each wallet (6 decimals).
const START: u64 = 1_000_000_000;

/// Suite settings.
#[derive(Clone, Debug, Serialize)]
pub struct Settings {
    /// Seed of every random choice.
    pub seed: u64,
    /// Random parameter sets after the minimum and the maximum.
    pub random_sets: u32,
    /// Operations per parameter set.
    pub ops: u32,
}

/// What went wrong, by class.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Violation {
    /// `over_cut`, `bad_answer`, `refusal`, `cu_over_declared`, `cu_blowup`, `conservation`,
    /// `not_closeable`, `equip`, `no_transfers`, `other`.
    pub class: String,
    /// The parameter set.
    pub params: Vec<u32>,
    /// What happened.
    pub detail: String,
}

/// One parameter set's run.
#[derive(Clone, Debug, Serialize)]
pub struct SetResult {
    pub params: Vec<u32>,
    pub transfers_ok: u32,
    pub burns_ok: u32,
    pub refusals: u32,
    pub cut_total: u64,
    pub max_cu_per_call: u64,
    pub closed_holdings: u32,
}

/// The suite's outcome.
#[derive(Clone, Debug, Serialize)]
pub struct SuiteResult {
    pub settings: Settings,
    pub sets: Vec<SetResult>,
    pub violations: Vec<Violation>,
    /// The highest compute per callback seen (transaction minus the same transfer with the slot
    /// empty: it includes the token program's slot dispatch, so it overstates the template).
    pub max_cu_per_call: u64,
}

impl SuiteResult {
    /// No violation.
    pub fn passed(&self) -> bool {
        self.violations.is_empty()
    }
}

/// xorshift64*: deterministic, no dependency.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed ^ 0x9E37_79B9_7F4A_7C15 | 1)
    }
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: u64, hi: u64) -> u64 {
        if hi <= lo {
            return lo;
        }
        lo + self.next() % (hi - lo + 1)
    }
}

/// The token error names a broken answer produces.
const BAD_ANSWER: [&str; 6] = [
    "SlotDataLength",
    "UnsupportedHookReturn",
    "InvalidHookReturn",
    "TooManyDeltas",
    "ZeroDelta",
    "WrongEquipVault",
];

/// Classifies a failed transaction by its logs.
pub fn classify(logs: &[String], program: &Pubkey) -> (&'static str, String) {
    let code = logs
        .iter()
        .find_map(|l| l.split("Error Code: ").nth(1))
        .map(|s| s.split('.').next().unwrap_or("").to_string());
    if let Some(c) = &code {
        if c == "SlotCutExceeded" {
            return ("over_cut", c.clone());
        }
        if BAD_ANSWER.contains(&c.as_str()) {
            return ("bad_answer", c.clone());
        }
    }
    if logs
        .iter()
        .any(|l| l.contains("exceeded CUs meter") || l.contains("ComputationalBudgetExceeded"))
    {
        return ("cu_blowup", "compute budget exceeded".into());
    }
    let failed = format!("Program {program} failed");
    if logs.iter().any(|l| l.starts_with(&failed)) {
        return ("refusal", code.unwrap_or_else(|| "template failed".into()));
    }
    ("other", code.unwrap_or_else(|| logs.last().cloned().unwrap_or_default()))
}

/// The parameter sets: all minimums, all maximums, then random ones inside the bounds.
pub fn param_sets(m: &Manifest, rng: &mut Rng, random: u32) -> Vec<Vec<u32>> {
    let n = usize::from(m.params.field_count);
    let mut v = vec![m.params.field_min.clone(), m.params.field_max.clone()];
    for _ in 0..random {
        v.push(
            (0..n)
                .map(|i| rng.range(u64::from(m.params.field_min[i]), u64::from(m.params.field_max[i])) as u32)
                .collect(),
        );
    }
    v.dedup();
    v
}

/// Runs the suite on `so` (the template's built program) for `manifest`.
pub fn run(manifest: &Manifest, so: &[u8], settings: Settings) -> Result<SuiteResult, String> {
    let program = manifest.program()?;
    let mut rng = Rng::new(settings.seed);
    let mut sets = Vec::new();
    let mut violations = Vec::new();
    for params in param_sets(manifest, &mut rng, settings.random_sets) {
        let (set, mut v) = run_set(manifest, so, program, &params, &mut rng, settings.ops)?;
        sets.push(set);
        violations.append(&mut v);
    }
    let max_cu_per_call = sets.iter().map(|s| s.max_cu_per_call).max().unwrap_or(0);
    Ok(SuiteResult {
        settings,
        sets,
        violations,
        max_cu_per_call,
    })
}

/// The test world of one parameter set.
struct Lab {
    w: World,
    program: Pubkey,
    mint: Pubkey,
    item: Pubkey,
    vault: Option<Pubkey>,
    vault_owner: Option<Pubkey>,
    wallets: Vec<Keypair>,
    extras: Vec<AccountMeta>,
}

impl Lab {
    fn send(&mut self, ixs: &[Instruction], payer: &Keypair) -> Tx {
        self.w.env.svm.expire_blockhash();
        self.w.env.send_paid_by(ixs, payer, &[])
    }

    fn slice(&self, op: SlotOp) -> Vec<AccountMeta> {
        let m: Mint = self.w.env.read(&self.mint);
        let s = &m.active_slots()[usize::from(SLOT)];
        if is_called(s, op) {
            token::slot_slice(self.program, self.extras.clone())
        } else {
            Vec::new()
        }
    }

    fn holding(&self, owner: &Pubkey) -> u64 {
        self.w.env.holding(&self.mint, owner)
    }

    fn supply(&self) -> u64 {
        self.w.env.read::<Mint>(&self.mint).supply
    }

    fn vault_balance(&self) -> u64 {
        self.vault_owner.map_or(0, |o| self.holding(&o))
    }

    fn transfer_ix(&self, from: &Pubkey, to: &Pubkey, amount: u64, op: SlotOp) -> Instruction {
        token::transfer_with(
            *from,
            token::holding_address(&self.mint, from),
            token::holding_address(&self.mint, to),
            self.mint,
            None,
            if op == SlotOp::Transfer { self.slice(SlotOp::Transfer) } else { Vec::new() },
            amount,
        )
    }
}

fn item_account(params: &[u32]) -> Account {
    let len = 8 + Item::INIT_SPACE;
    let mut data = vec![0u8; len];
    data[..8].copy_from_slice(Item::DISCRIMINATOR);
    data[8] = 1;
    for (i, p) in params.iter().take(PARAM_FIELDS).enumerate() {
        let at = ITEM_PARAMS_OFFSET + 4 * i;
        data[at..at + 4].copy_from_slice(&p.to_le_bytes());
    }
    Account {
        lamports: 10_000_000,
        data,
        owner: ids::ARMORY_ID,
        executable: false,
        rent_epoch: 0,
    }
}

fn kind_of(m: &Manifest) -> u8 {
    match m.kind.as_str() {
        "reward" => slot_kind::REWARD,
        "defense" => slot_kind::DEFENSE,
        "relation" => slot_kind::RELATION,
        "pool" => slot_kind::POOL,
        _ => slot_kind::FEE,
    }
}

fn setup(m: &Manifest, so: &[u8], program: Pubkey, params: &[u32]) -> Result<(Lab, u64), String> {
    let mut w = World::with_slots();
    w.env
        .svm
        .add_program(program, so)
        .map_err(|e| format!("the program does not load: {e:?}"))?;
    let owner = w.env.funded(100_000_000_000);
    let mint_kp = Keypair::new();
    let mint = mint_kp.pubkey();
    let mut slot = item_slot(kind_of(m), equip_rule::LOCKED, m.max_cut_transfer_bps, m.data_len(), m.answers_touch);
    slot.bounds.may_refuse = m.may_refuse;
    slot.bounds.may_burn = m.may_burn;
    if let Err(f) = w.create_slot_mint(&owner, &mint_kp, vec![slot], true).result {
        return Err(format!("create_slot_mint: {:?}\n{}", f.err, f.meta.logs.join("\n")));
    }
    let item = Pubkey::new_unique();
    w.env.put(item, item_account(params));
    let (vault, vault_owner) = if m.cuts() {
        let (o, h) = bordrless_program_tests::slots::equip_vault(&mint, SLOT);
        w.make_equip_vault(&owner, &mint, SLOT);
        (Some(h), Some(o))
    } else {
        (None, None)
    };
    let extras: Vec<AccountMeta> = m
        .extras
        .iter()
        .map(|x| match x.as_str() {
            "item" => AccountMeta::new_readonly(item, false),
            _ => AccountMeta::new(vault.expect("cuts"), false),
        })
        .collect();
    let wallets: Vec<Keypair> = (0..WALLETS).map(|_| w.env.funded(10_000_000_000)).collect();
    let mut lab = Lab {
        w,
        program,
        mint,
        item,
        vault,
        vault_owner,
        wallets,
        extras,
    };
    for i in 0..WALLETS {
        let k = lab.wallets[i].pubkey();
        let ixs = [
            token::create_holding(owner.pubkey(), mint, k),
            token::mint_to_with(owner.pubkey(), mint, token::holding_address(&mint, &k), None, vec![], START),
        ];
        if let Err(f) = lab.send(&ixs, &owner).result {
            return Err(format!("mint_to: {:?}\n{}", f.err, f.meta.logs.join("\n")));
        }
    }
    // Baseline: the same transfer with the slot still empty.
    let (a, b) = (lab.wallets[0].insecure_clone(), lab.wallets[1].pubkey());
    let base = lab.send(&[lab.transfer_ix(&a.pubkey(), &b, 1_000, SlotOp::Burn)], &a);
    let baseline = match &base.result {
        Ok(meta) => meta.compute_units_consumed,
        Err(f) => return Err(format!("baseline transfer: {:?}", f.err)),
    };
    let ix = token::set_slot_item(
        authority_of(&mint),
        mint,
        program,
        lab.vault,
        SLOT,
        lab.item,
        m.flags(),
        0,
        lab.extras.len() as u8,
    );
    let tx = lab.send(&[as_armory(&mint, ix)], &owner);
    if let Err(f) = tx.result {
        return Err(format!("equip: {:?}\n{}", f.err, f.meta.logs.join("\n")));
    }
    Ok((lab, baseline))
}

fn run_set(
    m: &Manifest,
    so: &[u8],
    program: Pubkey,
    params: &[u32],
    rng: &mut Rng,
    ops: u32,
) -> Result<(SetResult, Vec<Violation>), String> {
    let mut v = Vec::new();
    let mut add = |class: &str, detail: String, v: &mut Vec<Violation>| {
        if v.len() < 32 {
            v.push(Violation {
                class: class.into(),
                params: params.to_vec(),
                detail,
            });
        }
    };
    let (mut lab, baseline) = match setup(m, so, program, params) {
        Ok(x) => x,
        Err(e) => {
            add("equip", e, &mut v);
            return Ok((empty_set(params), v));
        }
    };
    let mut r = SetResult {
        params: params.to_vec(),
        transfers_ok: 0,
        burns_ok: 0,
        refusals: 0,
        cut_total: 0,
        max_cu_per_call: 0,
        closed_holdings: 0,
    };
    for _ in 0..ops {
        let i = rng.range(0, WALLETS as u64 - 1) as usize;
        let from = lab.wallets[i].insecure_clone();
        let balance = lab.holding(&from.pubkey());
        if balance == 0 {
            continue;
        }
        // One op in six empties the sender; the rest move a random part.
        let amount = if rng.range(0, 5) == 0 { balance } else { rng.range(1, balance) };
        let is_burn = rng.range(0, 7) == 0;
        let supply0 = lab.supply();
        let vault0 = lab.vault_balance();
        if is_burn {
            let ix = token::burn_with(
                from.pubkey(),
                token::holding_address(&lab.mint, &from.pubkey()),
                lab.mint,
                None,
                lab.slice(SlotOp::Burn),
                amount,
            );
            let tx = lab.send(&[ix], &from);
            match &tx.result {
                Ok(_) => {
                    r.burns_ok += 1;
                    if lab.supply() + amount != supply0 {
                        add("conservation", format!("burn of {amount} moved supply by {}", supply0 - lab.supply()), &mut v);
                    }
                }
                Err(_) => failed(m, &tx, &program, &mut r, &mut v, &mut add, "burn", amount),
            }
        } else {
            let mut j = rng.range(0, WALLETS as u64 - 2) as usize;
            if j >= i {
                j += 1;
            }
            let to = lab.wallets[j].pubkey();
            let to0 = lab.holding(&to);
            let ix = lab.transfer_ix(&from.pubkey(), &to, amount, SlotOp::Transfer);
            let tx = lab.send(&[ix], &from);
            match &tx.result {
                Ok(meta) => {
                    r.transfers_ok += 1;
                    let received = lab.holding(&to) - to0;
                    let cut = amount.saturating_sub(received);
                    let vault_in = lab.vault_balance() - vault0;
                    r.cut_total += cut;
                    if cut != vault_in {
                        add("conservation", format!("cut {cut} but the equip vault got {vault_in}"), &mut v);
                    }
                    if u128::from(cut) * 10_000 > u128::from(amount) * u128::from(m.max_cut_transfer_bps) {
                        add("over_cut", format!("cut {cut} of {amount} passed the token checks"), &mut v);
                    }
                    let cu = meta.compute_units_consumed.saturating_sub(baseline);
                    r.max_cu_per_call = r.max_cu_per_call.max(cu);
                    if cu > m.max_cu_per_call {
                        add("cu_over_declared", format!("{cu} CU per call, declared {}", m.max_cu_per_call), &mut v);
                    }
                }
                Err(_) => failed(m, &tx, &program, &mut r, &mut v, &mut add, "transfer", amount),
            }
        }
        let held: u64 = lab.wallets.iter().map(|k| lab.holding(&k.pubkey())).sum::<u64>() + lab.vault_balance();
        if held != lab.supply() {
            add("conservation", format!("supply {} but holdings sum to {held}", lab.supply()), &mut v);
        }
    }
    // Empty one wallet into another (through the template), then close every empty holding.
    let last = lab.wallets[WALLETS - 1].insecure_clone();
    let left = lab.holding(&last.pubkey());
    if left > 0 {
        let ix = lab.transfer_ix(&last.pubkey(), &lab.wallets[0].pubkey(), left, SlotOp::Transfer);
        let tx = lab.send(&[ix], &last);
        if tx.result.is_err() {
            failed(m, &tx, &program, &mut r, &mut v, &mut add, "emptying transfer", left);
        }
    }
    for k in &lab.wallets.iter().map(|k| k.insecure_clone()).collect::<Vec<_>>() {
        if lab.holding(&k.pubkey()) != 0 {
            continue;
        }
        let h = token::holding_address(&lab.mint, &k.pubkey());
        let tx = lab.send(&[token::close_holding(k.pubkey(), lab.mint, h, k.pubkey())], k);
        match &tx.result {
            Ok(_) => r.closed_holdings += 1,
            Err(_) => {
                let (_, code) = classify(tx.logs(), &program);
                add("not_closeable", format!("an empty holding does not close: {code}"), &mut v);
            }
        }
    }
    if r.transfers_ok == 0 {
        add("no_transfers", "no transfer went through".into(), &mut v);
    }
    Ok((r, v))
}

#[allow(clippy::too_many_arguments)]
fn failed(
    m: &Manifest,
    tx: &Tx,
    program: &Pubkey,
    r: &mut SetResult,
    v: &mut Vec<Violation>,
    add: &mut impl FnMut(&str, String, &mut Vec<Violation>),
    what: &str,
    amount: u64,
) {
    let (class, code) = classify(tx.logs(), program);
    if class == "refusal" {
        r.refusals += 1;
        if m.may_refuse {
            return;
        }
        add("refusal", format!("{what} of {amount} refused ({code}) but the manifest says may_refuse false"), v);
        return;
    }
    add(class, format!("{what} of {amount}: {code}"), v);
}

fn empty_set(params: &[u32]) -> SetResult {
    SetResult {
        params: params.to_vec(),
        transfers_ok: 0,
        burns_ok: 0,
        refusals: 0,
        cut_total: 0,
        max_cu_per_call: 0,
        closed_holdings: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_item_layout_offset_matches_the_armory() {
        // version, bump, item_mint, template_id come before params.
        assert_eq!(ITEM_PARAMS_OFFSET, 8 + 1 + 1 + 32 + 2);
        let a = item_account(&[7, 9]);
        assert_eq!(&a.data[44..48], &7u32.to_le_bytes());
        assert_eq!(&a.data[48..52], &9u32.to_le_bytes());
        assert_eq!(a.data.len(), 8 + Item::INIT_SPACE);
    }

    #[test]
    fn classification_reads_the_token_error_names() {
        let p = Pubkey::new_unique();
        let logs = |s: &str| vec![format!("Program log: AnchorError occurred. Error Code: {s}. Error Number: 6000.")];
        assert_eq!(classify(&logs("SlotCutExceeded"), &p).0, "over_cut");
        assert_eq!(classify(&logs("SlotDataLength"), &p).0, "bad_answer");
        assert_eq!(classify(&logs("WrongEquipVault"), &p).0, "bad_answer");
        let refused = vec![
            "Program log: AnchorError occurred. Error Code: Refused. Error Number: 6003.".to_string(),
            format!("Program {p} failed: custom program error: 0x1773"),
        ];
        assert_eq!(classify(&refused, &p).0, "refusal");
        assert_eq!(
            classify(&[format!("Program {p} failed: exceeded CUs meter at BPF instruction")], &p).0,
            "cu_blowup"
        );
        assert_eq!(classify(&["Program x failed: something".into()], &p).0, "other");
    }

    #[test]
    fn param_sets_start_at_the_bounds_and_stay_inside() {
        let m = Manifest::parse(include_str!("../../../examples/hook-template/hooklab.json")).unwrap();
        let sets = param_sets(&m, &mut Rng::new(1), 20);
        assert_eq!(sets[0], vec![0]);
        assert_eq!(sets[1], vec![500]);
        assert!(sets.iter().all(|s| s[0] <= 500));
        let again = param_sets(&m, &mut Rng::new(1), 20);
        assert_eq!(sets, again);
    }
}
