/**
 * Runtime config for the web app. Values flow from NEXT_PUBLIC_* env vars
 * so the same build can point at localnet / devnet / mainnet.
 */
import { PublicKey } from "@solana/web3.js";

export const RPC_URL =
  process.env.NEXT_PUBLIC_RPC_URL ?? "http://127.0.0.1:8899";

export const PROGRAM_ID = (() => {
  const s = process.env.NEXT_PUBLIC_PROGRAM_ID;
  return s ? new PublicKey(s) : undefined;
})();

export const CLUSTER_LABEL =
  process.env.NEXT_PUBLIC_CLUSTER_LABEL ?? "localnet";
