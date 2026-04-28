#!/usr/bin/env node
/**
 * dps-admin: admin CLI for the dataprovider_staking program.
 *
 * Subcommands:
 *   initialize        Create the GlobalConfig. Signer becomes admin unless --admin is passed.
 *   add-pool          Register a new stake mint (creates stake+reward vaults).
 *   deposit-rewards   Send USDC into a pool's reward vault for pro-rata distribution.
 *   propose-admin     Start a 2-step admin rotation.
 *   accept-admin      Accept a pending admin rotation (run as the new admin).
 *   cancel-admin      Cancel a pending admin rotation.
 *   status            Dump config + all pools.
 *   pool              Dump a single pool by stake mint.
 *
 * Usage example:
 *   dps-admin --cluster localnet --keypair ~/.config/solana/id.json status
 */
import { Command } from "commander";
import {
  createAssociatedTokenAccountIdempotent,
  getMint,
} from "@solana/spl-token";
import { PublicKey, Transaction } from "@solana/web3.js";

import { loadContext, CliGlobalOpts } from "./context";

const program = new Command();
program
  .name("dps-admin")
  .description("Admin CLI for the dataprovider_staking program")
  .option("-c, --cluster <cluster>", "rpc cluster or URL", "localnet")
  .option(
    "-k, --keypair <path>",
    "payer/admin keypair path",
    `${process.env.HOME}/.config/solana/id.json`,
  )
  .option("-p, --program-id <pubkey>", "override program id");

function globalOpts(): CliGlobalOpts {
  return program.opts<CliGlobalOpts>();
}

// ----- initialize -----
program
  .command("initialize")
  .description("Create the GlobalConfig singleton")
  .requiredOption("--usdc-mint <pubkey>", "USDC mint used for rewards")
  .option("--admin <pubkey>", "admin pubkey (defaults to signer)")
  .action(async (opts: { usdcMint: string; admin?: string }) => {
    const ctx = loadContext(globalOpts());
    const admin = opts.admin
      ? new PublicKey(opts.admin)
      : ctx.keypair.publicKey;
    if (!admin.equals(ctx.keypair.publicKey)) {
      throw new Error(
        "Admin must sign `initialize`. If --admin is someone else, run this script as them instead.",
      );
    }
    const sig = await ctx.client.initializeAndSend(
      admin,
      new PublicKey(opts.usdcMint),
    );
    console.log(`initialized. tx: ${sig}`);
    const cfg = await ctx.client.fetchConfig();
    console.log("config:", {
      admin: cfg?.admin.toBase58(),
      usdcMint: cfg?.usdcMint.toBase58(),
      poolCount: cfg?.poolCount,
    });
  });

// ----- add-pool -----
program
  .command("add-pool")
  .description("Register a new stake mint as a pool")
  .requiredOption("--stake-mint <pubkey>", "stake-mint to register")
  .action(async (opts: { stakeMint: string }) => {
    const ctx = loadContext(globalOpts());
    const cfg = await ctx.client.fetchConfig();
    if (!cfg) throw new Error("GlobalConfig not found; run `initialize` first");
    if (!cfg.admin.equals(ctx.keypair.publicKey)) {
      throw new Error(
        `Signer ${ctx.keypair.publicKey.toBase58()} is not current admin ${cfg.admin.toBase58()}`,
      );
    }
    const stakeMint = new PublicKey(opts.stakeMint);
    const sig = await ctx.client.addPoolAndSend(
      cfg.admin,
      stakeMint,
      cfg.usdcMint,
    );
    console.log(`pool created for ${stakeMint.toBase58()}. tx: ${sig}`);
  });

// ----- deposit-rewards -----
program
  .command("deposit-rewards")
  .description("Deposit USDC rewards into a pool, split pro-rata to stakers")
  .requiredOption("--stake-mint <pubkey>", "target pool's stake mint")
  .requiredOption("--amount <value>", "USDC amount in base units (lamports)")
  .action(async (opts: { stakeMint: string; amount: string }) => {
    const ctx = loadContext(globalOpts());
    const cfg = await ctx.client.fetchConfig();
    if (!cfg) throw new Error("GlobalConfig not found");
    if (!cfg.admin.equals(ctx.keypair.publicKey)) {
      throw new Error("Signer is not admin");
    }
    const stakeMint = new PublicKey(opts.stakeMint);
    // Ensure admin has a USDC ATA (idempotent).
    const ata = await createAssociatedTokenAccountIdempotent(
      ctx.connection,
      ctx.keypair,
      cfg.usdcMint,
      ctx.keypair.publicKey,
    );
    console.log(`admin USDC ATA: ${ata.toBase58()}`);
    const amount = BigInt(opts.amount);
    const sig = await ctx.client.depositRewardsAndSend(
      stakeMint,
      cfg.usdcMint,
      amount,
    );
    console.log(`deposited ${amount} USDC units to pool. tx: ${sig}`);
  });

// ----- propose-admin -----
program
  .command("propose-admin")
  .description("Propose a new admin (2-step rotation)")
  .requiredOption("--new-admin <pubkey>", "proposed new admin")
  .action(async (opts: { newAdmin: string }) => {
    const ctx = loadContext(globalOpts());
    const cfg = await ctx.client.fetchConfig();
    if (!cfg) throw new Error("GlobalConfig not found");
    if (!cfg.admin.equals(ctx.keypair.publicKey)) {
      throw new Error("Signer is not admin");
    }
    const newAdmin = new PublicKey(opts.newAdmin);
    const ix = await ctx.client.proposeAdminIx(cfg.admin, newAdmin);
    const sig = await ctx.client.provider.sendAndConfirm!(
      new Transaction().add(ix),
    );
    console.log(`proposed ${newAdmin.toBase58()}. tx: ${sig}`);
  });

// ----- accept-admin -----
program
  .command("accept-admin")
  .description("Accept a pending admin rotation (run as the new admin)")
  .action(async () => {
    const ctx = loadContext(globalOpts());
    const cfg = await ctx.client.fetchConfig();
    if (!cfg) throw new Error("GlobalConfig not found");
    if (!cfg.pendingAdmin.equals(ctx.keypair.publicKey)) {
      throw new Error(
        `Signer ${ctx.keypair.publicKey.toBase58()} is not the pending admin ${cfg.pendingAdmin.toBase58()}`,
      );
    }
    const ix = await ctx.client.acceptAdminIx(ctx.keypair.publicKey);
    const sig = await ctx.client.provider.sendAndConfirm!(
      new Transaction().add(ix),
    );
    console.log(`accepted admin. tx: ${sig}`);
  });

// ----- cancel-admin -----
program
  .command("cancel-admin")
  .description("Cancel a pending admin rotation")
  .action(async () => {
    const ctx = loadContext(globalOpts());
    const cfg = await ctx.client.fetchConfig();
    if (!cfg) throw new Error("GlobalConfig not found");
    if (!cfg.admin.equals(ctx.keypair.publicKey)) {
      throw new Error("Signer is not admin");
    }
    const ix = await ctx.client.cancelAdminProposalIx(cfg.admin);
    const sig = await ctx.client.provider.sendAndConfirm!(
      new Transaction().add(ix),
    );
    console.log(`cancelled. tx: ${sig}`);
  });

// ----- status -----
program
  .command("status")
  .description("Show GlobalConfig + all registered pools")
  .action(async () => {
    const ctx = loadContext(globalOpts());
    const cfg = await ctx.client.fetchConfig();
    if (!cfg) {
      console.log("GlobalConfig: <not initialized>");
      return;
    }
    console.log("GlobalConfig:");
    console.log("  admin:        ", cfg.admin.toBase58());
    console.log(
      "  pendingAdmin: ",
      cfg.pendingAdmin.equals(PublicKey.default)
        ? "(none)"
        : cfg.pendingAdmin.toBase58(),
    );
    console.log("  usdcMint:     ", cfg.usdcMint.toBase58());
    console.log("  poolCount:    ", cfg.poolCount);

    const pools = await ctx.client.listPools();
    console.log(`\nPools (${pools.length}):`);
    for (const { address, data } of pools) {
      console.log(`  [${address.toBase58()}]`);
      console.log(`    stakeMint:             ${data.stakeMint.toBase58()}`);
      console.log(`    totalStaked:           ${data.totalStaked}`);
      console.log(`    totalRewardsDeposited: ${data.totalRewardsDeposited}`);
      console.log(`    totalRewardsClaimed:   ${data.totalRewardsClaimed}`);
      console.log(`    accRewardPerShare:     ${data.accRewardPerShare}`);
    }
  });

// ----- pool (single-pool detail) -----
program
  .command("pool")
  .description("Show pool state by stake-mint")
  .requiredOption("--stake-mint <pubkey>", "pool stake mint")
  .action(async (opts: { stakeMint: string }) => {
    const ctx = loadContext(globalOpts());
    const stakeMint = new PublicKey(opts.stakeMint);
    const pool = await ctx.client.fetchPool(stakeMint);
    if (!pool) {
      console.log("<no pool for that mint>");
      return;
    }
    const vaultAuth = ctx.client.vaultAuthorityPda(stakeMint);
    const stakeVault = ctx.client.stakeVaultPda(stakeMint);
    const rewardVault = ctx.client.rewardVaultPda(stakeMint);
    console.log(`pool @ ${ctx.client.poolPda(stakeMint).toBase58()}`);
    console.log(`  vaultAuthority: ${vaultAuth.toBase58()}`);
    console.log(`  stakeVault:     ${stakeVault.toBase58()}`);
    console.log(`  rewardVault:    ${rewardVault.toBase58()}`);
    console.log(`  totalStaked:    ${pool.totalStaked}`);
    console.log(`  rewardsIn:      ${pool.totalRewardsDeposited}`);
    console.log(`  rewardsClaimed: ${pool.totalRewardsClaimed}`);
    console.log(`  acc/share:      ${pool.accRewardPerShare}`);

    // Peek at mint decimals for convenience.
    try {
      const m = await getMint(ctx.connection, stakeMint);
      console.log(`  (stake mint decimals: ${m.decimals})`);
    } catch {
      /* non-fatal */
    }
  });

program.parseAsync(process.argv).catch((e) => {
  console.error("error:", e.message ?? e);
  process.exit(1);
});
