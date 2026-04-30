import type { Metadata } from "next";
import type { ReactNode } from "react";
import Providers from "./providers";
import "./globals.css";

export const metadata: Metadata = {
  title: "GHC1CHEM Staking",
  description:
    "Stake $GHC1CHEM on Solana, earn USDC dividends. Multi-mint staking with pro-rata reward distribution.",
  applicationName: "GHC1CHEM Staking",
  keywords: [
    "Solana",
    "Staking",
    "GHC1CHEM",
    "USDC",
    "Dividends",
    "DeFi",
  ],
  authors: [{ name: "dataprovider" }],
  openGraph: {
    title: "GHC1CHEM Staking",
    description: "Stake $GHC1CHEM on Solana, earn USDC dividends.",
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "GHC1CHEM Staking",
    description: "Stake $GHC1CHEM on Solana, earn USDC dividends.",
  },
};

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <body>
        <Providers>{children}</Providers>
      </body>
    </html>
  );
}
