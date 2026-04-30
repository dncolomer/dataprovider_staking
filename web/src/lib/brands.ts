/**
 * Static registry of token metadata for known stake mints.
 *
 * Mapping pool.stakeMint → display name + logo + unit label + pool byline.
 * Unknown mints fall back to a truncated base58 string.
 */
export interface TokenBrand {
  symbol: string;
  name: string;
  /** Path under /public, e.g. "/ghc1chem.avif". */
  logo?: string;
  /** Short description shown under the pool title. */
  tagline?: string;
}

export const TOKEN_BRANDS: Record<string, TokenBrand> = {
  "3pi9trvC6hrMUHHhQnQy5aAPk5CzxAGxsLyiXzshpump": {
    symbol: "GHC1CHEM",
    name: "$GHC1CHEM",
    logo: "/ghc1chem.avif",
    tagline: "Stake $GHC1CHEM, earn USDC dividends.",
  },
};

export function brandFor(mint: string): TokenBrand {
  return (
    TOKEN_BRANDS[mint] ?? {
      symbol: `${mint.slice(0, 4)}…${mint.slice(-4)}`,
      name: `${mint.slice(0, 4)}…${mint.slice(-4)}`,
    }
  );
}
