//! End-to-end staking + reward distribution scenarios.

mod common;

use {common::*, solana_signer::Signer};

/// Helper: set up env + config + one pool, returning (admin, usdc, stake_mint).
fn setup_one_pool(env: &mut Env) -> (solana_keypair::Keypair, solana_pubkey::Pubkey, solana_pubkey::Pubkey) {
    let usdc = env.create_mint(6);
    let stake_mint = env.create_mint(9);
    let admin = env.fresh_user(10_000_000_000);

    let payer = env.payer.insecure_clone();
    env.send(
        &[ix_initialize(&payer.pubkey(), &admin.pubkey(), &usdc)],
        &[&payer, &admin],
    )
    .unwrap();
    env.send(
        &[ix_add_pool(&payer.pubkey(), &admin.pubkey(), &stake_mint, &usdc)],
        &[&payer, &admin],
    )
    .expect("add_pool ok");

    (admin, usdc, stake_mint)
}

#[test]
fn add_pool_increments_counter_and_creates_vaults() {
    let mut env = Env::new();
    let (_admin, usdc, stake_mint) = setup_one_pool(&mut env);

    let cfg = env.fetch_config();
    assert_eq!(cfg.pool_count, 1);

    let pool = env.fetch_pool(&stake_mint);
    assert_eq!(pool.stake_mint, stake_mint);
    assert_eq!(pool.total_staked, 0);
    assert_eq!(pool.acc_reward_per_share, 0);

    // Vaults must exist with the right mints.
    let (stake_vault, _) = derive_stake_vault(&stake_mint);
    let (reward_vault, _) = derive_reward_vault(&stake_mint);
    assert_eq!(env.token_balance(&stake_vault), 0);
    assert_eq!(env.token_balance(&reward_vault), 0);

    // Drop intentionally unused.
    let _ = usdc;
}

#[test]
fn non_admin_cannot_add_pool() {
    let mut env = Env::new();
    let usdc = env.create_mint(6);
    let admin = env.fresh_user(1_000_000_000);
    let attacker = env.fresh_user(1_000_000_000);
    let stake_mint = env.create_mint(9);

    let payer = env.payer.insecure_clone();
    env.send(
        &[ix_initialize(&payer.pubkey(), &admin.pubkey(), &usdc)],
        &[&payer, &admin],
    )
    .unwrap();

    let res = env.send(
        &[ix_add_pool(&payer.pubkey(), &attacker.pubkey(), &stake_mint, &usdc)],
        &[&payer, &attacker],
    );
    assert!(res.is_err(), "attacker should not be able to add pool");
}

#[test]
fn stake_then_unstake_returns_principal() {
    let mut env = Env::new();
    let (_admin, _usdc, stake_mint) = setup_one_pool(&mut env);

    let user = env.fresh_user(5_000_000_000);
    let user_ata = env.create_ata(&user.pubkey(), &stake_mint);
    env.mint_to(&stake_mint, &user_ata, 10_000_000_000); // 10 tokens (9dec)

    let payer = env.payer.insecure_clone();
    env.send(
        &[ix_stake(&user.pubkey(), &stake_mint, &user_ata, 4_000_000_000)],
        &[&payer, &user],
    )
    .expect("stake ok");

    let pool = env.fetch_pool(&stake_mint);
    assert_eq!(pool.total_staked, 4_000_000_000);
    let u = env.fetch_user(&stake_mint, &user.pubkey());
    assert_eq!(u.amount, 4_000_000_000);
    assert_eq!(u.pending_rewards, 0);

    // Unstake partial
    env.send(
        &[ix_unstake(&user.pubkey(), &stake_mint, &user_ata, 1_000_000_000)],
        &[&payer, &user],
    )
    .expect("unstake ok");
    assert_eq!(env.fetch_pool(&stake_mint).total_staked, 3_000_000_000);
    assert_eq!(env.fetch_user(&stake_mint, &user.pubkey()).amount, 3_000_000_000);
    // Balance check: started 10, staked 4, unstaked 1, remaining liquid = 7
    assert_eq!(env.token_balance(&user_ata), 7_000_000_000);

    // Unstake the rest
    env.send(
        &[ix_unstake(&user.pubkey(), &stake_mint, &user_ata, 3_000_000_000)],
        &[&payer, &user],
    )
    .unwrap();
    assert_eq!(env.fetch_pool(&stake_mint).total_staked, 0);
    assert_eq!(env.token_balance(&user_ata), 10_000_000_000);
}

#[test]
fn cannot_deposit_rewards_when_no_stakers() {
    let mut env = Env::new();
    let (admin, usdc, stake_mint) = setup_one_pool(&mut env);

    let admin_usdc = env.create_ata(&admin.pubkey(), &usdc);
    env.mint_to(&usdc, &admin_usdc, 1_000_000);

    let payer = env.payer.insecure_clone();
    let res = env.send(
        &[ix_deposit_rewards(
            &admin.pubkey(),
            &stake_mint,
            &usdc,
            &admin_usdc,
            500_000,
        )],
        &[&payer, &admin],
    );
    assert!(res.is_err(), "no-stakers deposit must fail");
}

#[test]
fn cannot_claim_if_nothing_pending() {
    let mut env = Env::new();
    let (_admin, usdc, stake_mint) = setup_one_pool(&mut env);
    let user = env.fresh_user(5_000_000_000);
    let user_stake_ata = env.create_ata(&user.pubkey(), &stake_mint);
    env.mint_to(&stake_mint, &user_stake_ata, 100);
    let user_usdc = env.create_ata(&user.pubkey(), &usdc);

    let payer = env.payer.insecure_clone();
    env.send(
        &[ix_stake(&user.pubkey(), &stake_mint, &user_stake_ata, 100)],
        &[&payer, &user],
    )
    .unwrap();

    let res = env.send(
        &[ix_claim_rewards(&user.pubkey(), &stake_mint, &usdc, &user_usdc)],
        &[&payer, &user],
    );
    assert!(res.is_err(), "claim with 0 pending must fail");
}

/// Single staker earns 100% of deposited rewards.
#[test]
fn single_staker_receives_full_rewards() {
    let mut env = Env::new();
    let (admin, usdc, stake_mint) = setup_one_pool(&mut env);

    let admin_usdc = env.create_ata(&admin.pubkey(), &usdc);
    env.mint_to(&usdc, &admin_usdc, 1_000_000_000);

    let user = env.fresh_user(5_000_000_000);
    let user_stake_ata = env.create_ata(&user.pubkey(), &stake_mint);
    env.mint_to(&stake_mint, &user_stake_ata, 1_000_000_000);
    let user_usdc = env.create_ata(&user.pubkey(), &usdc);

    let payer = env.payer.insecure_clone();
    env.send(
        &[ix_stake(&user.pubkey(), &stake_mint, &user_stake_ata, 1_000_000_000)],
        &[&payer, &user],
    )
    .unwrap();

    // Admin deposits 500_000 USDC (6 dec).
    env.send(
        &[ix_deposit_rewards(
            &admin.pubkey(),
            &stake_mint,
            &usdc,
            &admin_usdc,
            500_000,
        )],
        &[&payer, &admin],
    )
    .expect("deposit ok");

    let pool = env.fetch_pool(&stake_mint);
    assert_eq!(pool.total_rewards_deposited, 500_000);
    assert!(pool.acc_reward_per_share > 0);

    env.send(
        &[ix_claim_rewards(&user.pubkey(), &stake_mint, &usdc, &user_usdc)],
        &[&payer, &user],
    )
    .expect("claim ok");

    assert_eq!(env.token_balance(&user_usdc), 500_000);
    assert_eq!(env.fetch_user(&stake_mint, &user.pubkey()).total_claimed, 500_000);
    assert_eq!(env.fetch_user(&stake_mint, &user.pubkey()).pending_rewards, 0);
}

/// Two stakers with 1:3 ratio split rewards proportionally.
#[test]
fn two_stakers_split_rewards_proportionally() {
    let mut env = Env::new();
    let (admin, usdc, stake_mint) = setup_one_pool(&mut env);
    let admin_usdc = env.create_ata(&admin.pubkey(), &usdc);
    env.mint_to(&usdc, &admin_usdc, 1_000_000_000);

    let alice = env.fresh_user(5_000_000_000);
    let bob = env.fresh_user(5_000_000_000);
    let alice_ata = env.create_ata(&alice.pubkey(), &stake_mint);
    let bob_ata = env.create_ata(&bob.pubkey(), &stake_mint);
    env.mint_to(&stake_mint, &alice_ata, 1_000);
    env.mint_to(&stake_mint, &bob_ata, 3_000);

    let alice_usdc = env.create_ata(&alice.pubkey(), &usdc);
    let bob_usdc = env.create_ata(&bob.pubkey(), &usdc);

    let payer = env.payer.insecure_clone();
    // Both stake in same block so reward accrues to both.
    env.send(
        &[ix_stake(&alice.pubkey(), &stake_mint, &alice_ata, 1_000)],
        &[&payer, &alice],
    )
    .unwrap();
    env.send(
        &[ix_stake(&bob.pubkey(), &stake_mint, &bob_ata, 3_000)],
        &[&payer, &bob],
    )
    .unwrap();

    // Admin deposits 4000 USDC units; should split 1000 / 3000.
    env.send(
        &[ix_deposit_rewards(&admin.pubkey(), &stake_mint, &usdc, &admin_usdc, 4_000)],
        &[&payer, &admin],
    )
    .unwrap();

    env.send(
        &[ix_claim_rewards(&alice.pubkey(), &stake_mint, &usdc, &alice_usdc)],
        &[&payer, &alice],
    )
    .unwrap();
    env.send(
        &[ix_claim_rewards(&bob.pubkey(), &stake_mint, &usdc, &bob_usdc)],
        &[&payer, &bob],
    )
    .unwrap();

    assert_eq!(env.token_balance(&alice_usdc), 1_000);
    assert_eq!(env.token_balance(&bob_usdc), 3_000);
}

/// New staker joining AFTER a reward deposit earns nothing on that batch,
/// but earns pro-rata on subsequent deposits.
#[test]
fn late_staker_only_earns_on_future_deposits() {
    let mut env = Env::new();
    let (admin, usdc, stake_mint) = setup_one_pool(&mut env);
    let admin_usdc = env.create_ata(&admin.pubkey(), &usdc);
    env.mint_to(&usdc, &admin_usdc, 1_000_000_000);

    let early = env.fresh_user(5_000_000_000);
    let late = env.fresh_user(5_000_000_000);
    let early_stake = env.create_ata(&early.pubkey(), &stake_mint);
    let late_stake = env.create_ata(&late.pubkey(), &stake_mint);
    env.mint_to(&stake_mint, &early_stake, 1_000);
    env.mint_to(&stake_mint, &late_stake, 1_000);

    let early_usdc = env.create_ata(&early.pubkey(), &usdc);
    let late_usdc = env.create_ata(&late.pubkey(), &usdc);

    let payer = env.payer.insecure_clone();
    env.send(
        &[ix_stake(&early.pubkey(), &stake_mint, &early_stake, 1_000)],
        &[&payer, &early],
    )
    .unwrap();

    // Deposit 1: only early holder exists.
    env.send(
        &[ix_deposit_rewards(&admin.pubkey(), &stake_mint, &usdc, &admin_usdc, 1_000)],
        &[&payer, &admin],
    )
    .unwrap();

    // Late joins.
    env.send(
        &[ix_stake(&late.pubkey(), &stake_mint, &late_stake, 1_000)],
        &[&payer, &late],
    )
    .unwrap();

    // Deposit 2: split 50/50 between early and late.
    env.send(
        &[ix_deposit_rewards(&admin.pubkey(), &stake_mint, &usdc, &admin_usdc, 2_000)],
        &[&payer, &admin],
    )
    .unwrap();

    env.send(
        &[ix_claim_rewards(&early.pubkey(), &stake_mint, &usdc, &early_usdc)],
        &[&payer, &early],
    )
    .unwrap();
    env.send(
        &[ix_claim_rewards(&late.pubkey(), &stake_mint, &usdc, &late_usdc)],
        &[&payer, &late],
    )
    .unwrap();

    // Early: 1000 (all of deposit 1) + 1000 (half of deposit 2) = 2000.
    // Late: 1000 (half of deposit 2).
    assert_eq!(env.token_balance(&early_usdc), 2_000);
    assert_eq!(env.token_balance(&late_usdc), 1_000);
}

/// Unstaking mid-cycle does NOT grant rewards to already-deposited batches,
/// but DOES preserve pending rewards until claim.
#[test]
fn unstake_then_claim_preserves_pending() {
    let mut env = Env::new();
    let (admin, usdc, stake_mint) = setup_one_pool(&mut env);
    let admin_usdc = env.create_ata(&admin.pubkey(), &usdc);
    env.mint_to(&usdc, &admin_usdc, 1_000_000_000);

    let user = env.fresh_user(5_000_000_000);
    let user_ata = env.create_ata(&user.pubkey(), &stake_mint);
    env.mint_to(&stake_mint, &user_ata, 1_000);
    let user_usdc = env.create_ata(&user.pubkey(), &usdc);

    let payer = env.payer.insecure_clone();
    env.send(
        &[ix_stake(&user.pubkey(), &stake_mint, &user_ata, 1_000)],
        &[&payer, &user],
    )
    .unwrap();
    env.send(
        &[ix_deposit_rewards(&admin.pubkey(), &stake_mint, &usdc, &admin_usdc, 500)],
        &[&payer, &admin],
    )
    .unwrap();
    // Unstake entire position WITHOUT claiming.
    env.send(
        &[ix_unstake(&user.pubkey(), &stake_mint, &user_ata, 1_000)],
        &[&payer, &user],
    )
    .unwrap();
    // Pending should be 500 now; claim should work.
    let u = env.fetch_user(&stake_mint, &user.pubkey());
    assert_eq!(u.amount, 0);
    assert_eq!(u.pending_rewards, 500);

    env.send(
        &[ix_claim_rewards(&user.pubkey(), &stake_mint, &usdc, &user_usdc)],
        &[&payer, &user],
    )
    .unwrap();
    assert_eq!(env.token_balance(&user_usdc), 500);
}

/// Staker staking in multiple pools: independent rewards per pool.
#[test]
fn user_can_stake_in_multiple_pools() {
    let mut env = Env::new();
    let usdc = env.create_mint(6);
    let mint_a = env.create_mint(9);
    let mint_b = env.create_mint(9);
    let admin = env.fresh_user(10_000_000_000);
    let payer = env.payer.insecure_clone();

    env.send(
        &[ix_initialize(&payer.pubkey(), &admin.pubkey(), &usdc)],
        &[&payer, &admin],
    )
    .unwrap();
    env.send(
        &[ix_add_pool(&payer.pubkey(), &admin.pubkey(), &mint_a, &usdc)],
        &[&payer, &admin],
    )
    .unwrap();
    env.send(
        &[ix_add_pool(&payer.pubkey(), &admin.pubkey(), &mint_b, &usdc)],
        &[&payer, &admin],
    )
    .unwrap();

    let admin_usdc = env.create_ata(&admin.pubkey(), &usdc);
    env.mint_to(&usdc, &admin_usdc, 1_000_000_000);

    let user = env.fresh_user(5_000_000_000);
    let ata_a = env.create_ata(&user.pubkey(), &mint_a);
    let ata_b = env.create_ata(&user.pubkey(), &mint_b);
    env.mint_to(&mint_a, &ata_a, 100);
    env.mint_to(&mint_b, &ata_b, 100);
    let user_usdc = env.create_ata(&user.pubkey(), &usdc);

    env.send(
        &[ix_stake(&user.pubkey(), &mint_a, &ata_a, 100)],
        &[&payer, &user],
    )
    .unwrap();
    env.send(
        &[ix_stake(&user.pubkey(), &mint_b, &ata_b, 100)],
        &[&payer, &user],
    )
    .unwrap();

    // Deposit different amounts to each pool.
    env.send(
        &[ix_deposit_rewards(&admin.pubkey(), &mint_a, &usdc, &admin_usdc, 700)],
        &[&payer, &admin],
    )
    .unwrap();
    env.send(
        &[ix_deposit_rewards(&admin.pubkey(), &mint_b, &usdc, &admin_usdc, 300)],
        &[&payer, &admin],
    )
    .unwrap();

    // Claim both.
    env.send(
        &[ix_claim_rewards(&user.pubkey(), &mint_a, &usdc, &user_usdc)],
        &[&payer, &user],
    )
    .unwrap();
    env.send(
        &[ix_claim_rewards(&user.pubkey(), &mint_b, &usdc, &user_usdc)],
        &[&payer, &user],
    )
    .unwrap();
    assert_eq!(env.token_balance(&user_usdc), 1_000);

    // Pool counters are independent.
    assert_eq!(env.fetch_pool(&mint_a).total_rewards_deposited, 700);
    assert_eq!(env.fetch_pool(&mint_b).total_rewards_deposited, 300);
}

/// Staking more after a reward deposit does not steal past rewards.
#[test]
fn increment_stake_after_deposit_does_not_backfill() {
    let mut env = Env::new();
    let (admin, usdc, stake_mint) = setup_one_pool(&mut env);
    let admin_usdc = env.create_ata(&admin.pubkey(), &usdc);
    env.mint_to(&usdc, &admin_usdc, 1_000_000_000);

    let user = env.fresh_user(5_000_000_000);
    let user_ata = env.create_ata(&user.pubkey(), &stake_mint);
    env.mint_to(&stake_mint, &user_ata, 1_000_000);
    let user_usdc = env.create_ata(&user.pubkey(), &usdc);

    let payer = env.payer.insecure_clone();
    env.send(
        &[ix_stake(&user.pubkey(), &stake_mint, &user_ata, 100)],
        &[&payer, &user],
    )
    .unwrap();
    env.send(
        &[ix_deposit_rewards(&admin.pubkey(), &stake_mint, &usdc, &admin_usdc, 1_000)],
        &[&payer, &admin],
    )
    .unwrap();
    // Now add a lot more stake.
    env.send(
        &[ix_stake(&user.pubkey(), &stake_mint, &user_ata, 900_000)],
        &[&payer, &user],
    )
    .unwrap();

    // Claim -> only 1000 (from the first batch, user had the full pool alone).
    env.send(
        &[ix_claim_rewards(&user.pubkey(), &stake_mint, &usdc, &user_usdc)],
        &[&payer, &user],
    )
    .unwrap();
    assert_eq!(env.token_balance(&user_usdc), 1_000);
}
