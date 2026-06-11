import type { ChainConfig } from "./chains";

export const formatNative = (n: number, chain: ChainConfig, digits = 4) =>
  `${n.toLocaleString(undefined, { maximumFractionDigits: digits })} ${chain.nativeToken}`;

export const formatUSD = (n: number) =>
  `$${n.toLocaleString(undefined, { maximumFractionDigits: 2 })}`;

export const formatNum = (n: number, digits = 0) =>
  n.toLocaleString(undefined, { maximumFractionDigits: digits });

export const formatPct = (n: number, digits = 2) => `${(n * 100).toFixed(digits)}%`;

export const truncHash = (h: string, l = 6, r = 4) =>
  h.length <= l + r + 2 ? h : `${h.slice(0, l)}…${h.slice(-r)}`;

export const formatAge = (ts: number) => {
  const sec = Math.floor((Date.now() - ts) / 1000);
  if (sec < 60) return `${sec}s ago`;
  if (sec < 3600) return `${Math.floor(sec / 60)}m ago`;
  if (sec < 86400) return `${Math.floor(sec / 3600)}h ago`;
  return `${Math.floor(sec / 86400)}d ago`;
};

export const TOKENS = ["WETH", "USDC", "USDT", "DAI", "WBTC", "ARB", "OP", "MATIC", "LINK", "UNI"];
export const LONGTAIL_TOKENS = ["RARE", "PEPE", "FLOKI", "TURBO", "MOG", "BONK", "WIF", "BRETT"];
export const BRIDGE_PROTOCOLS = ["Stargate", "Across", "Hop", "LayerZero", "Synapse"];
export const AGGREGATORS = ["1inch", "Odos", "ParaSwap", "0x", "KyberSwap", "OpenOcean", "CowSwap", "Bebop"];