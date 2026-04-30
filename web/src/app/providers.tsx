"use client";

/**
 * Client-side wallet + connection providers.
 *
 * Wallet discovery order:
 *   1. Explicitly-registered adapters below (Phantom, Solflare, Coinbase,
 *      Ledger, Trust) — always shown in the modal even if not installed,
 *      so the user knows they're supported.
 *   2. Any wallet implementing the Solana Wallet Standard that's installed
 *      in the user's browser. This is picked up automatically by
 *      `useStandardWalletAdapters` (called internally by WalletProvider).
 *      Covers Backpack, OKX Wallet, Glow, Magic Eden Wallet, Exodus, and
 *      essentially every modern Solana wallet. They appear in the modal
 *      marked "Detected" when present.
 *
 * Kept in a dedicated client component so the root layout can remain a
 * Server Component.
 */
import { ConnectionProvider, WalletProvider } from "@solana/wallet-adapter-react";
import { WalletModalProvider } from "@solana/wallet-adapter-react-ui";
import {
  CoinbaseWalletAdapter,
  LedgerWalletAdapter,
  PhantomWalletAdapter,
  SolflareWalletAdapter,
  TrustWalletAdapter,
} from "@solana/wallet-adapter-wallets";
import { useMemo, type ReactNode } from "react";
import { RPC_URL } from "../lib/config";

// Required CSS for the default wallet modal UI.
import "@solana/wallet-adapter-react-ui/styles.css";

export default function Providers({ children }: { children: ReactNode }) {
  const wallets = useMemo(
    () => [
      // Popular browser-extension wallets with explicit adapters.
      new PhantomWalletAdapter(),
      new SolflareWalletAdapter(),
      new CoinbaseWalletAdapter(),
      new TrustWalletAdapter(),
      // Hardware wallet support.
      new LedgerWalletAdapter(),
      // OKX Wallet, Backpack, Glow, Magic Eden, Exodus, and other
      // Solana-Wallet-Standard compliant wallets are auto-detected and
      // added to the modal by WalletProvider if installed in the browser.
    ],
    [],
  );
  return (
    <ConnectionProvider endpoint={RPC_URL}>
      <WalletProvider wallets={wallets} autoConnect>
        <WalletModalProvider>{children}</WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
}
