"use client";

import {
  ACC_PRECISION,
  ataForTokenProgram,
  resolveMintTokenProgram,
  type TokenPoolData,
  type UserStakeData,
} from "@dataprovider/staking-sdk";
import { getMint } from "@solana/spl-token";
import { PublicKey } from "@solana/web3.js";
import Image from "next/image";
import { useEffect, useMemo, useState, type FormEvent } from "react";
import { brandFor } from "../lib/brands";
import {
  formatAmount,
  parseAmount,
  shortAddress,
  toDecimalString,
} from "../lib/format";
import { useStakingClient } from "../lib/useStakingClient";

interface Props {
  pool: TokenPoolData;
  usdcMint: PublicKey;
  usdcDecimals: number;
  user: PublicKey | null;
  onAction: () => void;
}

/**
 * One-pool dashboard card: branded header, headline stats, stake/unstake/claim.
 * Auto-detects the stake mint's token program (SPL vs Token-2022) so it
 * handles both.
 */
export function PoolCard({
  pool,
  usdcMint,
  usdcDecimals,
  user,
  onAction,
}: Props) {
  const client = useStakingClient();
  const brand = brandFor(pool.stakeMint.toBase58());

  const [decimals, setDecimals] = useState<number | null>(null);
  const [userStake, setUserStake] = useState<UserStakeData | null>(null);
  const [userBalance, setUserBalance] = useState<bigint | null>(null);
  const [stakeInput, setStakeInput] = useState("");
  const [unstakeInput, setUnstakeInput] = useState("");
  const [status, setStatus] = useState<{
    kind: "ok" | "err";
    msg: string;
  } | null>(null);
  const [busy, setBusy] = useState(false);
  const [copied, setCopied] = useState(false);

  // Load stake-mint decimals, token program, and the user's stake/balance.
  useEffect(() => {
    if (!client) return;
    let cancelled = false;
    (async () => {
      let tokenProgram: PublicKey | null = null;
      try {
        tokenProgram = await resolveMintTokenProgram(
          client.provider.connection,
          pool.stakeMint,
        );
      } catch {
        /* leave null */
      }
      if (tokenProgram) {
        try {
          const mint = await getMint(
            client.provider.connection,
            pool.stakeMint,
            undefined,
            tokenProgram,
          );
          if (cancelled) return;
          setDecimals(mint.decimals);
        } catch {
          /* ignore */
        }
      }
      if (user) {
        const us = await client.fetchUserStake(pool.stakeMint, user);
        if (!cancelled) setUserStake(us);
        try {
          if (!tokenProgram) throw new Error("no token program resolved");
          const ata = ataForTokenProgram(pool.stakeMint, user, tokenProgram);
          const acc =
            await client.provider.connection.getTokenAccountBalance(ata);
          if (!cancelled) setUserBalance(BigInt(acc.value.amount));
        } catch {
          if (!cancelled) setUserBalance(0n);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [client, pool.stakeMint, user]);

  const claimable = useMemo<bigint>(() => {
    if (!userStake) return 0n;
    const accrued =
      userStake.amount === 0n
        ? 0n
        : (userStake.amount * pool.accRewardPerShare) / ACC_PRECISION -
          userStake.rewardDebt;
    return userStake.pendingRewards + accrued;
  }, [userStake, pool.accRewardPerShare]);

  /** Safe token amount formatter; falls back to raw if decimals unknown. */
  function fmtToken(raw: bigint, d: number | null): string {
    if (d == null) return raw.toString();
    return formatAmount(raw, d);
  }
  /** USDC-specific formatter (always 2 fractional). */
  function fmtUsdc(raw: bigint): string {
    return formatAmount(raw, usdcDecimals, { maxFraction: 2, minFraction: 2 });
  }

  async function withBusy(label: string, fn: () => Promise<string>) {
    setStatus(null);
    setBusy(true);
    try {
      const sig = await fn();
      setStatus({
        kind: "ok",
        msg: `${label} ok · ${sig.slice(0, 8)}…`,
      });
      onAction();
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      setStatus({ kind: "err", msg });
    } finally {
      setBusy(false);
    }
  }

  async function onStake(e: FormEvent) {
    e.preventDefault();
    if (!client) return;
    if (decimals == null) return;
    let amt: bigint;
    try {
      amt = parseAmount(stakeInput, decimals);
    } catch (err) {
      setStatus({
        kind: "err",
        msg: err instanceof Error ? err.message : String(err),
      });
      return;
    }
    if (userBalance == null || userBalance === 0n) {
      setStatus({
        kind: "err",
        msg: `You don't hold any ${brand.symbol} yet. Acquire some first to stake.`,
      });
      return;
    }
    if (amt > userBalance) {
      setStatus({
        kind: "err",
        msg: `Amount exceeds your wallet balance (${fmtToken(userBalance, decimals)} ${brand.symbol}).`,
      });
      return;
    }
    await withBusy("staked", () => client.stakeAndSend(pool.stakeMint, amt));
    setStakeInput("");
  }

  async function onUnstake(e: FormEvent) {
    e.preventDefault();
    if (!client) return;
    if (decimals == null) return;
    let amt: bigint;
    try {
      amt = parseAmount(unstakeInput, decimals);
    } catch (err) {
      setStatus({
        kind: "err",
        msg: err instanceof Error ? err.message : String(err),
      });
      return;
    }
    await withBusy("unstaked", () =>
      client.unstakeAndSend(pool.stakeMint, amt),
    );
    setUnstakeInput("");
  }

  async function onClaim() {
    if (!client) return;
    await withBusy("claimed", () =>
      client.claimRewardsAndSend(pool.stakeMint, usdcMint),
    );
  }

  function setStakeMax() {
    if (userBalance == null || decimals == null) return;
    setStakeInput(toDecimalString(userBalance, decimals));
  }
  function setUnstakeMax() {
    if (!userStake || decimals == null) return;
    setUnstakeInput(toDecimalString(userStake.amount, decimals));
  }

  async function copyMint() {
    try {
      await navigator.clipboard.writeText(pool.stakeMint.toBase58());
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      /* ignore */
    }
  }

  return (
    <section className="card pool-card">
      <div className="pool-head">
        <div className="pool-identity">
          {brand.logo ? (
            <Image
              src={brand.logo}
              alt={`${brand.symbol} logo`}
              width={56}
              height={56}
              className="pool-logo"
              priority
            />
          ) : (
            <div className="pool-logo pool-logo--placeholder">
              {brand.symbol.slice(0, 2)}
            </div>
          )}
          <div className="pool-meta">
            <div className="pool-title">{brand.name}</div>
            {brand.tagline && (
              <div className="pool-tagline">{brand.tagline}</div>
            )}
            <button
              className="mint-chip"
              onClick={copyMint}
              title="Copy mint address"
              type="button"
            >
              <span className="mono">
                {shortAddress(pool.stakeMint.toBase58(), 4, 4)}
              </span>
              <span className="mint-chip__action">
                {copied ? "copied" : "copy"}
              </span>
            </button>
          </div>
        </div>
      </div>

      <div className="stat-grid">
        <Stat
          label="Total staked"
          value={fmtToken(pool.totalStaked, decimals)}
          unit={brand.symbol}
        />
        <Stat
          label="Rewards paid in"
          value={fmtUsdc(pool.totalRewardsDeposited)}
          unit="USDC"
          accent
        />
        <Stat
          label="Rewards claimed"
          value={fmtUsdc(pool.totalRewardsClaimed)}
          unit="USDC"
        />
        <Stat
          label="Your stake"
          value={fmtToken(userStake?.amount ?? 0n, decimals)}
          unit={brand.symbol}
          muted={!user || (userStake?.amount ?? 0n) === 0n}
        />
        <Stat
          label="Your claimable"
          value={fmtUsdc(claimable)}
          unit="USDC"
          accent={claimable > 0n}
          muted={!user}
        />
        <Stat
          label="Wallet balance"
          value={fmtToken(userBalance ?? 0n, decimals)}
          unit={brand.symbol}
          muted={!user}
        />
      </div>

      {!user && (
        <p className="muted pool-hint">Connect a wallet to stake.</p>
      )}

      {user && decimals == null && (
        <p className="muted pool-hint">Loading pool mint info…</p>
      )}

      {user && decimals != null && (
        <>
          {userBalance === 0n && (
            <p className="muted pool-hint">
              You don&apos;t hold any {brand.symbol} yet. Acquire some first to
              stake.
            </p>
          )}

          <div className="actions">
            <form className="action" onSubmit={onStake}>
              <label className="action__label">Stake</label>
              <div className="action__input-row">
                <input
                  type="text"
                  placeholder={`amount in ${brand.symbol}`}
                  value={stakeInput}
                  onChange={(e) => setStakeInput(e.target.value)}
                  disabled={busy || userBalance === 0n}
                  inputMode="decimal"
                />
                <button
                  type="button"
                  className="chip"
                  onClick={setStakeMax}
                  disabled={busy || (userBalance ?? 0n) === 0n}
                >
                  max
                </button>
                <button
                  disabled={busy || !stakeInput || userBalance === 0n}
                  type="submit"
                >
                  Stake
                </button>
              </div>
            </form>

            <form className="action" onSubmit={onUnstake}>
              <label className="action__label">Unstake</label>
              <div className="action__input-row">
                <input
                  type="text"
                  placeholder={`amount in ${brand.symbol}`}
                  value={unstakeInput}
                  onChange={(e) => setUnstakeInput(e.target.value)}
                  disabled={
                    busy || !userStake || userStake.amount === 0n
                  }
                  inputMode="decimal"
                />
                <button
                  type="button"
                  className="chip"
                  onClick={setUnstakeMax}
                  disabled={
                    busy || !userStake || userStake.amount === 0n
                  }
                >
                  max
                </button>
                <button
                  className="secondary"
                  disabled={
                    busy ||
                    !unstakeInput ||
                    !userStake ||
                    userStake.amount === 0n
                  }
                  type="submit"
                >
                  Unstake
                </button>
              </div>
            </form>

            <div className="action">
              <label className="action__label">Rewards</label>
              <button
                className="claim-btn"
                disabled={busy || claimable <= 0n}
                onClick={onClaim}
              >
                {claimable > 0n
                  ? `Claim ${fmtUsdc(claimable)} USDC`
                  : "No rewards to claim"}
              </button>
            </div>
          </div>
        </>
      )}

      {status && (
        <div className={status.kind === "ok" ? "success" : "error"}>
          {status.msg}
        </div>
      )}
    </section>
  );
}

function Stat({
  label,
  value,
  unit,
  accent = false,
  muted = false,
}: {
  label: string;
  value: string;
  unit?: string;
  accent?: boolean;
  muted?: boolean;
}) {
  return (
    <div
      className={[
        "stat",
        accent ? "stat--accent" : "",
        muted ? "stat--muted" : "",
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <span className="label">{label}</span>
      <span className="value">
        {value}
        {unit && <span className="unit">{unit}</span>}
      </span>
    </div>
  );
}
