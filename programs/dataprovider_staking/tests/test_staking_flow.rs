//! End-to-end staking + reward distribution scenarios.

mod common;

use {common::*, solana_signer::Signer};

// Anchor custom error codes (base 6000 + enum index).
const ERR_ZERO_AMOUNT: u32 = 6006;
const ERR_INSUFFICIENT_STAKE: u32 = 6007;
const ERR_INVALID_REWARD_MINT: u32 = 6010;
const ERR_INVALID_STAKE_MINT: u32 = 6011;
const ERR_REWARD_DEPOSIT_TOO_SMALL: u32 = 6012;
const ERR_MAX_POOLS_REACHED: u32 = 6004;

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

#[test]
fn max_pools_reached() {
    let mut env = Env::new();
    let usdc = env.create_mint(6);
    let admin = env.fresh_user(10_000_000_000);
    let payer = env.payer.insecure_clone();

    env.send(
        &[ix_initialize(&payer.pubkey(), &admin.pubkey(), &usdc)],
        &[&payer, &admin],
    )
    .unwrap();

    // Create 5 pools (the max).
    let mut mints = vec![];
    for _ in 0..5 {
        let m = env.create_mint(9);
        mints.push(m);
        env.send(
            &[ix_add_pool(&payer.pubkey(), &admin.pubkey(), &m, &usdc)],
            &[&payer, &admin],
        )
        .unwrap();
    }
    assert_eq!(env.fetch_config().pool_count, 5);

    // 6th pool must fail.
    let extra = env.create_mint(9);
    let res = env.send(
        &[ix_add_pool(&payer.pubkey(), &admin.pubkey(), &extra, &usdc)],
        &[&payer, &admin],
    );
    assert_error(res, ERR_MAX_POOLS_REACHED);
}

#[test]
fn unstake_zero_amount_fails() {
    let mut env = Env::new();
    let (_admin, _usdc, stake_mint) = setup_one_pool(&mut env);

    let user = env.fresh_user(5_000_000_000);
    let user_ata = env.create_ata(&user.pubkey(), &stake_mint);
    env.mint_to(&stake_mint, &user_ata, 1_000);

    let payer = env.payer.insecure_clone();
    env.send(
        &[ix_stake(&user.pubkey(), &stake_mint, &user_ata, 1_000)],
        &[&payer, &user],
    )
    .unwrap();

    let res = env.send(
        &[ix_unstake(&user.pubkey(), &stake_mint, &user_ata, 0)],
        &[&payer, &user],
    );
    assert_error(res, ERR_ZERO_AMOUNT);
}

#[test]
fn deposit_rewards_zero_amount_fails() {
    let mut env = Env::new();
    let (admin, usdc, stake_mint) = setup_one_pool(&mut env);

    let user = env.fresh_user(5_000_000_000);
    let user_ata = env.create_ata(&user.pubkey(), &stake_mint);
    env.mint_to(&stake_mint, &user_ata, 1_000);
    let admin_usdc = env.create_ata(&admin.pubkey(), &usdc);
    env.mint_to(&usdc, &admin_usdc, 1_000_000);

    let payer = env.payer.insecure_clone();
    env.send(
        &[ix_stake(&user.pubkey(), &stake_mint, &user_ata, 1_000)],
        &[&payer, &user],
    )
    .unwrap();

    let res = env.send(
        &[ix_deposit_rewards(&admin.pubkey(), &stake_mint, &usdc, &admin_usdc, 0)],
        &[&payer, &admin],
    );
    assert_error(res, ERR_ZERO_AMOUNT);
}

#[test]
fn cannot_unstake_more_than_staked() {
    let mut env = Env::new();
    let (_admin, _usdc, stake_mint) = setup_one_pool(&mut env);

    let user = env.fresh_user(5_000_000_000);
    let user_ata = env.create_ata(&user.pubkey(), &stake_mint);
    env.mint_to(&stake_mint, &user_ata, 1_000);

    let payer = env.payer.insecure_clone();
    env.send(
        &[ix_stake(&user.pubkey(), &stake_mint, &user_ata, 500)],
        &[&payer, &user],
    )
    .unwrap();

    let res = env.send(
        &[ix_unstake(&user.pubkey(), &stake_mint, &user_ata, 501)],
        &[&payer, &user],
    );
    assert_error(res, ERR_INSUFFICIENT_STAKE);
}

#[test]
fn stake_with_wrong_vault_fails() {
    let mut env = Env::new();
    let (_admin, _usdc, stake_mint) = setup_one_pool(&mut env);

    let user = env.fresh_user(5_000_000_000);
    let user_ata = env.create_ata(&user.pubkey(), &stake_mint);
    env.mint_to(&stake_mint, &user_ata, 1_000);

    // Create a second token account of the same mint (different owner) to use as a fake stake_vault.
    let fake_owner = env.fresh_user(1_000_000);
    let fake_vault = env.create_ata(&fake_owner.pubkey(), &stake_mint);

    let (pool, _) = derive_pool(&stake_mint);
    let (user_stake, _) = derive_user(&stake_mint, &user.pubkey());

    let payer = env.payer.insecure_clone();
    let res = env.send(
        &[ix_stake_raw(
            &user.pubkey(),
            &stake_mint,
            &pool,
            &fake_vault,      // wrong vault for this pool
            &user_stake,
            &user_ata,
            100,
        )],
        &[&payer, &user],
    );
    assert_error(res, ERR_INVALID_STAKE_MINT);
}

#[test]
fn deposit_rewards_with_wrong_reward_vault_fails() {
    let mut env = Env::new();
    let (admin, usdc, stake_mint) = setup_one_pool(&mut env);

    let user = env.fresh_user(5_000_000_000);
    let user_ata = env.create_ata(&user.pubkey(), &stake_mint);
    env.mint_to(&stake_mint, &user_ata, 1_000);
    let admin_usdc = env.create_ata(&admin.pubkey(), &usdc);
    env.mint_to(&usdc, &admin_usdc, 1_000_000);

    let payer = env.payer.insecure_clone();
    env.send(
        &[ix_stake(&user.pubkey(), &stake_mint, &user_ata, 1_000)],
        &[&payer, &user],
    )
    .unwrap();

    // Create a second USDC token account (different owner) to use as a fake reward_vault.
    let fake_owner = env.fresh_user(1_000_000);
    let fake_reward_vault = env.create_ata(&fake_owner.pubkey(), &usdc);

    let (pool, _) = derive_pool(&stake_mint);

    let res = env.send(
        &[ix_deposit_rewards_raw(
            &admin.pubkey(),
            &stake_mint,
            &pool,
            &fake_reward_vault,   // wrong reward vault for this pool
            &usdc,
            &admin_usdc,
            500,
        )],
        &[&payer, &admin],
    );
    assert_error(res, ERR_INVALID_REWARD_MINT);
}

#[test]
fn restake_after_full_unstake_reuses_account() {
    let mut env = Env::new();
    let (_admin, usdc, stake_mint) = setup_one_pool(&mut env);

    let user = env.fresh_user(5_000_000_000);
    let user_ata = env.create_ata(&user.pubkey(), &stake_mint);
    env.mint_to(&stake_mint, &user_ata, 10_000);
    let _user_usdc = env.create_ata(&user.pubkey(), &usdc);

    let payer = env.payer.insecure_clone();
    env.send(
        &[ix_stake(&user.pubkey(), &stake_mint, &user_ata, 5_000)],
        &[&payer, &user],
    )
    .unwrap();

    let first = env.fetch_user(&stake_mint, &user.pubkey());
    assert_eq!(first.amount, 5_000);

    // Unstake everything.
    env.send(
        &[ix_unstake(&user.pubkey(), &stake_mint, &user_ata, 5_000)],
        &[&payer, &user],
    )
    .unwrap();
    let after_unstake = env.fetch_user(&stake_mint, &user.pubkey());
    assert_eq!(after_unstake.amount, 0);

    // Stake again — should reuse the same PDA.
    env.send(
        &[ix_stake(&user.pubkey(), &stake_mint, &user_ata, 2_000)],
        &[&payer, &user],
    )
    .unwrap();
    let second = env.fetch_user(&stake_mint, &user.pubkey());
    assert_eq!(second.amount, 2_000);
    assert_eq!(second.total_claimed, first.total_claimed);
}

#[test]
fn sequential_deposits_accumulate_correctly() {
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

    // Three sequential deposits without any claim.
    env.send(
        &[ix_deposit_rewards(&admin.pubkey(), &stake_mint, &usdc, &admin_usdc, 1_000)],
        &[&payer, &admin],
    )
    .unwrap();
    env.send(
        &[ix_deposit_rewards(&admin.pubkey(), &stake_mint, &usdc, &admin_usdc, 2_000)],
        &[&payer, &admin],
    )
    .unwrap();
    env.send(
        &[ix_deposit_rewards(&admin.pubkey(), &stake_mint, &usdc, &admin_usdc, 3_000)],
        &[&payer, &admin],
    )
    .unwrap();

    env.send(
        &[ix_claim_rewards(&user.pubkey(), &stake_mint, &usdc, &user_usdc)],
        &[&payer, &user],
    )
    .unwrap();

    // 1000 + 2000 + 3000 = 6000
    assert_eq!(env.token_balance(&user_usdc), 6_000);
    assert_eq!(env.fetch_pool(&stake_mint).total_rewards_deposited, 6_000);
}

#[test]
fn tiny_reward_deposit_rejected() {
    let mut env = Env::new();
    let (admin, usdc, stake_mint) = setup_one_pool(&mut env);
    let admin_usdc = env.create_ata(&admin.pubkey(), &usdc);
    env.mint_to(&usdc, &admin_usdc, 1_000_000);

    // Stake a large amount so that 1 USDC unit yields zero delta.
    let user = env.fresh_user(5_000_000_000);
    let user_ata = env.create_ata(&user.pubkey(), &stake_mint);
    env.mint_to(&stake_mint, &user_ata, 2_000_000_000_000); // 2000 tokens (9 dec)

    let payer = env.payer.insecure_clone();
    env.send(
        &[ix_stake(&user.pubkey(), &stake_mint, &user_ata, 2_000_000_000_000)],
        &[&payer, &user],
    )
    .unwrap();

    // amount * ACC_PRECISION / total_staked = 1 * 1e12 / 2e12 = 0 (integer division)
    let res = env.send(
        &[ix_deposit_rewards(&admin.pubkey(), &stake_mint, &usdc, &admin_usdc, 1)],
        &[&payer, &admin],
    );
    assert_error(res, ERR_REWARD_DEPOSIT_TOO_SMALL);

    // Slightly larger deposit should succeed: 2 * 1e12 / 2e12 = 1
    env.send(
        &[ix_deposit_rewards(&admin.pubkey(), &stake_mint, &usdc, &admin_usdc, 2)],
        &[&payer, &admin],
    )
    .expect("deposit of 2 should succeed");
}

/// End-to-end Token-2022 stake → deposit → claim.
///
/// Creates a Token-2022 stake mint (mirrors the real $GHC1CHEM mint on
/// mainnet), pairs it with a classic-SPL USDC reward mint, and runs the
/// full flow: add_pool → stake → deposit_rewards → claim_rewards.
///
/// This is the critical regression guard: if the program ever reverts to
/// pinning the stake token program to classic SPL, this test will fail
/// because the Token-2022 mint account is owned by `TokenzQd...`, not
/// `Tokenkeg...`.
#[test]
fn token_2022_stake_mint_end_to_end() {
    let mut env = Env::new();

    // USDC stays classic SPL (matches mainnet USDC).
    let usdc = env.create_mint(6);
    // Stake mint is Token-2022.
    let stake_mint = env.create_mint_2022(9);

    let admin = env.fresh_user(10_000_000_000);
    let payer = env.payer.insecure_clone();

    // init + add_pool (with the correct token programs for each mint)
    env.send(
        &[ix_initialize(&payer.pubkey(), &admin.pubkey(), &usdc)],
        &[&payer, &admin],
    )
    .unwrap();
    env.send(
        &[ix_add_pool_with_programs(
            &payer.pubkey(),
            &admin.pubkey(),
            &stake_mint,
            &usdc,
            &TOKEN_2022_ID, // stake mint uses Token-2022
            &SPL_TOKEN_ID,  // USDC uses classic SPL
        )],
        &[&payer, &admin],
    )
    .expect("add_pool (token-2022 stake mint) ok");

    // Fund admin + user.
    let admin_usdc = env.create_ata(&admin.pubkey(), &usdc);
    env.mint_to(&usdc, &admin_usdc, 1_000_000_000);

    let user = env.fresh_user(5_000_000_000);
    let user_stake_ata = env.create_ata_2022(&user.pubkey(), &stake_mint);
    env.mint_to_2022(&stake_mint, &user_stake_ata, 1_000_000_000);
    let user_usdc = env.create_ata(&user.pubkey(), &usdc);

    // Stake (must pass TOKEN_2022_ID).
    env.send(
        &[ix_stake_with_program(
            &user.pubkey(),
            &stake_mint,
            &user_stake_ata,
            1_000_000_000,
            &TOKEN_2022_ID,
        )],
        &[&payer, &user],
    )
    .expect("stake (token-2022) ok");

    assert_eq!(
        env.fetch_pool(&stake_mint).total_staked,
        1_000_000_000,
        "pool total_staked should reflect Token-2022 deposit"
    );

    // Deposit USDC rewards (classic SPL path).
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
    .expect("deposit_rewards ok");

    // Claim USDC (classic SPL transfer out of reward vault).
    env.send(
        &[ix_claim_rewards(&user.pubkey(), &stake_mint, &usdc, &user_usdc)],
        &[&payer, &user],
    )
    .expect("claim ok");
    assert_eq!(env.token_balance(&user_usdc), 500_000);

    // Partial unstake (Token-2022 path back out).
    env.send(
        &[ix_unstake_with_program(
            &user.pubkey(),
            &stake_mint,
            &user_stake_ata,
            400_000_000,
            &TOKEN_2022_ID,
        )],
        &[&payer, &user],
    )
    .expect("unstake (token-2022) ok");
    assert_eq!(env.fetch_pool(&stake_mint).total_staked, 600_000_000);
    assert_eq!(env.token_balance(&user_stake_ata), 400_000_000);
}
