// Changed by Hookwars: new file (M3b).
//! Treaty (id 5) and Tribute (id 6), 04 sections 3.5 and 3.6. Extras: the partner's `Mint`, then
//! the partner's `EquipState` of every slot. Active only while the partner equips the same item
//! aimed back at us (Tribute: with the opposite role). Cuts go to the partner's treaty inbox.

use anchor_lang::prelude::*;
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::PoolHookArgs;
use hookwars_common::{pda, template_id as t};

use super::{fee, Env, PoolOut, BUY};
use crate::EquipState;

/// Whether the partner equips `item` aimed at `mint`, and its role.
pub fn partner_role(extras: &[AccountInfo], partner: &Pubkey, item: &Pubkey, mint: &Pubkey) -> Option<u8> {
    let m = extras.first()?;
    if m.key != partner || *m.owner != bordrless_token::ID {
        return None;
    }
    let mint_data = bordrless_token::state::Mint::try_deserialize(&mut &m.try_borrow_data().ok()?[..]).ok()?;
    let slot = mint_data.slots.iter().position(|s| s.item == *item)? as u8;
    let es = extras.get(1 + usize::from(slot))?;
    if *es.key != pda::equip_state(partner, slot).0 || *es.owner != crate::ID {
        return None;
    }
    let s = EquipState::try_deserialize(&mut &es.try_borrow_data().ok()?[..]).ok()?;
    (s.item == *item && s.config.targets.first() == Some(mint)).then_some(s.config.role)
}

/// `pool_before_swap` on a buy.
pub fn pool(template: u16, env: &Env, args: &PoolHookArgs, ctx: &ItemPoolContext, before: bool) -> Result<PoolOut> {
    if !before || args.direction != BUY {
        return Ok(PoolOut::default());
    }
    let Some(partner) = env.targets.first().copied() else {
        return Ok(PoolOut::default());
    };
    let Some(their_role) = partner_role(env.extras, &partner, &env.item, &env.mint) else {
        return Ok(PoolOut::default());
    };
    let bps = match template {
        t::TREATY => {
            if env.mint.to_bytes() < partner.to_bytes() {
                env.params[0]
            } else {
                env.params[1]
            }
        }
        _ => {
            if env.role == 1 && their_role == 2 {
                env.params[0]
            } else {
                0
            }
        }
    };
    Ok(PoolOut {
        cut: fee(ctx.side_amount, bps),
        ..Default::default()
    })
}
