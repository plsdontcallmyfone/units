// Changed by Hookwars: new file, helpers for the expansion programs (docs/spec/10-expansion.md); integration pass 3: Item.has_wear.
//! `hookwars_market` and `hookwars_social` in the LiteSVM suites: loads them into a world, sets
//! their upgrade authority to the deployer, initializes them with TEST values, and builds their
//! instructions from the Anchor account structs.

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{AccountSerialize, InstructionData, ToAccountMetas};
use bordrless_token::client as token;
use hookwars_common::{ids, pda};
use hookwars_market::state::{self as ms, MarketParams};
use hookwars_social::SocialParams;
use solana_account::Account;
use solana_keypair::Keypair;
use solana_signer::Signer;

use crate::env::{Env, Tx};
use crate::program_bytes;

/// TEST values of every market parameter (named TEST: none is a decision, 10 section 13).
pub const TEST_MARKET: MarketParams = MarketParams {
    fee_bps: 250,
    author_resale_bps: 500,
    collection_max_templates: 4,
    max_rent_bps: 5_000,
    lease_min_secs: 3_600,
    lease_max_secs: 30 * 86_400,
    commission_min_lamports: 100_000_000,
    commission_vote_secs: 7_200,
    admin_timelock_secs: 600,
    skill_min_fee_lamports: 0,
};

/// TEST values of every social parameter.
pub const TEST_SOCIAL: SocialParams = SocialParams {
    guild_max_officers: 5,
    guild_timelock_secs: 600,
    admin_timelock_secs: 600,
};

/// Loads both programs and initializes them (admin = the deployer).
pub fn load(env: &mut Env, market_treasury: Pubkey) {
    for (name, id) in [
        ("hookwars_market", hookwars_market::ID),
        ("hookwars_social", hookwars_social::ID),
    ] {
        env.svm
            .add_program(id, &program_bytes(name))
            .unwrap_or_else(|e| panic!("load {name}: {e:?}"));
        let deployer = env.deployer.pubkey();
        env.set_upgrade_authority(id, Some(deployer));
    }
    let deployer = env.deployer.insecure_clone();
    let ixs = [
        market_ix(
            hookwars_market::accounts::Init {
                authority: deployer.pubkey(),
                config: ms::config_address().0,
                program_data: hookwars_common::programdata_address(&hookwars_market::ID),
                system_program: anchor_lang::system_program::ID,
            },
            hookwars_market::instruction::Init {
                admin: deployer.pubkey(),
                treasury: market_treasury,
                params: TEST_MARKET,
            },
        ),
        social_ix(
            hookwars_social::accounts::Init {
                authority: deployer.pubkey(),
                config: social_config(),
                program_data: hookwars_common::programdata_address(&hookwars_social::ID),
                system_program: anchor_lang::system_program::ID,
            },
            hookwars_social::instruction::Init {
                admin: deployer.pubkey(),
                params: TEST_SOCIAL,
            },
        ),
    ];
    env.send(&ixs, &[&deployer]).ok();
}

/// A market instruction.
pub fn market_ix(accounts: impl ToAccountMetas, data: impl InstructionData) -> Instruction {
    Instruction {
        program_id: hookwars_market::ID,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

/// A social instruction.
pub fn social_ix(accounts: impl ToAccountMetas, data: impl InstructionData) -> Instruction {
    Instruction {
        program_id: hookwars_social::ID,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

/// `#[event_cpi]` accounts of a program.
pub fn events(program: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"__event_authority"], program).0
}

/// The social config address.
pub fn social_config() -> Pubkey {
    Pubkey::find_program_address(&[hookwars_social::seeds::CONFIG], &hookwars_social::ID).0
}

/// `list` of `item` (mint `item_mint`) by `seller`.
pub fn list_ix(seller: &Pubkey, item: &Pubkey, item_mint: &Pubkey, price: u64, expires_at: i64) -> Instruction {
    let escrow = ms::escrow_address(item_mint).0;
    market_ix(
        hookwars_market::accounts::List {
            seller: *seller,
            item: *item,
            item_mint: *item_mint,
            seller_holding: token::holding_address(item_mint, seller),
            escrow,
            escrow_holding: token::holding_address(item_mint, &escrow),
            listing: ms::Listing::address(item_mint).0,
            token_program: bordrless_token::ID,
            token_event_authority: token::event_authority(),
            system_program: anchor_lang::system_program::ID,
            event_authority: events(&hookwars_market::ID),
            program: hookwars_market::ID,
        },
        hookwars_market::instruction::List {
            price_lamports: price,
            expires_at,
        },
    )
}

/// `delist` (by the seller) or `expire` (by anyone) of `item_mint` sold by `seller`.
pub fn unlist_ix(caller: &Pubkey, seller: &Pubkey, item_mint: &Pubkey, expire: bool) -> Instruction {
    let escrow = ms::escrow_address(item_mint).0;
    let accounts = hookwars_market::accounts::Unlist {
        caller: *caller,
        seller: *seller,
        listing: ms::Listing::address(item_mint).0,
        item_mint: *item_mint,
        escrow,
        escrow_holding: token::holding_address(item_mint, &escrow),
        seller_holding: token::holding_address(item_mint, seller),
        token_program: bordrless_token::ID,
        token_event_authority: token::event_authority(),
        system_program: anchor_lang::system_program::ID,
        event_authority: events(&hookwars_market::ID),
        program: hookwars_market::ID,
    };
    if expire {
        market_ix(accounts, hookwars_market::instruction::Expire {})
    } else {
        market_ix(accounts, hookwars_market::instruction::Delist {})
    }
}

/// `buy` of `item_mint` (item `item`, author `author`, seller `seller`) by `buyer`.
#[allow(clippy::too_many_arguments)]
pub fn buy_ix(
    buyer: &Pubkey,
    seller: &Pubkey,
    author: &Pubkey,
    treasury: &Pubkey,
    item: &Pubkey,
    item_mint: &Pubkey,
    max_price: u64,
) -> Instruction {
    let escrow = ms::escrow_address(item_mint).0;
    market_ix(
        hookwars_market::accounts::Buy {
            buyer: *buyer,
            config: ms::config_address().0,
            listing: ms::Listing::address(item_mint).0,
            seller: *seller,
            author: *author,
            treasury: *treasury,
            item: *item,
            item_mint: *item_mint,
            escrow,
            escrow_holding: token::holding_address(item_mint, &escrow),
            buyer_holding: token::holding_address(item_mint, buyer),
            token_program: bordrless_token::ID,
            token_event_authority: token::event_authority(),
            system_program: anchor_lang::system_program::ID,
            event_authority: events(&hookwars_market::ID),
            program: hookwars_market::ID,
        },
        hookwars_market::instruction::Buy { max_price },
    )
}

/// `create_collection` with `template_ids` (each template account passed in order).
pub fn create_collection_ix(curator: &Pubkey, id: u32, name: &str, template_ids: Vec<u16>) -> Instruction {
    let mut ix = market_ix(
        hookwars_market::accounts::CreateCollection {
            curator: *curator,
            config: ms::config_address().0,
            collection: ms::collection_address(id).0,
            system_program: anchor_lang::system_program::ID,
            event_authority: events(&hookwars_market::ID),
            program: hookwars_market::ID,
        },
        hookwars_market::instruction::CreateCollection {
            name: name.to_string(),
            template_ids: template_ids.clone(),
        },
    );
    for t in template_ids {
        ix.accounts.push(AccountMeta::new_readonly(pda::template(t).0, false));
    }
    ix
}

/// `offer_lease`.
#[allow(clippy::too_many_arguments)]
pub fn offer_lease_ix(
    lessor: &Pubkey,
    item: &Pubkey,
    item_mint: &Pubkey,
    token_mint: &Pubkey,
    slot: u8,
    rent_bps: u16,
    fee_lamports: u64,
    term_secs: u32,
) -> Instruction {
    let le = ms::lease_escrow_address(item_mint).0;
    market_ix(
        hookwars_market::accounts::OfferLease {
            lessor: *lessor,
            config: ms::config_address().0,
            item: *item,
            item_mint: *item_mint,
            lessor_holding: token::holding_address(item_mint, lessor),
            token_mint: *token_mint,
            lease_escrow: le,
            lease_escrow_holding: token::holding_address(item_mint, &le),
            lease: ms::lease_address(item).0,
            token_program: bordrless_token::ID,
            token_event_authority: token::event_authority(),
            system_program: anchor_lang::system_program::ID,
            event_authority: events(&hookwars_market::ID),
            program: hookwars_market::ID,
        },
        hookwars_market::instruction::OfferLease {
            token_mint: *token_mint,
            slot,
            rent_bps,
            fee_lamports,
            term_secs,
        },
    )
}

/// `accept_lease`.
pub fn accept_lease_ix(payer: &Pubkey, item: &Pubkey, lessor: &Pubkey) -> Instruction {
    market_ix(
        hookwars_market::accounts::AcceptLease {
            payer: *payer,
            lease: ms::lease_address(item).0,
            lessor: *lessor,
            system_program: anchor_lang::system_program::ID,
            event_authority: events(&hookwars_market::ID),
            program: hookwars_market::ID,
        },
        hookwars_market::instruction::AcceptLease {},
    )
}

/// `withdraw_offer` (`end` false) or `end_lease` (`end` true).
pub fn close_lease_ix(caller: &Pubkey, item: &Pubkey, item_mint: &Pubkey, lessor: &Pubkey, end: bool) -> Instruction {
    let le = ms::lease_escrow_address(item_mint).0;
    let accounts = hookwars_market::accounts::CloseLease {
        caller: *caller,
        lease: ms::lease_address(item).0,
        lessor: *lessor,
        item_mint: *item_mint,
        lease_escrow: le,
        lease_escrow_holding: token::holding_address(item_mint, &le),
        lessor_holding: token::holding_address(item_mint, lessor),
        token_program: bordrless_token::ID,
        token_event_authority: token::event_authority(),
        system_program: anchor_lang::system_program::ID,
        event_authority: events(&hookwars_market::ID),
        program: hookwars_market::ID,
    };
    if end {
        market_ix(accounts, hookwars_market::instruction::EndLease {})
    } else {
        market_ix(accounts, hookwars_market::instruction::WithdrawOffer {})
    }
}

/// `open_commission`.
pub fn open_commission_ix(
    creator: &Pubkey,
    token_mint: &Pubkey,
    nonce: u64,
    slot: u8,
    bounty: u64,
    window_secs: u32,
) -> Instruction {
    let c = ms::commission_address(token_mint, nonce).0;
    market_ix(
        hookwars_market::accounts::OpenCommission {
            creator: *creator,
            config: ms::config_address().0,
            token_mint: *token_mint,
            commission: c,
            vault: ms::commission_vault_address(&c).0,
            system_program: anchor_lang::system_program::ID,
            event_authority: events(&hookwars_market::ID),
            program: hookwars_market::ID,
        },
        hookwars_market::instruction::OpenCommission {
            nonce,
            slot,
            brief_uri: "https://example.invalid/brief".to_string(),
            bounty_lamports: bounty,
            window_secs,
        },
    )
}

/// `submit`.
pub fn submit_ix(submitter: &Pubkey, commission: &Pubkey, token_mint: &Pubkey, item: &Pubkey, item_mint: &Pubkey) -> Instruction {
    market_ix(
        hookwars_market::accounts::Submit {
            submitter: *submitter,
            commission: *commission,
            token_mint: *token_mint,
            item: *item,
            item_mint: *item_mint,
            submitter_holding: token::holding_address(item_mint, submitter),
            submission: ms::submission_address(commission, item).0,
            system_program: anchor_lang::system_program::ID,
            event_authority: events(&hookwars_market::ID),
            program: hookwars_market::ID,
        },
        hookwars_market::instruction::Submit {},
    )
}

/// `pay_commission`.
pub fn pay_commission_ix(commission: &Pubkey, token_mint: &Pubkey, item: &Pubkey, submitter: &Pubkey) -> Instruction {
    market_ix(
        hookwars_market::accounts::PayCommission {
            commission: *commission,
            submission: ms::submission_address(commission, item).0,
            token_mint: *token_mint,
            submitter: *submitter,
            vault: ms::commission_vault_address(commission).0,
            system_program: anchor_lang::system_program::ID,
            event_authority: events(&hookwars_market::ID),
            program: hookwars_market::ID,
        },
        hookwars_market::instruction::PayCommission {},
    )
}

/// `refund_commission`.
pub fn refund_commission_ix(commission: &Pubkey, creator: &Pubkey) -> Instruction {
    market_ix(
        hookwars_market::accounts::RefundCommission {
            config: ms::config_address().0,
            commission: *commission,
            creator: *creator,
            vault: ms::commission_vault_address(commission).0,
            system_program: anchor_lang::system_program::ID,
            event_authority: events(&hookwars_market::ID),
            program: hookwars_market::ID,
        },
        hookwars_market::instruction::RefundCommission {},
    )
}

/// `create_badge` by the admin (the deployer); `id` is the next badge id.
pub fn create_badge_ix(admin: &Pubkey, id: u32, name: &str, criterion: hookwars_social::Criterion) -> Instruction {
    let badge = Pubkey::find_program_address(&[hookwars_social::seeds::BADGE, &id.to_le_bytes()], &hookwars_social::ID).0;
    social_ix(
        hookwars_social::accounts::CreateBadge {
            admin: *admin,
            config: social_config(),
            badge,
            badge_mint: hookwars_social::badge_mint_address(id).0,
            token_program: bordrless_token::ID,
            token_event_authority: token::event_authority(),
            system_program: anchor_lang::system_program::ID,
            event_authority: events(&hookwars_social::ID),
            program: hookwars_social::ID,
        },
        hookwars_social::instruction::CreateBadge {
            name: name.to_string(),
            criterion,
        },
    )
}

/// `claim_badge` of badge `id` for `recipient`; `extra` are the criterion's accounts.
pub fn claim_badge_ix(claimant: &Pubkey, id: u32, recipient: &Pubkey, extra: Vec<AccountMeta>) -> Instruction {
    let badge = Pubkey::find_program_address(&[hookwars_social::seeds::BADGE, &id.to_le_bytes()], &hookwars_social::ID).0;
    let award = Pubkey::find_program_address(
        &[hookwars_social::seeds::AWARD, &id.to_le_bytes(), recipient.as_ref()],
        &hookwars_social::ID,
    )
    .0;
    let mint = hookwars_social::badge_mint_address(id).0;
    let mut ix = social_ix(
        hookwars_social::accounts::ClaimBadge {
            claimant: *claimant,
            badge,
            award,
            recipient: *recipient,
            badge_mint: mint,
            recipient_holding: token::holding_address(&mint, recipient),
            minter: hookwars_social::minter_address().0,
            token_program: bordrless_token::ID,
            token_event_authority: token::event_authority(),
            system_program: anchor_lang::system_program::ID,
            event_authority: events(&hookwars_social::ID),
            program: hookwars_social::ID,
        },
        hookwars_social::instruction::ClaimBadge {
            badge_id: id,
            recipient_key: *recipient,
        },
    );
    ix.accounts.extend(extra);
    ix
}

/// The guild `id`'s address.
pub fn guild_address(id: u32) -> Pubkey {
    Pubkey::find_program_address(&[hookwars_social::seeds::GUILD, &id.to_le_bytes()], &hookwars_social::ID).0
}

/// The guild action `nonce` of guild `id`.
pub fn action_address(id: u32, nonce: u64) -> Pubkey {
    Pubkey::find_program_address(
        &[hookwars_social::seeds::ACTION, &id.to_le_bytes(), &nonce.to_le_bytes()],
        &hookwars_social::ID,
    )
    .0
}

/// `create_guild` (`id` is the next guild id).
pub fn create_guild_ix(founder: &Pubkey, id: u32, name: &str) -> Instruction {
    social_ix(
        hookwars_social::accounts::CreateGuild {
            founder: *founder,
            config: social_config(),
            guild: guild_address(id),
            system_program: anchor_lang::system_program::ID,
            event_authority: events(&hookwars_social::ID),
            program: hookwars_social::ID,
        },
        hookwars_social::instruction::CreateGuild {
            name: name.to_string(),
        },
    )
}

/// `deposit_sol`.
pub fn deposit_sol_ix(from: &Pubkey, id: u32, lamports: u64) -> Instruction {
    social_ix(
        hookwars_social::accounts::DepositSol {
            from: *from,
            guild: guild_address(id),
            treasury: hookwars_social::treasury_address(id).0,
            system_program: anchor_lang::system_program::ID,
            event_authority: events(&hookwars_social::ID),
            program: hookwars_social::ID,
        },
        hookwars_social::instruction::DepositSol { lamports },
    )
}

/// `propose_action` (`nonce` is the guild's next action).
pub fn propose_action_ix(officer: &Pubkey, id: u32, nonce: u64, kind: hookwars_social::GuildActionKind) -> Instruction {
    social_ix(
        hookwars_social::accounts::ProposeAction {
            officer: *officer,
            config: social_config(),
            guild: guild_address(id),
            action: action_address(id, nonce),
            system_program: anchor_lang::system_program::ID,
            event_authority: events(&hookwars_social::ID),
            program: hookwars_social::ID,
        },
        hookwars_social::instruction::ProposeAction { kind },
    )
}

/// `approve_action`.
pub fn approve_action_ix(officer: &Pubkey, id: u32, nonce: u64) -> Instruction {
    social_ix(
        hookwars_social::accounts::ApproveAction {
            officer: *officer,
            guild: guild_address(id),
            action: action_address(id, nonce),
            event_authority: events(&hookwars_social::ID),
            program: hookwars_social::ID,
        },
        hookwars_social::instruction::ApproveAction {},
    )
}

/// `execute_action`; `to` is the recipient (any account for `SetOfficers`), `extra` the token
/// accounts of a `SpendToken` (mint, treasury holding, recipient holding, then slot slices).
pub fn execute_action_ix(id: u32, nonce: u64, to: &Pubkey, extra: Vec<AccountMeta>) -> Instruction {
    let mut ix = social_ix(
        hookwars_social::accounts::ExecuteAction {
            guild: guild_address(id),
            action: action_address(id, nonce),
            treasury: hookwars_social::treasury_address(id).0,
            to: *to,
            token_program: bordrless_token::ID,
            token_event_authority: token::event_authority(),
            system_program: anchor_lang::system_program::ID,
            event_authority: events(&hookwars_social::ID),
            program: hookwars_social::ID,
        },
        hookwars_social::instruction::ExecuteAction {},
    );
    ix.accounts.extend(extra);
    ix
}

/// Writes an Anchor account at `key` owned by `owner`.
pub fn put_anchor<T: AccountSerialize>(env: &mut Env, key: Pubkey, owner: Pubkey, value: &T, min_len: usize) {
    let mut data = Vec::new();
    value.try_serialize(&mut data).expect("serialize");
    data.resize(data.len().max(min_len), 0);
    let lamports = env.rent(data.len());
    env.put(
        key,
        Account {
            lamports,
            data,
            owner,
            executable: false,
            rent_epoch: 0,
        },
    );
}

/// Writes an armory `Item` of `template_id` at the address the armory gives `item_mint`.
pub fn put_item(env: &mut Env, item_mint: Pubkey, template_id: u16, level: u8) -> Pubkey {
    let key = pda::item(&item_mint).0;
    let it = hookwars_armory::state::Item {
        version: 1,
        bump: pda::item(&item_mint).1,
        item_mint,
        template_id,
        params: [0; hookwars_common::PARAM_FIELDS],
        manifest: hookwars_common::Manifest::default(),
        author: Pubkey::default(),
        royalty_bps: 0,
        level,
        source: 0,
        equipped_count: 1,
        royalty_owner_bump: 0,
        created_at: 0,
        has_wear: false,
        access_mode: 0,
        exclusive: false,
        reserved: [0; 29],
    };
    put_anchor(env, key, ids::ARMORY_ID, &it, 0);
    key
}

/// Sends `ixs` paid by `payer` and returns the transaction.
pub fn send(env: &mut Env, payer: &Keypair, ixs: &[Instruction]) -> Tx {
    env.send_paid_by(ixs, payer, &[])
}

/// Anchor's custom error code of a market error.
pub fn market_code(e: hookwars_market::error::MarketError) -> u32 {
    anchor_lang::error::ERROR_CODE_OFFSET + e as u32
}

/// Anchor's custom error code of a social error.
pub fn social_code(e: hookwars_social::SocialError) -> u32 {
    anchor_lang::error::ERROR_CODE_OFFSET + e as u32
}
