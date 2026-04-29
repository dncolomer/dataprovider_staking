/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  // Transpile our local SDK workspace package.
  transpilePackages: ["@dataprovider/staking-sdk"],
  webpack: (config) => {
    // Node built-ins used by @solana/web3.js aren't needed in the browser.
    config.resolve.fallback = {
      ...config.resolve.fallback,
      fs: false,
      os: false,
      path: false,
      crypto: false,
    };
    // pino (transitive dep via WalletConnect) optionally tries to load
    // pino-pretty for dev-friendly logs. It's not installed and not needed
    // in production; mark it external so webpack doesn't try to resolve it.
    config.externals = config.externals || [];
    config.externals.push({ "pino-pretty": "commonjs pino-pretty" });
    return config;
  },
};

module.exports = nextConfig;
