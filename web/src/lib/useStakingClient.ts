"use client";

import { AnchorProvider, type Wallet as AnchorWallet } from "@coral-xyz/anchor";
import { StakingClient } from "@dataprovider/staking-sdk";
import {
  useAnchorWallet,
  useConnection,
} from "@solana/wallet-adapter-react";
import { useMemo } from "react";
import { PROGRAM_ID } from "./config";

/**
 * Build a StakingClient bound to the connected wallet. Returns `null` when
 * no wallet is connected (so components can render a connect prompt).
 */
export function useStakingClient(): StakingClient | null {
  const { connection } = useConnection();
  const wallet = useAnchorWallet();
  return useMemo(() => {
    if (!wallet) return null;
    // `useAnchorWallet()` returns an object shape-compatible with AnchorWallet.
    const provider = new AnchorProvider(connection, wallet as AnchorWallet, {
      commitment: "confirmed",
    });
    return new StakingClient(provider, { programId: PROGRAM_ID });
  }, [connection, wallet]);
}
