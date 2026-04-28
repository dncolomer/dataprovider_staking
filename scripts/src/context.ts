/**
 * Shared context helpers for admin CLI scripts.
 *
 * Handles: loading a keypair from disk, building an AnchorProvider, and
 * instantiating the StakingClient. All CLI commands go through `loadContext`
 * so that the user experience (flags, errors) is consistent.
 */
import { AnchorProvider, Wallet } from "@coral-xyz/anchor";
import { StakingClient } from "@dataprovider/staking-sdk";
import {
  Connection,
  Keypair,
  PublicKey,
  clusterApiUrl,
} from "@solana/web3.js";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";

export interface CliGlobalOpts {
  cluster: string;
  keypair: string;
  programId?: string;
}

export interface CliContext {
  connection: Connection;
  wallet: Wallet;
  keypair: Keypair;
  client: StakingClient;
}

/**
 * Resolve an rpc URL. Accepts:
 *   - a shortcut name: "localnet", "devnet", "mainnet-beta"
 *   - a full URL starting with "http"
 */
export function resolveRpc(cluster: string): string {
  if (cluster.startsWith("http")) return cluster;
  switch (cluster) {
    case "localnet":
    case "local":
      return "http://127.0.0.1:8899";
    case "devnet":
      return clusterApiUrl("devnet");
    case "mainnet":
    case "mainnet-beta":
      return clusterApiUrl("mainnet-beta");
    default:
      throw new Error(`Unknown cluster: ${cluster}`);
  }
}

/** Load a Solana CLI-style keypair file (JSON array of 64 bytes). */
export function loadKeypair(keypairPath: string): Keypair {
  const expanded = keypairPath.startsWith("~")
    ? path.join(os.homedir(), keypairPath.slice(1))
    : keypairPath;
  if (!fs.existsSync(expanded)) {
    throw new Error(`Keypair file not found: ${expanded}`);
  }
  const raw = JSON.parse(fs.readFileSync(expanded, "utf8")) as number[];
  return Keypair.fromSecretKey(Uint8Array.from(raw));
}

export function loadContext(opts: CliGlobalOpts): CliContext {
  const connection = new Connection(resolveRpc(opts.cluster), "confirmed");
  const keypair = loadKeypair(opts.keypair);
  const wallet = new Wallet(keypair);
  const provider = new AnchorProvider(connection, wallet, {
    commitment: "confirmed",
    preflightCommitment: "confirmed",
  });
  const client = new StakingClient(provider, {
    programId: opts.programId ? new PublicKey(opts.programId) : undefined,
  });
  return { connection, wallet, keypair, client };
}
