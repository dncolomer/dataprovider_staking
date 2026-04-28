/**
 * High-level client for the dataprovider_staking program.
 *
 * This wraps the Anchor-generated client and provides convenience methods for
 * every instruction, PDA resolution, and account fetching. Designed to be
 * usable from both Node.js (admin scripts) and the browser (investor UI).
 */
import {
  AnchorProvider,
  BN,
  Program,
  type Idl,
  type Wallet,
} from "@coral-xyz/anchor";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import {
  Connection,
  PublicKey,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
  type Commitment,
  type ConfirmOptions,
  type TransactionInstruction,
} from "@solana/web3.js";

import idlJson from "../idl/dataprovider_staking.json";
import type { DataproviderStaking } from "./idl-types";
import {
  DATAPROVIDER_STAKING_PROGRAM_ID,
  ACC_PRECISION,
} from "./constants";
import {
  findConfigPda,
  findPoolPda,
  findRewardVaultPda,
  findStakeVaultPda,
  findUserStakePda,
  findVaultAuthorityPda,
} from "./pda";

export interface ClientOpts {
  /** Program id override. Defaults to the baked-in one. */
  programId?: PublicKey;
  /** Commitment for reads. Defaults to the provider's. */
  commitment?: Commitment;
  /** Confirm options for writes. */
  confirmOpts?: ConfirmOptions;
}

export interface GlobalConfigData {
  admin: PublicKey;
  pendingAdmin: PublicKey;
  usdcMint: PublicKey;
  poolCount: number;
  bump: number;
}

export interface TokenPoolData {
  stakeMint: PublicKey;
  stakeVault: PublicKey;
  rewardVault: PublicKey;
  totalStaked: bigint;
  accRewardPerShare: bigint;
  totalRewardsDeposited: bigint;
  totalRewardsClaimed: bigint;
  bump: number;
  vaultAuthorityBump: number;
}

export interface UserStakeData {
  owner: PublicKey;
  stakeMint: PublicKey;
  amount: bigint;
  rewardDebt: bigint;
  pendingRewards: bigint;
  totalClaimed: bigint;
  bump: number;
}

/**
 * Convert `BN`/`number` fields on the raw anchor account to bigint uniformly.
 * Anchor 0.31 returns BN for u64 fields, number for u8.
 */
function bnToBigint(v: BN | bigint | number): bigint {
  if (typeof v === "bigint") return v;
  if (typeof v === "number") return BigInt(v);
  return BigInt(v.toString());
}

/**
 * StakingClient is the main entry point. It's thin: nearly every call
 * returns an `Instruction` or a sent-transaction signature, so callers can
 * compose or batch however they like.
 */
export class StakingClient {
  readonly program: Program<DataproviderStaking>;
  readonly programId: PublicKey;
  readonly provider: AnchorProvider;

  constructor(provider: AnchorProvider, opts: ClientOpts = {}) {
    this.provider = provider;
    this.programId = opts.programId ?? DATAPROVIDER_STAKING_PROGRAM_ID;
    // Inject the programId into the IDL so Anchor resolves the right pubkey.
    const idl = { ...(idlJson as Idl), address: this.programId.toBase58() };
    this.program = new Program<DataproviderStaking>(idl as unknown as DataproviderStaking, provider);
  }

  /**
   * Convenience factory: build a client from a Connection + Wallet without
   * needing to wire up an AnchorProvider yourself.
   */
  static from(
    connection: Connection,
    wallet: Wallet,
    opts: ClientOpts = {},
  ): StakingClient {
    const provider = new AnchorProvider(
      connection,
      wallet,
      opts.confirmOpts ?? AnchorProvider.defaultOptions(),
    );
    return new StakingClient(provider, opts);
  }

  // ===== PDA helpers (instance-scoped so they pick up overridden programId) =====

  configPda(): PublicKey {
    return findConfigPda(this.programId)[0];
  }
  poolPda(stakeMint: PublicKey): PublicKey {
    return findPoolPda(stakeMint, this.programId)[0];
  }
  vaultAuthorityPda(stakeMint: PublicKey): PublicKey {
    return findVaultAuthorityPda(stakeMint, this.programId)[0];
  }
  stakeVaultPda(stakeMint: PublicKey): PublicKey {
    return findStakeVaultPda(stakeMint, this.programId)[0];
  }
  rewardVaultPda(stakeMint: PublicKey): PublicKey {
    return findRewardVaultPda(stakeMint, this.programId)[0];
  }
  userStakePda(stakeMint: PublicKey, owner: PublicKey): PublicKey {
    return findUserStakePda(stakeMint, owner, this.programId)[0];
  }

  // ===== Reads =====

  async fetchConfig(): Promise<GlobalConfigData | null> {
    const acc = await this.program.account.globalConfig.fetchNullable(
      this.configPda(),
    );
    if (!acc) return null;
    return {
      admin: acc.admin,
      pendingAdmin: acc.pendingAdmin,
      usdcMint: acc.usdcMint,
      poolCount: acc.poolCount,
      bump: acc.bump,
    };
  }

  async fetchPool(stakeMint: PublicKey): Promise<TokenPoolData | null> {
    const acc = await this.program.account.tokenPool.fetchNullable(
      this.poolPda(stakeMint),
    );
    if (!acc) return null;
    return {
      stakeMint: acc.stakeMint,
      stakeVault: acc.stakeVault,
      rewardVault: acc.rewardVault,
      totalStaked: bnToBigint(acc.totalStaked),
      accRewardPerShare: bnToBigint(acc.accRewardPerShare),
      totalRewardsDeposited: bnToBigint(acc.totalRewardsDeposited),
      totalRewardsClaimed: bnToBigint(acc.totalRewardsClaimed),
      bump: acc.bump,
      vaultAuthorityBump: acc.vaultAuthorityBump,
    };
  }

  async fetchUserStake(
    stakeMint: PublicKey,
    owner: PublicKey,
  ): Promise<UserStakeData | null> {
    const acc = await this.program.account.userStake.fetchNullable(
      this.userStakePda(stakeMint, owner),
    );
    if (!acc) return null;
    return {
      owner: acc.owner,
      stakeMint: acc.stakeMint,
      amount: bnToBigint(acc.amount),
      rewardDebt: bnToBigint(acc.rewardDebt),
      pendingRewards: bnToBigint(acc.pendingRewards),
      totalClaimed: bnToBigint(acc.totalClaimed),
      bump: acc.bump,
    };
  }

  /**
   * Return the off-chain-computed *current* claimable amount for a user,
   * including unsettled accrual since the user's last settle point.
   *
   * This is what a UI should display; it mirrors the on-chain `settle` math.
   */
  async computeClaimable(
    stakeMint: PublicKey,
    owner: PublicKey,
  ): Promise<bigint> {
    const [pool, user] = await Promise.all([
      this.fetchPool(stakeMint),
      this.fetchUserStake(stakeMint, owner),
    ]);
    if (!pool || !user) return 0n;
    const unsettled =
      user.amount === 0n
        ? 0n
        : (user.amount * pool.accRewardPerShare) / ACC_PRECISION -
          user.rewardDebt;
    return user.pendingRewards + unsettled;
  }

  /** Enumerate every pool account registered under this program. */
  async listPools(): Promise<Array<{ address: PublicKey; data: TokenPoolData }>> {
    const raw = await this.program.account.tokenPool.all();
    return raw.map((r) => ({
      address: r.publicKey,
      data: {
        stakeMint: r.account.stakeMint,
        stakeVault: r.account.stakeVault,
        rewardVault: r.account.rewardVault,
        totalStaked: bnToBigint(r.account.totalStaked),
        accRewardPerShare: bnToBigint(r.account.accRewardPerShare),
        totalRewardsDeposited: bnToBigint(r.account.totalRewardsDeposited),
        totalRewardsClaimed: bnToBigint(r.account.totalRewardsClaimed),
        bump: r.account.bump,
        vaultAuthorityBump: r.account.vaultAuthorityBump,
      },
    }));
  }

  // ===== Instruction builders =====
  //
  // Each returns a `TransactionInstruction` so callers compose at will. A
  // matching `*AndSend` variant actually sends it via the provider wallet.

  initializeIx(admin: PublicKey, usdcMint: PublicKey): Promise<TransactionInstruction> {
    return this.program.methods
      .initialize()
      .accountsPartial({
        payer: this.provider.publicKey!,
        admin,
        usdcMint,
      })
      .instruction();
  }

  /**
   * Build an `add_pool` instruction. The caller must pass the USDC mint that
   * matches `GlobalConfig.usdc_mint`; use `fetchConfig()` to obtain it.
   */
  addPoolIx(
    admin: PublicKey,
    stakeMint: PublicKey,
    usdcMint: PublicKey,
  ): Promise<TransactionInstruction> {
    return this.program.methods
      .addPool()
      .accountsPartial({
        payer: this.provider.publicKey!,
        admin,
        stakeMint,
        usdcMint,
      })
      .instruction();
  }

  stakeIx(
    user: PublicKey,
    stakeMint: PublicKey,
    amount: bigint | number | BN,
  ): Promise<TransactionInstruction> {
    const userAta = getAssociatedTokenAddressSync(stakeMint, user);
    return this.program.methods
      .stake(new BN(amount.toString()))
      .accountsPartial({
        user,
        stakeMint,
        userTokenAccount: userAta,
      })
      .instruction();
  }

  unstakeIx(
    user: PublicKey,
    stakeMint: PublicKey,
    amount: bigint | number | BN,
  ): Promise<TransactionInstruction> {
    const userAta = getAssociatedTokenAddressSync(stakeMint, user);
    return this.program.methods
      .unstake(new BN(amount.toString()))
      .accountsPartial({
        user,
        stakeMint,
        userTokenAccount: userAta,
      })
      .instruction();
  }

  depositRewardsIx(
    admin: PublicKey,
    stakeMint: PublicKey,
    usdcMint: PublicKey,
    amount: bigint | number | BN,
  ): Promise<TransactionInstruction> {
    const adminUsdc = getAssociatedTokenAddressSync(usdcMint, admin);
    return this.program.methods
      .depositRewards(new BN(amount.toString()))
      .accountsPartial({
        admin,
        stakeMint,
        usdcMint,
        adminUsdcAccount: adminUsdc,
      })
      .instruction();
  }

  claimRewardsIx(
    user: PublicKey,
    stakeMint: PublicKey,
    usdcMint: PublicKey,
  ): Promise<TransactionInstruction> {
    const userUsdc = getAssociatedTokenAddressSync(usdcMint, user);
    return this.program.methods
      .claimRewards()
      .accountsPartial({
        user,
        stakeMint,
        usdcMint,
        userUsdcAccount: userUsdc,
      })
      .instruction();
  }

  proposeAdminIx(admin: PublicKey, newAdmin: PublicKey): Promise<TransactionInstruction> {
    return this.program.methods
      .proposeAdmin()
      .accountsPartial({ admin, newAdmin })
      .instruction();
  }

  acceptAdminIx(newAdmin: PublicKey): Promise<TransactionInstruction> {
    return this.program.methods
      .acceptAdmin()
      .accountsPartial({ newAdmin })
      .instruction();
  }

  cancelAdminProposalIx(admin: PublicKey): Promise<TransactionInstruction> {
    return this.program.methods
      .cancelAdminProposal()
      .accountsPartial({ admin })
      .instruction();
  }

  // ===== Convenience `send` variants using the provider wallet =====
  //
  // These assume the provider wallet IS the signer required by the chosen
  // instruction. Use the *Ix builders for more control (co-signers, batching).

  async initializeAndSend(admin: PublicKey, usdcMint: PublicKey): Promise<string> {
    return this.program.methods
      .initialize()
      .accountsPartial({ payer: this.provider.publicKey!, admin, usdcMint })
      .rpc();
  }

  async addPoolAndSend(
    admin: PublicKey,
    stakeMint: PublicKey,
    usdcMint: PublicKey,
  ): Promise<string> {
    return this.program.methods
      .addPool()
      .accountsPartial({ payer: this.provider.publicKey!, admin, stakeMint, usdcMint })
      .rpc();
  }

  async stakeAndSend(
    stakeMint: PublicKey,
    amount: bigint | number | BN,
  ): Promise<string> {
    const user = this.provider.publicKey!;
    const userAta = getAssociatedTokenAddressSync(stakeMint, user);
    return this.program.methods
      .stake(new BN(amount.toString()))
      .accountsPartial({ user, stakeMint, userTokenAccount: userAta })
      .rpc();
  }

  async unstakeAndSend(
    stakeMint: PublicKey,
    amount: bigint | number | BN,
  ): Promise<string> {
    const user = this.provider.publicKey!;
    const userAta = getAssociatedTokenAddressSync(stakeMint, user);
    return this.program.methods
      .unstake(new BN(amount.toString()))
      .accountsPartial({ user, stakeMint, userTokenAccount: userAta })
      .rpc();
  }

  async claimRewardsAndSend(
    stakeMint: PublicKey,
    usdcMint: PublicKey,
  ): Promise<string> {
    const user = this.provider.publicKey!;
    const userUsdc = getAssociatedTokenAddressSync(usdcMint, user);
    return this.program.methods
      .claimRewards()
      .accountsPartial({ user, stakeMint, usdcMint, userUsdcAccount: userUsdc })
      .rpc();
  }

  async depositRewardsAndSend(
    stakeMint: PublicKey,
    usdcMint: PublicKey,
    amount: bigint | number | BN,
  ): Promise<string> {
    const admin = this.provider.publicKey!;
    const adminUsdc = getAssociatedTokenAddressSync(usdcMint, admin);
    return this.program.methods
      .depositRewards(new BN(amount.toString()))
      .accountsPartial({ admin, stakeMint, usdcMint, adminUsdcAccount: adminUsdc })
      .rpc();
  }

  // Expose a few helpers that callers sometimes need inline.
  static readonly TOKEN_PROGRAM_ID = TOKEN_PROGRAM_ID;
  static readonly ASSOCIATED_TOKEN_PROGRAM_ID = ASSOCIATED_TOKEN_PROGRAM_ID;
  static readonly SYSTEM_PROGRAM_ID = SystemProgram.programId;
  static readonly RENT_SYSVAR = SYSVAR_RENT_PUBKEY;
}
