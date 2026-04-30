/**
 * BigInt-safe number formatting helpers for token amounts.
 *
 * - `toDecimalString(raw, decimals)` — lossless decimal conversion.
 * - `formatAmount(raw, decimals, opts)` — human display with K/M/B suffix,
 *   comma separators, configurable max fraction digits.
 */
export function toDecimalString(raw: bigint, decimals: number): string {
  if (decimals === 0) return raw.toString();
  const base = 10n ** BigInt(decimals);
  const neg = raw < 0n;
  const abs = neg ? -raw : raw;
  const whole = abs / base;
  const frac = abs % base;
  const fracStr = frac.toString().padStart(decimals, "0");
  return `${neg ? "-" : ""}${whole}.${fracStr}`;
}

export interface FormatAmountOpts {
  /** Max digits after the decimal point. Default 2. */
  maxFraction?: number;
  /** Use compact suffixes (K, M, B, T) for large numbers. Default true. */
  compact?: boolean;
  /** If `true`, always show at least `minFraction` digits. */
  minFraction?: number;
}

/**
 * Format an on-chain token amount (raw u64) as a human-readable string.
 *
 * Examples (decimals=6):
 *   formatAmount(123_456_789n, 6)              -> "123.46"
 *   formatAmount(1_500_000_000n, 6)            -> "1.5K"
 *   formatAmount(1_500_000_000_000n, 6)        -> "1.5M"
 *   formatAmount(0n, 6)                        -> "0"
 *   formatAmount(1_000n, 6)                    -> "0.001"
 *   formatAmount(1n, 6)                        -> "0.000001"
 */
export function formatAmount(
  raw: bigint,
  decimals: number,
  opts: FormatAmountOpts = {},
): string {
  const { maxFraction = 2, compact = true, minFraction = 0 } = opts;
  if (raw === 0n) return "0";

  const asStr = toDecimalString(raw, decimals);
  // Split into whole + fractional parts.
  const [wholeStr, fracStr = ""] = asStr.split(".");
  const whole = BigInt(wholeStr);

  if (compact && whole >= 1_000n) {
    const suffixes = [
      { v: 1_000_000_000_000n, s: "T" },
      { v: 1_000_000_000n, s: "B" },
      { v: 1_000_000n, s: "M" },
      { v: 1_000n, s: "K" },
    ];
    for (const { v, s } of suffixes) {
      if (whole >= v) {
        // Show 2 fractional digits of "thousands"-scaled value.
        const scaled = (whole * 100n) / v;
        const scaledWhole = scaled / 100n;
        const scaledFrac = scaled % 100n;
        const fracOut =
          scaledFrac === 0n ? "" : `.${scaledFrac.toString().padStart(2, "0").replace(/0+$/, "")}`;
        return `${withCommas(scaledWhole.toString())}${fracOut}${s}`;
      }
    }
  }

  // No compact suffix: print whole with commas + limited decimals.
  let trimmedFrac = fracStr.slice(0, maxFraction);
  // Drop trailing zeros only down to minFraction.
  while (
    trimmedFrac.length > minFraction &&
    trimmedFrac.endsWith("0")
  ) {
    trimmedFrac = trimmedFrac.slice(0, -1);
  }
  const wholeFmt = withCommas(whole.toString());
  // For very small amounts (< 1) we might still want more precision so
  // the user sees a non-zero value.
  if (whole === 0n && trimmedFrac === "") {
    // find first non-zero digit
    const firstNonZero = fracStr.search(/[1-9]/);
    if (firstNonZero >= 0) {
      // Keep up to 2 significant digits past the first non-zero
      trimmedFrac = fracStr.slice(0, firstNonZero + 2);
    }
  }
  return trimmedFrac ? `${wholeFmt}.${trimmedFrac}` : wholeFmt;
}

function withCommas(whole: string): string {
  // No Intl — avoid locale jitter. Pure BigInt-safe impl.
  const neg = whole.startsWith("-");
  const digits = neg ? whole.slice(1) : whole;
  let out = "";
  for (let i = 0; i < digits.length; i++) {
    if (i > 0 && (digits.length - i) % 3 === 0) out += ",";
    out += digits[i];
  }
  return neg ? `-${out}` : out;
}

/**
 * Parse a user-entered decimal string into a raw u64 amount.
 * Throws on invalid input / negative / precision loss past `decimals`.
 */
export function parseAmount(input: string, decimals: number): bigint {
  const s = input.trim();
  if (!s) throw new Error("amount is empty");
  const [w, f = ""] = s.split(".");
  if (!/^\d+$/.test(w) || (f && !/^\d+$/.test(f))) {
    throw new Error("not a valid number");
  }
  const padded = (f + "0".repeat(decimals)).slice(0, decimals);
  const raw =
    BigInt(w || "0") * 10n ** BigInt(decimals) + BigInt(padded || "0");
  if (raw <= 0n) throw new Error("amount must be positive");
  return raw;
}

/** Short base58 string, e.g. `3pi9…pump`. */
export function shortAddress(s: string, head = 4, tail = 4): string {
  if (s.length <= head + tail + 1) return s;
  return `${s.slice(0, head)}…${s.slice(-tail)}`;
}
