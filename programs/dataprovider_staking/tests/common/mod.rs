//! Shared helpers for the dataprovider_staking integration tests.
//!
//! Each test file gets a fresh `LiteSVM` instance via [`Env::new`] which
//! loads the freshly-compiled program binary. All TokenAccount / Mint
//! setup goes through `litesvm-token` for brevity.
//!
//! Each test file includes this module directly (`mod common;`), so the Rust
//! compiler treats it as a separate copy per test binary. Functions not used
//! in a given file would otherwise trip `dead_code`, hence the blanket
//! allow below.
#![allow(dead_code)]

use {
    anchor_lang::{prelude::AccountMeta, InstructionData},
    dataprovider_staking::{
        state::{GlobalConfig, TokenPool, UserStake},
        ID as PROGRAM_ID,
    },
    litesvm::{types::TransactionMetadata, LiteSVM},
    litesvm_token::{
        spl_token, CreateAssociatedTokenAccount, CreateMint, MintTo,
    },
    solana_instruction::Instruction,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

pub const CONFIG_SEED: &[u8] = b"config";
pub const POOL_SEED: &[u8] = b"pool";
pub const VAULT_AUTH_SEED: &[u8] = b"vault_auth";
pub const USER_SEED: &[u8] = b"user";

/// Classic SPL Token program id (Tokenkeg…).
pub const SPL_TOKEN_ID: Pubkey = spl_token::ID;

/// Token-2022 program id (TokenzQd…).
/// Hardcoded from the canonical Token-2022 program; litesvm-token's embedded
/// token-2022 program is loaded by default.
pub const TOKEN_2022_ID: Pubkey = solana_pubkey::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

/// A loaded litesvm environment with the staking program preloaded.
pub struct Env {
    pub svm: LiteSVM,
    pub payer: Keypair,
}

impl Env {
    pub fn new() -> Self {
        let mut svm = LiteSVM::new();
        let bytes = include_bytes!("../../../../target/deploy/dataprovider_staking.so");
        svm.add_program(PROGRAM_ID, bytes).unwrap();
        let payer = Keypair::new();
        svm.airdrop(&payer.pubkey(), 1_000_000_000_000).unwrap();
        Self { svm, payer }
    }

    /// Airdrop a fresh funded keypair.
    pub fn fresh_user(&mut self, lamports: u64) -> Keypair {
        let kp = Keypair::new();
        self.svm.airdrop(&kp.pubkey(), lamports).unwrap();
        kp
    }

    /// Send a transaction signed by `signers` (payer is first) and return metadata.
    pub fn send(
        &mut self,
        ixs: &[Instruction],
        signers: &[&Keypair],
    ) -> Result<TransactionMetadata, String> {
        let blockhash = self.svm.latest_blockhash();
        let payer_pk = signers[0].pubkey();
        let msg = Message::new_with_blockhash(ixs, Some(&payer_pk), &blockhash);
        let signers_owned: Vec<&Keypair> = signers.iter().copied().collect();
        let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &signers_owned)
            .map_err(|e| e.to_string())?;
        self.svm.send_transaction(tx).map_err(|e| format!("{:?}", e))
    }

    /// Create a fresh SPL mint with given decimals, authority owned by the env payer.
    pub fn create_mint(&mut self, decimals: u8) -> Pubkey {
        let payer_pk = self.payer.pubkey();
        let payer = self.payer.insecure_clone();
        CreateMint::new(&mut self.svm, &payer)
            .decimals(decimals)
            .authority(&payer_pk)
            .send()
            .unwrap()
    }

    /// Create a fresh Token-2022 mint (no extensions) with given decimals.
    pub fn create_mint_2022(&mut self, decimals: u8) -> Pubkey {
        let payer_pk = self.payer.pubkey();
        let payer = self.payer.insecure_clone();
        CreateMint::new(&mut self.svm, &payer)
            .decimals(decimals)
            .authority(&payer_pk)
            .token_program_id(&TOKEN_2022_ID)
            .send()
            .unwrap()
    }

    /// Create ATA for owner and mint (classic SPL Token program).
    pub fn create_ata(&mut self, owner: &Pubkey, mint: &Pubkey) -> Pubkey {
        let payer = self.payer.insecure_clone();
        CreateAssociatedTokenAccount::new(&mut self.svm, &payer, mint)
            .owner(owner)
            .send()
            .unwrap()
    }

    /// Create ATA for a Token-2022 mint.
    pub fn create_ata_2022(&mut self, owner: &Pubkey, mint: &Pubkey) -> Pubkey {
        let payer = self.payer.insecure_clone();
        CreateAssociatedTokenAccount::new(&mut self.svm, &payer, mint)
            .owner(owner)
            .token_program_id(&TOKEN_2022_ID)
            .send()
            .unwrap()
    }

    /// Mint `amount` to `destination` ATA, classic SPL Token program.
    pub fn mint_to(&mut self, mint: &Pubkey, destination: &Pubkey, amount: u64) {
        let payer = self.payer.insecure_clone();
        MintTo::new(&mut self.svm, &payer, mint, destination, amount)
            .send()
            .unwrap();
    }

    /// Mint `amount` of a Token-2022 mint to `destination` ATA.
    pub fn mint_to_2022(&mut self, mint: &Pubkey, destination: &Pubkey, amount: u64) {
        let payer = self.payer.insecure_clone();
        MintTo::new(&mut self.svm, &payer, mint, destination, amount)
            .token_program_id(&TOKEN_2022_ID)
            .send()
            .unwrap();
    }

    pub fn token_balance(&self, ata: &Pubkey) -> u64 {
        let acct = self.svm.get_account(ata).expect("ata exists");
        // Works for both classic SPL and Token-2022: the first 165 bytes of
        // a Token-2022 account are the same layout as a classic SPL account;
        // extensions live after that region.
        let parsed = <spl_token::state::Account as solana_program_pack::Pack>::unpack(
            &acct.data[..spl_token::state::Account::LEN],
        )
        .unwrap();
        parsed.amount
    }

    pub fn fetch_config(&self) -> GlobalConfig {
        let (pda, _) = derive_config();
        let acct = self.svm.get_account(&pda).expect("config pda");
        GlobalConfig::try_deserialize(&mut &acct.data[..]).unwrap()
    }

    pub fn fetch_pool(&self, stake_mint: &Pubkey) -> TokenPool {
        let (pda, _) = derive_pool(stake_mint);
        let acct = self.svm.get_account(&pda).expect("pool pda");
        TokenPool::try_deserialize(&mut &acct.data[..]).unwrap()
    }

    pub fn fetch_user(&self, stake_mint: &Pubkey, owner: &Pubkey) -> UserStake {
        let (pda, _) = derive_user(stake_mint, owner);
        let acct = self.svm.get_account(&pda).expect("user pda");
        UserStake::try_deserialize(&mut &acct.data[..]).unwrap()
    }
}

/// Assert that a transaction result failed with a specific Anchor custom error code.
/// Anchor custom errors start at 6000 + enum index.
pub fn assert_error(res: Result<TransactionMetadata, String>, expected_code: u32) {
    let err = res.expect_err("expected tx to fail");
    let needle = format!("Custom({})", expected_code);
    assert!(
        err.contains(&needle),
        "expected error {} in: {}",
        needle,
        err
    );
}

// Need Pack for SPL Account size/unpack.
use solana_program_pack::Pack;
// And anchor trait for deserialize.
use anchor_lang::AccountDeserialize;

// ----- PDA helpers -----

pub fn derive_config() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[CONFIG_SEED], &PROGRAM_ID)
}

pub fn derive_pool(stake_mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[POOL_SEED, stake_mint.as_ref()], &PROGRAM_ID)
}

pub fn derive_vault_auth(stake_mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[VAULT_AUTH_SEED, stake_mint.as_ref()], &PROGRAM_ID)
}

pub fn derive_stake_vault(stake_mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"stake_vault", stake_mint.as_ref()], &PROGRAM_ID)
}

pub fn derive_reward_vault(stake_mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"reward_vault", stake_mint.as_ref()], &PROGRAM_ID)
}

pub fn derive_user(stake_mint: &Pubkey, owner: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[USER_SEED, stake_mint.as_ref(), owner.as_ref()], &PROGRAM_ID)
}

// ----- Instruction builders -----

pub fn ix_initialize(payer: &Pubkey, admin: &Pubkey, usdc_mint: &Pubkey) -> Instruction {
    let (config, _) = derive_config();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(*admin, true),
            AccountMeta::new_readonly(*usdc_mint, false),
            AccountMeta::new(config, false),
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        ],
        data: dataprovider_staking::instruction::Initialize {}.data(),
    }
}

pub fn ix_add_pool(
    payer: &Pubkey,
    admin: &Pubkey,
    stake_mint: &Pubkey,
    usdc_mint: &Pubkey,
) -> Instruction {
    ix_add_pool_with_programs(
        payer,
        admin,
        stake_mint,
        usdc_mint,
        &spl_token::ID,
        &spl_token::ID,
    )
}

/// Like `ix_add_pool` but lets callers pass the stake-mint token program
/// (classic SPL or Token-2022) and the USDC token program independently.
pub fn ix_add_pool_with_programs(
    payer: &Pubkey,
    admin: &Pubkey,
    stake_mint: &Pubkey,
    usdc_mint: &Pubkey,
    stake_token_program: &Pubkey,
    usdc_token_program: &Pubkey,
) -> Instruction {
    let (config, _) = derive_config();
    let (pool, _) = derive_pool(stake_mint);
    let (vault_auth, _) = derive_vault_auth(stake_mint);
    let (stake_vault, _) = derive_stake_vault(stake_mint);
    let (reward_vault, _) = derive_reward_vault(stake_mint);
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(config, false),
            AccountMeta::new_readonly(*admin, true),
            AccountMeta::new_readonly(*stake_mint, false),
            AccountMeta::new_readonly(*usdc_mint, false),
            AccountMeta::new(pool, false),
            AccountMeta::new_readonly(vault_auth, false),
            AccountMeta::new(stake_vault, false),
            AccountMeta::new(reward_vault, false),
            AccountMeta::new_readonly(*stake_token_program, false),
            AccountMeta::new_readonly(*usdc_token_program, false),
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
            AccountMeta::new_readonly(solana_sysvar::rent::ID, false),
        ],
        data: dataprovider_staking::instruction::AddPool {}.data(),
    }
}

pub fn ix_stake(
    user: &Pubkey,
    stake_mint: &Pubkey,
    user_token_account: &Pubkey,
    amount: u64,
) -> Instruction {
    ix_stake_with_program(
        user,
        stake_mint,
        user_token_account,
        amount,
        &spl_token::ID,
    )
}

pub fn ix_stake_with_program(
    user: &Pubkey,
    stake_mint: &Pubkey,
    user_token_account: &Pubkey,
    amount: u64,
    token_program: &Pubkey,
) -> Instruction {
    let (pool, _) = derive_pool(stake_mint);
    let (stake_vault, _) = derive_stake_vault(stake_mint);
    let (user_stake, _) = derive_user(stake_mint, user);
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*user, true),
            AccountMeta::new_readonly(*stake_mint, false),
            AccountMeta::new(pool, false),
            AccountMeta::new(stake_vault, false),
            AccountMeta::new(user_stake, false),
            AccountMeta::new(*user_token_account, false),
            AccountMeta::new_readonly(*token_program, false),
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
            AccountMeta::new_readonly(solana_sysvar::rent::ID, false),
        ],
        data: dataprovider_staking::instruction::Stake { amount }.data(),
    }
}

pub fn ix_unstake(
    user: &Pubkey,
    stake_mint: &Pubkey,
    user_token_account: &Pubkey,
    amount: u64,
) -> Instruction {
    ix_unstake_with_program(
        user,
        stake_mint,
        user_token_account,
        amount,
        &spl_token::ID,
    )
}

pub fn ix_unstake_with_program(
    user: &Pubkey,
    stake_mint: &Pubkey,
    user_token_account: &Pubkey,
    amount: u64,
    token_program: &Pubkey,
) -> Instruction {
    let (pool, _) = derive_pool(stake_mint);
    let (stake_vault, _) = derive_stake_vault(stake_mint);
    let (vault_auth, _) = derive_vault_auth(stake_mint);
    let (user_stake, _) = derive_user(stake_mint, user);
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*user, true),
            AccountMeta::new_readonly(*stake_mint, false),
            AccountMeta::new(pool, false),
            AccountMeta::new(stake_vault, false),
            AccountMeta::new_readonly(vault_auth, false),
            AccountMeta::new(user_stake, false),
            AccountMeta::new(*user_token_account, false),
            AccountMeta::new_readonly(*token_program, false),
        ],
        data: dataprovider_staking::instruction::Unstake { amount }.data(),
    }
}

pub fn ix_deposit_rewards(
    admin: &Pubkey,
    stake_mint: &Pubkey,
    usdc_mint: &Pubkey,
    admin_usdc_account: &Pubkey,
    amount: u64,
) -> Instruction {
    ix_deposit_rewards_with_program(
        admin,
        stake_mint,
        usdc_mint,
        admin_usdc_account,
        amount,
        &spl_token::ID,
    )
}

pub fn ix_deposit_rewards_with_program(
    admin: &Pubkey,
    stake_mint: &Pubkey,
    usdc_mint: &Pubkey,
    admin_usdc_account: &Pubkey,
    amount: u64,
    token_program: &Pubkey,
) -> Instruction {
    let (config, _) = derive_config();
    let (pool, _) = derive_pool(stake_mint);
    let (reward_vault, _) = derive_reward_vault(stake_mint);
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(config, false),
            AccountMeta::new_readonly(*admin, true),
            AccountMeta::new_readonly(*stake_mint, false),
            AccountMeta::new(pool, false),
            AccountMeta::new(reward_vault, false),
            AccountMeta::new_readonly(*usdc_mint, false),
            AccountMeta::new(*admin_usdc_account, false),
            AccountMeta::new_readonly(*token_program, false),
        ],
        data: dataprovider_staking::instruction::DepositRewards { amount }.data(),
    }
}

pub fn ix_claim_rewards(
    user: &Pubkey,
    stake_mint: &Pubkey,
    usdc_mint: &Pubkey,
    user_usdc_account: &Pubkey,
) -> Instruction {
    ix_claim_rewards_with_program(
        user,
        stake_mint,
        usdc_mint,
        user_usdc_account,
        &spl_token::ID,
    )
}

pub fn ix_claim_rewards_with_program(
    user: &Pubkey,
    stake_mint: &Pubkey,
    usdc_mint: &Pubkey,
    user_usdc_account: &Pubkey,
    token_program: &Pubkey,
) -> Instruction {
    let (config, _) = derive_config();
    let (pool, _) = derive_pool(stake_mint);
    let (reward_vault, _) = derive_reward_vault(stake_mint);
    let (vault_auth, _) = derive_vault_auth(stake_mint);
    let (user_stake, _) = derive_user(stake_mint, user);
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*user, true),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new_readonly(*stake_mint, false),
            AccountMeta::new(pool, false),
            AccountMeta::new(reward_vault, false),
            AccountMeta::new_readonly(*usdc_mint, false),
            AccountMeta::new_readonly(vault_auth, false),
            AccountMeta::new(user_stake, false),
            AccountMeta::new(*user_usdc_account, false),
            AccountMeta::new_readonly(*token_program, false),
        ],
        data: dataprovider_staking::instruction::ClaimRewards {}.data(),
    }
}

pub fn ix_propose_admin(admin: &Pubkey, new_admin: &Pubkey) -> Instruction {
    let (config, _) = derive_config();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(config, false),
            AccountMeta::new_readonly(*admin, true),
            AccountMeta::new_readonly(*new_admin, false),
        ],
        data: dataprovider_staking::instruction::ProposeAdmin {}.data(),
    }
}

pub fn ix_accept_admin(new_admin: &Pubkey) -> Instruction {
    let (config, _) = derive_config();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(config, false),
            AccountMeta::new_readonly(*new_admin, true),
        ],
        data: dataprovider_staking::instruction::AcceptAdmin {}.data(),
    }
}

pub fn ix_cancel_admin_proposal(admin: &Pubkey) -> Instruction {
    let (config, _) = derive_config();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(config, false),
            AccountMeta::new_readonly(*admin, true),
        ],
        data: dataprovider_staking::instruction::CancelAdminProposal {}.data(),
    }
}

// ----- Raw instruction builders (for negative tests with wrong accounts) -----

pub fn ix_stake_raw(
    user: &Pubkey,
    stake_mint: &Pubkey,
    pool: &Pubkey,
    stake_vault: &Pubkey,
    user_stake: &Pubkey,
    user_token_account: &Pubkey,
    amount: u64,
) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*user, true),
            AccountMeta::new_readonly(*stake_mint, false),
            AccountMeta::new(*pool, false),
            AccountMeta::new(*stake_vault, false),
            AccountMeta::new(*user_stake, false),
            AccountMeta::new(*user_token_account, false),
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
            AccountMeta::new_readonly(solana_sysvar::rent::ID, false),
        ],
        data: dataprovider_staking::instruction::Stake { amount }.data(),
    }
}

pub fn ix_deposit_rewards_raw(
    admin: &Pubkey,
    stake_mint: &Pubkey,
    pool: &Pubkey,
    reward_vault: &Pubkey,
    usdc_mint: &Pubkey,
    admin_usdc_account: &Pubkey,
    amount: u64,
) -> Instruction {
    let (config, _) = derive_config();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(config, false),
            AccountMeta::new_readonly(*admin, true),
            AccountMeta::new_readonly(*stake_mint, false),
            AccountMeta::new(*pool, false),
            AccountMeta::new(*reward_vault, false),
            AccountMeta::new_readonly(*usdc_mint, false),
            AccountMeta::new(*admin_usdc_account, false),
            AccountMeta::new_readonly(spl_token::ID, false),
        ],
        data: dataprovider_staking::instruction::DepositRewards { amount }.data(),
    }
}


