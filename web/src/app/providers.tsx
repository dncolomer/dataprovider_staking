"use client";

/**
 * Client-side wallet + connection providers.
 *
 * Wallet discovery order:
 *   1. Explicitly-registered adapters below — always shown in the modal
 *      even if not installed, so the user knows they're supported and can
 *      click through to install them.
 *   2. Any wallet implementing the Solana Wallet Standard that's installed
 *      in the user's browser. This is picked up automatically by
 *      `useStandardWalletAdapters` (called internally by WalletProvider).
 *      Covers Backpack, Glow, Magic Eden Wallet, Exodus, etc. They appear
 *      in the modal marked "Detected" when present.
 *
 * OKX is registered explicitly via the local `OkxWalletAdapter` because
 * its Wallet Standard support is version-dependent; this ensures it
 * appears in the modal regardless.
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
import { OkxWalletAdapter } from "../lib/okxWalletAdapter";

// Required CSS for the default wallet modal UI.
import "@solana/wallet-adapter-react-ui/styles.css";

export default function Providers({ children }: { children: ReactNode }) {
  const wallets = useMemo(
    () => [
      // Popular browser-extension wallets with explicit adapters.
      new PhantomWalletAdapter(),
      new SolflareWalletAdapter(),
      new OkxWalletAdapter(),
      new CoinbaseWalletAdapter(),
      new TrustWalletAdapter(),
      // Hardware wallet support.
      new LedgerWalletAdapter(),
      // Backpack, Glow, Magic Eden, Exodus, and other Solana-Wallet-
      // Standard compliant wallets are auto-detected and added to the
      // modal by WalletProvider if installed in the browser.
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
