// Changed by Hookwars: new file (arsenal waves D and E), one budget row per template and payout.

use anchor_lang::prelude::Pubkey;
use bordrless_program_tests::arsenal2::*;
use bordrless_program_tests::env::Tx;
use bordrless_program_tests::items::{create_holding, equip, init_ledger, put_war_config, put_war_state, raid_route, transfer};
use hookwars_common::arsenal2 as a2;
use hookwars_common::pda;
use solana_signer::Signer;

fn row(label: &str, tx: &Tx) {
    tx.ok();
    println!(
        "budget | {label} | {} keys | {} bytes | {} trace | height {} | {} CU",
        tx.keys.len(),
        tx.size,
        tx.trace_len(),
        tx.max_height(),
        tx.cu()
    );
}

#[test]
fn arsenal2_budgets() {
    // Token side.
    for (label, template, p, slot) in [
        ("transfer (send), Gift Ember", a2::GIFT_EMBER, vec![500u32], 0u8),
        ("transfer (sell), Sell Ladder", a2::SELL_LADDER, vec![1_000, 50, 300], 0),
        ("transfer (send), Loyalty Pot stamp", a2::LOYALTY_POT, vec![100, 86_400], 1),
        ("transfer (sell), Patience mark", a2::PATIENCE, vec![0, 1_000, 1], 1),
        ("transfer (send), Mercenary range", a2::MERCENARY, vec![2], 1),
    ] {
        let mut hw = world();
        let t = tok(&mut hw);
        put_war_config(&mut hw, 1);
        let it = item(&mut hw, template, &p, 0);
        equip(&mut hw, &t.owner, &t.mint, slot, it, vec![], 0).ok();
        let payer = t.owner.insecure_clone();
        init_ledger(&mut hw, &payer, &t.mint).ok();
        let a = hw.w.env.funded(SOL);
        let b = hw.w.env.funded(SOL);
        hw.mint_to(&t.owner, &t.mint, &a.pubkey(), 1_000_000);
        create_holding(&mut hw, &a, &t.mint, &b.pubkey());
        let to = if label.contains("(sell)") { t.pool.pubkey() } else { b.pubkey() };
        let tx = transfer(&mut hw, &a, &t.mint, &to, 100_000);
        row(label, &tx);
    }

    // Pool side.
    for (label, template, p, before, dir, targets) in [
        ("pool_before_swap buy, Guest List", a2::GUEST_LIST, vec![0u32, 3_600], true, 1u8, 1usize),
        ("pool_before_swap buy, Ally Pass", a2::ALLY_PASS, vec![0, 2_000], true, 1, 1),
        ("pool_before_swap buy, Embargo", a2::EMBARGO, vec![500], true, 1, 1),
        ("pool_before_swap buy, Holder Stream", a2::HOLDER_STREAM, vec![100, 200], true, 1, 0),
        ("pool_before_swap buy, Garrison", a2::GARRISON, vec![1_000], true, 1, 0),
        ("pool_after_swap sell, War Levy", a2::WAR_LEVY, vec![1, 300], false, 0, 0),
        ("pool_before_swap sell, Target Burn", a2::TARGET_BURN, vec![100, 9_000], true, 0, 0),
        ("pool_after_swap buy, Mercenary mark", a2::MERCENARY, vec![2], false, 1, 0),
        ("pool_after_swap sell, Loyalty Pot", a2::LOYALTY_POT, vec![100, 86_400], false, 0, 0),
        ("pool_before_swap buy, First Blood", a2::FIRST_BLOOD, vec![1_000, 0], true, 1, 0),
        ("pool_before_swap buy, Referral", a2::REFERRAL, vec![200], true, 1, 0),
        ("pool_after_swap sell, Patience", a2::PATIENCE, vec![0, 1_000, 1], false, 0, 0),
    ] {
        let mut hw = world();
        let t = tok(&mut hw);
        put_war_config(&mut hw, 1);
        let now = hw.w.env.now;
        put_war_state(&mut hw, &t.mint, now + 600, Pubkey::new_unique());
        set_launch_supply(&mut hw, &t.mint, 750_000_000, 250_000_000);
        let actor = hw.w.env.funded(SOL);
        let tg: Vec<Pubkey> = (0..targets).map(|_| other_token(&mut hw, &actor.pubkey(), 1_000)).collect();
        let it = item(&mut hw, template, &p, 0);
        equip(&mut hw, &t.owner, &t.mint, 1, it, tg.clone(), 0).ok();
        let payer = t.owner.insecure_clone();
        init_ledger(&mut hw, &payer, &t.mint).ok();
        hw.mint_to(&t.owner, &t.mint, &actor.pubkey(), 1_000_000_000);
        if template == a2::FIRST_BLOOD {
            hw.w.env.send_paid_by(&[init_first_blood_ix(&payer.pubkey(), &t.mint)], &payer, &[]).ok();
        }
        let mut derived = vec![];
        if template == a2::REFERRAL {
            let r = hw.w.env.funded(SOL);
            hw.w.env.send_paid_by(&[set_referrer_ix(&actor.pubkey(), &t.mint, &r.pubkey())], &actor, &[]).ok();
            derived.push((a2::pda::referred(&t.mint, &actor.pubkey()).0, true));
        }
        if let Some(m) = tg.first() {
            derived.push((pda::holding(m, &actor.pubkey()), false));
        }
        let pk = t.pool.pubkey();
        let mut call = if dir == 1 { buy(5_000_000, &t.mint, &pk, before) } else { sell(5_000_000, &t.mint, &pk, before) };
        call.actor = actor.pubkey();
        if template == a2::EMBARGO || template == a2::MERCENARY {
            let input = tg.first().copied().unwrap_or_else(Pubkey::new_unique);
            call.route = raid_route(&input, &Pubkey::new_unique(), &t.mint, 5_000_000);
        }
        let (tx, _) = pool_call_with(&mut hw, &t.mint, &pk, 1, &call, &derived);
        row(label, &tx);
    }
}
