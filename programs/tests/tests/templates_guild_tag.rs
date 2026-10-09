// Changed by Hookwars: new file (arsenal wave B), Guild Tag (id 39, 08 section 4.7).

use anchor_lang::AnchorSerialize;
use bordrless_program_tests::armory::{items_code, params, test_schema, Hw};
use bordrless_program_tests::arsenal1::*;
use bordrless_program_tests::items::{equip, registry_extras};
use bordrless_token::client as token;
use hookwars_common::{combine, ids, template_id as T};
use hookwars_items::templates::guild_tag::{guild, GuildTouch};
use hookwars_items::ItemsError as E;
use solana_keypair::Keypair;
use solana_signer::Signer;

fn set_guild(hw: &mut Hw, caller: &Keypair, mint: &anchor_lang::prelude::Pubkey, owner: &anchor_lang::prelude::Pubkey, g: u16) -> bordrless_program_tests::env::Tx {
    let item = hw.slot_item(mint, 2);
    let mut payload = Vec::new();
    GuildTouch::SetGuild { guild: g }.serialize(&mut payload).unwrap();
    let ix = token::touch(
        caller.pubkey(),
        *mint,
        token::holding_address(mint, owner),
        ids::ITEMS_ID,
        2,
        payload,
        registry_extras(hw, mint, &item),
    );
    hw.w.env.send_paid_by(&[ix], caller, &[])
}

#[test]
fn only_the_holder_sets_their_guild_and_an_empty_holding_drops_it() {
    let mut hw = world(&[T::GUILD_TAG]);
    let t = tok(&mut hw);
    let gt = item(&mut hw, T::GUILD_TAG, &[1], 0);
    equip(&mut hw, &t.owner, &t.mint, 2, gt, vec![], 0).ok();
    let alice = hw.w.env.funded(SOL);
    let bob = hw.w.env.funded(SOL);
    hw.mint_to(&t.owner, &t.mint, &alice.pubkey(), 1_000);
    let owners = [alice.pubkey(), bob.pubkey()];

    set_guild(&mut hw, &alice, &t.mint, &alice.pubkey(), 7).ok();
    assert_eq!(guild(&range(&hw, &t.mint, &alice.pubkey(), 2)), Some(7));
    // Abuse: someone else cannot tag a holder.
    set_guild(&mut hw, &bob, &t.mint, &alice.pubkey(), 9).expect_code(items_code(E::NotHolder));
    assert_eq!(guild(&range(&hw, &t.mint, &alice.pubkey(), 2)), Some(7));
    // A partial send keeps the tag and gives the receiver none.
    send(&mut hw, &alice, &t.mint, &alice.pubkey(), &bob.pubkey(), 400).ok();
    assert_eq!(guild(&range(&hw, &t.mint, &alice.pubkey(), 2)), Some(7));
    assert_eq!(guild(&range(&hw, &t.mint, &bob.pubkey(), 2)), None);
    // Setting 0 removes it; emptying the holding drops it too.
    set_guild(&mut hw, &bob, &t.mint, &bob.pubkey(), 3).ok();
    set_guild(&mut hw, &bob, &t.mint, &bob.pubkey(), 0).ok();
    assert_eq!(guild(&range(&hw, &t.mint, &bob.pubkey(), 2)), None);
    send(&mut hw, &alice, &t.mint, &alice.pubkey(), &bob.pubkey(), 600).ok();
    assert_eq!(guild(&range(&hw, &t.mint, &alice.pubkey(), 2)), None);
    check(&hw, &t.mint, &owners);
}

#[test]
fn guild_tag_forges_only_with_the_same_version() {
    let (min, max, _, _) = test_schema(T::GUILD_TAG);
    assert_eq!(combine(T::GUILD_TAG, &min, &max, 5_000, &params(&[1]), &params(&[1])).unwrap()[0], 1);
    assert!(combine(T::GUILD_TAG, &min, &max, 5_000, &params(&[0]), &params(&[1])).is_err());
}
