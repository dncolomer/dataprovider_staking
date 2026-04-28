"use client";

/**
 * Client-side wallet + connection providers. Kept in a dedicated client
 * component so the root layout can remain a Server Component (which lets the
 * app import the SDK lazily only where it's used).
 */
import { ConnectionProvider, WalletProvider } from "@solana/wallet-adapter-react";
import { WalletModalProvider } from "@solana/wallet-adapter-react-ui";
import {
  PhantomWalletAdapter,
  SolflareWalletAdapter,
} from "@solana/wallet-adapter-wallets";
import { useMemo, type ReactNode } from "react";
import { RPC_URL } from "../lib/config";

// Required CSS for the default wallet modal UI.
import "@solana/wallet-adapter-react-ui/styles.css";

export default function Providers({ children }: { children: ReactNode }) {
  const wallets = useMemo(
    () => [new PhantomWalletAdapter(), new SolflareWalletAdapter()],
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
