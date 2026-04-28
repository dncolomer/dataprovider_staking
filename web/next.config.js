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
    return config;
  },
};

module.exports = nextConfig;
