#!/usr/bin/env node
/**
 * Quick devnet end-to-end test: stake → deposit rewards → claim
 */
import { AnchorProvider, Wallet, BN } from "@coral-xyz/anchor";
import {
  Connection,
  Keypair,
  PublicKey,
  clusterApiUrl,
} from "@solana/web3.js";
import { StakingClient } from "@dataprovider/staking-sdk";
import {
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountIdempotent,
} from "@solana/spl-token";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";

// ---- Config ----
const RPC_URL = clusterApiUrl("devnet");
const KEYPAIR_PATH = `${os.homedir()}/.config/solana/id.json`;
const PROGRAM_ID = new PublicKey("AnConH6PVX1UQYtdPgAgUNMowphcragEjbGsx3nQJ6up");

// These are the mints we created on devnet
const STAKE_MINT = new PublicKey("GunBcDzHL5iYLQNJQqb7w9v1TzxaNX6YnE9D36ZZ8yQ1");
const USDC_MINT = new PublicKey("EKZiA3ZM3GqH67eGNgvja8xf7xVTSRbbvqcVBj5Y1o1q");

const STAKE_AMOUNT = new BN(100_000_000_000); // 100 tokens (9 decimals)
const REWARD_AMOUNT = new BN(50_000_000); // 50 USDC (6 decimals)

function loadKeypair(keypairPath: string): Keypair {
  const expanded = keypairPath.startsWith("~")
    ? path.join(os.homedir(), keypairPath.slice(1))
    : keypairPath;
  const raw = JSON.parse(fs.readFileSync(expanded, "utf8"));
  return Keypair.fromSecretKey(Uint8Array.from(raw));
}

async function main() {
  const keypair = loadKeypair(KEYPAIR_PATH);
  const wallet = new Wallet(keypair);
  const connection = new Connection(RPC_URL, "confirmed");
  const provider = new AnchorProvider(connection, wallet, {
    commitment: "confirmed",
    preflightCommitment: "confirmed",
  });
  const client = new StakingClient(provider, { programId: PROGRAM_ID });

  console.log("Wallet:", wallet.publicKey.toBase58());
  const balance = await connection.getBalance(wallet.publicKey);
  console.log("SOL balance:", balance / 1e9);

  // 1. Fetch config
  const config = await client.fetchConfig();
  if (!config) throw new Error("Config not found!");
  console.log("\n✅ Config loaded");
  console.log("  Admin:", config.admin.toBase58());
  console.log("  USDC mint:", config.usdcMint.toBase58());
  console.log("  Pool count:", config.poolCount);

  // 2. Fetch pool
  const pool = await client.fetchPool(STAKE_MINT);
  if (!pool) throw new Error("Pool not found!");
  console.log("\n✅ Pool loaded");
  console.log("  Stake mint:", pool.stakeMint.toBase58());
  console.log("  Total staked:", pool.totalStaked.toString());

  // 3. Get user stake ATA
  const stakeAta = getAssociatedTokenAddressSync(STAKE_MINT, wallet.publicKey);
  const stakeBalance = await connection.getTokenAccountBalance(stakeAta);
  console.log("\n🪙 Stake token balance:", stakeBalance.value.uiAmount);

  // 4. Stake
  console.log("\n📝 Staking", STAKE_AMOUNT.toString(), "tokens...");
  const stakeSig = await client.stakeAndSend(STAKE_MINT, STAKE_AMOUNT);
  console.log("✅ Stake tx:", stakeSig);

  // 5. Verify stake
  const userStake = await client.fetchUserStake(STAKE_MINT, wallet.publicKey);
  console.log("\n✅ User stake:", userStake?.amount.toString());

  const poolAfter = await client.fetchPool(STAKE_MINT);
  console.log("✅ Pool total staked:", poolAfter?.totalStaked.toString());

  // 6. Deposit rewards
  console.log("\n💰 Depositing", REWARD_AMOUNT.toString(), "USDC rewards...");
  const depositSig = await client.depositRewardsAndSend(
    STAKE_MINT,
    USDC_MINT,
    REWARD_AMOUNT
  );
  console.log("✅ Deposit tx:", depositSig);

  // 7. Check claimable
  const claimable = await client.computeClaimable(STAKE_MINT, wallet.publicKey);
  console.log("\n📊 Claimable:", claimable.toString(), "USDC units");

  // 8. Claim rewards
  console.log("\n🏦 Claiming rewards...");
  const claimSig = await client.claimRewardsAndSend(STAKE_MINT, USDC_MINT);
  console.log("✅ Claim tx:", claimSig);

  // 9. Verify USDC balance
  const usdcAta = getAssociatedTokenAddressSync(USDC_MINT, wallet.publicKey);
  const usdcBalance = await connection.getTokenAccountBalance(usdcAta);
  console.log("\n✅ USDC balance:", usdcBalance.value.uiAmount, "USDC");

  // 10. Final status
  const finalPool = await client.fetchPool(STAKE_MINT);
  console.log("\n📊 Final pool state:");
  console.log("  Total staked:", finalPool?.totalStaked.toString());
  console.log("  Rewards deposited:", finalPool?.totalRewardsDeposited.toString());
  console.log("  Rewards claimed:", finalPool?.totalRewardsClaimed.toString());
  console.log("  Acc reward/share:", finalPool?.accRewardPerShare.toString());

  console.log("\n🎉 Devnet end-to-end test PASSED!");
}

main().catch((e) => {
  console.error("❌ Error:", e.message || e);
  process.exit(1);
});
