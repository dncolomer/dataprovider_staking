/**
 * Site footer: credits, links to the Uncertain Systems project, GitHub repo,
 * and the deployed Solana program id on the explorer.
 */
export function SiteFooter() {
  const PROGRAM_ID = "94Ja6Y8AuzmZHjQiyk2SzvoysnBr3F17nfHGrHm1idAZ";
  return (
    <footer className="site-footer">
      <div className="site-footer__inner">
        <div className="site-footer__col">
          <div className="site-footer__heading">About</div>
          <p className="site-footer__text">
            Dataprovider Staking is a non-custodial staking protocol on
            Solana, built as part of the{" "}
            <a
              href="https://uncertainsystems.xyz"
              target="_blank"
              rel="noopener noreferrer"
            >
              Uncertain Systems
            </a>{" "}
            research program — open-source tools for aligning on-chain
            incentives with long-term token holders.
          </p>
        </div>

        <div className="site-footer__col">
          <div className="site-footer__heading">Links</div>
          <ul className="site-footer__list">
            <li>
              <a
                href="https://github.com/dncolomer/dataprovider_staking"
                target="_blank"
                rel="noopener noreferrer"
              >
                GitHub repository
              </a>
            </li>
            <li>
              <a
                href={`https://explorer.solana.com/address/${PROGRAM_ID}`}
                target="_blank"
                rel="noopener noreferrer"
              >
                Program on Solana Explorer
              </a>
            </li>
            <li>
              <a
                href="https://uncertainsystems.xyz"
                target="_blank"
                rel="noopener noreferrer"
              >
                Uncertain Systems
              </a>
            </li>
          </ul>
        </div>

        <div className="site-footer__col">
          <div className="site-footer__heading">Program</div>
          <p className="site-footer__text mono">{PROGRAM_ID}</p>
          <p className="site-footer__text muted" style={{ fontSize: "0.72rem" }}>
            Mainnet · Anchor 1.0 · Token-2022 compatible
          </p>
        </div>
      </div>

      <div className="site-footer__bottom">
        <span>
          © {new Date().getFullYear()} Uncertain Systems. No warranty. Use
          at your own risk.
        </span>
      </div>
    </footer>
  );
}
