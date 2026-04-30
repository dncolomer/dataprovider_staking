"use client";

import {
  type GlobalConfigData,
  type TokenPoolData,
} from "@dataprovider/staking-sdk";
import { getMint } from "@solana/spl-token";
import { useWallet } from "@solana/wallet-adapter-react";
import { WalletMultiButton } from "@solana/wallet-adapter-react-ui";
import { useCallback, useEffect, useState } from "react";
import { PoolCard } from "../components/PoolCard";
import { SiteFooter } from "../components/SiteFooter";
import { CLUSTER_LABEL } from "../lib/config";
import { shortAddress } from "../lib/format";
import { useStakingClient } from "../lib/useStakingClient";

export default function HomePage() {
  const client = useStakingClient();
  const { publicKey } = useWallet();

  const [config, setConfig] = useState<GlobalConfigData | null>(null);
  const [pools, setPools] = useState<
    Array<{ address: string; data: TokenPoolData }>
  >([]);
  const [usdcDecimals, setUsdcDecimals] = useState<number>(6);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!client) {
      setConfig(null);
      setPools([]);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const cfg = await client.fetchConfig();
      setConfig(cfg);
      if (cfg) {
        const usdc = await getMint(client.provider.connection, cfg.usdcMint);
        setUsdcDecimals(usdc.decimals);
      }
      const ps = await client.listPools();
      setPools(
        ps.map((p) => ({ address: p.address.toBase58(), data: p.data })),
      );
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, [client]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return (
    <>
      <main>
        <header className="site">
          <div>
            <span className="brand">Dataprovider Staking</span>
            <span className="cluster">{CLUSTER_LABEL}</span>
          </div>
          <WalletMultiButton />
        </header>

        <section className="hero">
          <h1 className="hero__title">
            Stake tokens, earn <span className="accent">USDC</span>&nbsp;dividends.
          </h1>
          <p className="hero__copy">
            A non-custodial, multi-mint staking protocol on Solana. The admin
            periodically distributes USDC revenue pro-rata to stakers. Classic
            MasterChef-style accounting, Token-2022 compatible, fully
            open-source.
          </p>
          <p className="hero__byline">
            Part of the{" "}
            <a
              href="https://uncertainsystems.xyz"
              target="_blank"
              rel="noopener noreferrer"
            >
              Uncertain Systems
            </a>{" "}
            research program &middot;{" "}
            <a
              href="https://github.com/dncolomer/dataprovider_staking"
              target="_blank"
              rel="noopener noreferrer"
            >
              view source on GitHub
            </a>
          </p>
        </section>

        {!client && (
          <div className="card">
            <p>Connect a wallet to view pool stats and stake.</p>
          </div>
        )}

        {client && !config && !loading && (
          <div className="card">
            <p className="muted">
              The program is not initialized on this cluster yet, or the
              program id doesn&apos;t match this deployment.
            </p>
          </div>
        )}

        {config && (
          <div className="card config-card">
            <div className="stat-grid">
              <div className="stat">
                <span className="label">Admin</span>
                <span className="value mono config-pk" title={config.admin.toBase58()}>
                  {shortAddress(config.admin.toBase58(), 6, 6)}
                </span>
              </div>
              <div className="stat">
                <span className="label">USDC mint</span>
                <span
                  className="value mono config-pk"
                  title={config.usdcMint.toBase58()}
                >
                  {shortAddress(config.usdcMint.toBase58(), 6, 6)}
                </span>
              </div>
              <div className="stat">
                <span className="label">Pools</span>
                <span className="value">{config.poolCount}</span>
              </div>
            </div>
          </div>
        )}

        {error && <div className="card error">{error}</div>}

        {pools.length === 0 && config && (
          <div className="card muted">
            No pools have been created yet. Check back when the admin has
            registered a stake mint.
          </div>
        )}

        {config &&
          pools.map(({ address, data }) => (
            <PoolCard
              key={address}
              pool={data}
              usdcMint={config.usdcMint}
              usdcDecimals={usdcDecimals}
              user={publicKey}
              onAction={refresh}
            />
          ))}
      </main>
      <SiteFooter />
    </>
  );
}
