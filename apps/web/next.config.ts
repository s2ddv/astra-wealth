import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  transpilePackages: ["@astra-wealth/shared"],
  experimental: {
    useTypeScriptCli: false,
  },
};

export default nextConfig;
