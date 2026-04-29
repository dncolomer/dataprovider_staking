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
import { useEffect, useMemo, useState, type FormEvent } from "react";
import { useStakingClient } from "../lib/useStakingClient";

interface Props {
  pool: TokenPoolData;
  usdcMint: PublicKey;
  usdcDecimals: number;
  user: PublicKey | null;
  onAction: () => void;
}

/**
 * Renders one pool: stats + stake/unstake/claim forms. All three actions
 * settle pending rewards on-chain, so after any of them we re-trigger the
 * parent's refresh via `onAction`.
 */
export function PoolCard({ pool, usdcMint, usdcDecimals, user, onAction }: Props) {
  const client = useStakingClient();
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

  // Load stake-mint decimals and the user's stake/balance.
  useEffect(() => {
    if (!client) return;
    let cancelled = false;
    (async () => {
      // Resolve the mint's owning token program (classic SPL vs Token-2022)
      // so we derive the right ATA for the "Wallet balance" display. The
      // SDK's stake/unstake calls do the same auto-detection internally.
      let tokenProgram: PublicKey | null = null;
      try {
        const mint = await getMint(client.provider.connection, pool.stakeMint);
        if (cancelled) return;
        setDecimals(mint.decimals);
      } catch {
        /* ignore */
      }
      try {
        tokenProgram = await resolveMintTokenProgram(
          client.provider.connection,
          pool.stakeMint,
        );
      } catch {
        /* ignore; leave balance as null */
      }
      if (user) {
        const us = await client.fetchUserStake(pool.stakeMint, user);
        if (!cancelled) setUserStake(us);
        try {
          if (!tokenProgram) throw new Error("no token program resolved");
          const ata = ataForTokenProgram(pool.stakeMint, user, tokenProgram);
          const acc = await client.provider.connection.getTokenAccountBalance(
            ata,
          );
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

  function fmt(raw: bigint, d: number | null): string {
    if (d == null) return raw.toString();
    const base = 10n ** BigInt(d);
    const whole = raw / base;
    const frac = raw % base;
    return d === 0
      ? whole.toString()
      : `${whole}.${frac.toString().padStart(d, "0").replace(/0+$/, "") || "0"}`;
  }

  function parseAmount(s: string, d: number | null): bigint {
    if (!d && d !== 0) throw new Error("decimals unknown");
    const [w, f = ""] = s.trim().split(".");
    const padded = (f + "0".repeat(d)).slice(0, d);
    const raw = BigInt(w || "0") * 10n ** BigInt(d) + BigInt(padded || "0");
    if (raw <= 0n) throw new Error("amount must be positive");
    return raw;
  }

  async function withBusy(label: string, fn: () => Promise<string>) {
    setStatus(null);
    setBusy(true);
    try {
      const sig = await fn();
      setStatus({ kind: "ok", msg: `${label} ok (${sig.slice(0, 12)}…)` });
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
    const amt = parseAmount(stakeInput, decimals);
    await withBusy("staked", () => client.stakeAndSend(pool.stakeMint, amt));
    setStakeInput("");
  }
  async function onUnstake(e: FormEvent) {
    e.preventDefault();
    if (!client) return;
    const amt = parseAmount(unstakeInput, decimals);
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

  return (
    <section className="card">
      <div className="pool-header">
        <div className="pool-title">Pool</div>
        <div className="mono muted">{pool.stakeMint.toBase58()}</div>
      </div>
      <div className="stat-grid">
        <div className="stat">
          <span className="label">Total staked</span>
          <span className="value">{fmt(pool.totalStaked, decimals)}</span>
        </div>
        <div className="stat">
          <span className="label">Rewards paid in</span>
          <span className="value">
            {fmt(pool.totalRewardsDeposited, usdcDecimals)} USDC
          </span>
        </div>
        <div className="stat">
          <span className="label">Rewards claimed</span>
          <span className="value">
            {fmt(pool.totalRewardsClaimed, usdcDecimals)} USDC
          </span>
        </div>
        <div className="stat">
          <span className="label">Your stake</span>
          <span className="value">
            {fmt(userStake?.amount ?? 0n, decimals)}
          </span>
        </div>
        <div className="stat">
          <span className="label">Your claimable</span>
          <span className="value">{fmt(claimable, usdcDecimals)} USDC</span>
        </div>
        <div className="stat">
          <span className="label">Wallet balance</span>
          <span className="value">{fmt(userBalance ?? 0n, decimals)}</span>
        </div>
      </div>

      {!user && <p className="muted">Connect a wallet to stake.</p>}

      {user && (
        <>
          <form className="row" onSubmit={onStake} style={{ marginBottom: 8 }}>
            <input
              type="text"
              placeholder="amount to stake"
              value={stakeInput}
              onChange={(e) => setStakeInput(e.target.value)}
              disabled={busy}
            />
            <button disabled={busy || !stakeInput} type="submit">
              Stake
            </button>
          </form>
          <form className="row" onSubmit={onUnstake} style={{ marginBottom: 8 }}>
            <input
              type="text"
              placeholder="amount to unstake"
              value={unstakeInput}
              onChange={(e) => setUnstakeInput(e.target.value)}
              disabled={busy}
            />
            <button
              className="secondary"
              disabled={busy || !unstakeInput}
              type="submit"
            >
              Unstake
            </button>
          </form>
          <div className="row">
            <button disabled={busy || claimable <= 0n} onClick={onClaim}>
              Claim {fmt(claimable, usdcDecimals)} USDC
            </button>
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
