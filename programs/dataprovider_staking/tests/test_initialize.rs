//! Tests for `initialize` and admin rotation flow.

mod common;

use {
    common::*,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
};

// Anchor custom error codes start at 6000.
const ERR_UNAUTHORIZED: u32 = 6001;
const ERR_NOT_PENDING_ADMIN: u32 = 6002;
const ERR_NO_PENDING_ADMIN: u32 = 6003;

#[test]
fn initialize_sets_admin_and_usdc_mint() {
    let mut env = Env::new();
    let usdc = env.create_mint(6);
    let admin = env.fresh_user(1_000_000_000);

    let payer = env.payer.insecure_clone();
    env.send(
        &[ix_initialize(&payer.pubkey(), &admin.pubkey(), &usdc)],
        &[&payer, &admin],
    )
    .expect("initialize ok");

    let cfg = env.fetch_config();
    assert_eq!(cfg.admin, admin.pubkey());
    assert_eq!(cfg.pending_admin, Pubkey::default());
    assert_eq!(cfg.usdc_mint, usdc);
    assert_eq!(cfg.pool_count, 0);
}

#[test]
fn admin_rotation_two_step() {
    let mut env = Env::new();
    let usdc = env.create_mint(6);
    let admin = env.fresh_user(1_000_000_000);
    let new_admin = env.fresh_user(1_000_000_000);

    let payer = env.payer.insecure_clone();
    env.send(
        &[ix_initialize(&payer.pubkey(), &admin.pubkey(), &usdc)],
        &[&payer, &admin],
    )
    .unwrap();

    // Propose
    env.send(
        &[ix_propose_admin(&admin.pubkey(), &new_admin.pubkey())],
        &[&payer, &admin],
    )
    .expect("propose ok");
    assert_eq!(env.fetch_config().pending_admin, new_admin.pubkey());
    // Current admin unchanged.
    assert_eq!(env.fetch_config().admin, admin.pubkey());

    // Non-pending pubkey trying to accept must fail.
    let interloper = env.fresh_user(1_000_000_000);
    let res = env.send(
        &[ix_accept_admin(&interloper.pubkey())],
        &[&payer, &interloper],
    );
    assert_error(res, ERR_NOT_PENDING_ADMIN);

    // New admin accepts -> takes over.
    env.send(
        &[ix_accept_admin(&new_admin.pubkey())],
        &[&payer, &new_admin],
    )
    .expect("accept ok");
    let cfg = env.fetch_config();
    assert_eq!(cfg.admin, new_admin.pubkey());
    assert_eq!(cfg.pending_admin, Pubkey::default());
}

#[test]
fn admin_can_cancel_proposal() {
    let mut env = Env::new();
    let usdc = env.create_mint(6);
    let admin = env.fresh_user(1_000_000_000);
    let new_admin = env.fresh_user(1_000_000_000);

    let payer = env.payer.insecure_clone();
    env.send(
        &[ix_initialize(&payer.pubkey(), &admin.pubkey(), &usdc)],
        &[&payer, &admin],
    )
    .unwrap();

    env.send(
        &[ix_propose_admin(&admin.pubkey(), &new_admin.pubkey())],
        &[&payer, &admin],
    )
    .unwrap();
    env.send(
        &[ix_cancel_admin_proposal(&admin.pubkey())],
        &[&payer, &admin],
    )
    .expect("cancel ok");

    let cfg = env.fetch_config();
    assert_eq!(cfg.pending_admin, Pubkey::default());
    assert_eq!(cfg.admin, admin.pubkey());

    // After cancel, the previously-proposed admin cannot accept.
    let res = env.send(&[ix_accept_admin(&new_admin.pubkey())], &[&payer, &new_admin]);
    assert_error(res, ERR_NO_PENDING_ADMIN);
}

#[test]
fn non_admin_cannot_propose() {
    let mut env = Env::new();
    let usdc = env.create_mint(6);
    let admin = env.fresh_user(1_000_000_000);
    let attacker = env.fresh_user(1_000_000_000);

    let payer = env.payer.insecure_clone();
    env.send(
        &[ix_initialize(&payer.pubkey(), &admin.pubkey(), &usdc)],
        &[&payer, &admin],
    )
    .unwrap();
    let res = env.send(
        &[ix_propose_admin(&attacker.pubkey(), &attacker.pubkey())],
        &[&payer, &attacker],
    );
    assert_error(res, ERR_UNAUTHORIZED);
}

#[test]
fn accept_admin_fails_when_no_proposal() {
    let mut env = Env::new();
    let usdc = env.create_mint(6);
    let admin = env.fresh_user(1_000_000_000);
    let rando = env.fresh_user(1_000_000_000);

    let payer = env.payer.insecure_clone();
    env.send(
        &[ix_initialize(&payer.pubkey(), &admin.pubkey(), &usdc)],
        &[&payer, &admin],
    )
    .unwrap();

    let res = env.send(
        &[ix_accept_admin(&rando.pubkey())],
        &[&payer, &rando],
    );
    assert_error(res, ERR_NO_PENDING_ADMIN);
}
