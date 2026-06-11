export type StrategyId = "arb" | "jit" | "jitarb" | "sandwich" | "liquidation" | "longtail" | "aggregator";

export interface DexConfig {
  id: string;
  name: string;
  fork: "UniV2" | "UniV3" | "Curve" | "Balancer" | "Solidly" | "Algebra";
  router: string;
}

export interface LendingProtocolConfig {
  id: string;
  name: string;
  version: string;
  supportedAssets: string[];
  liquidationBonus: number;
  closeFactor: number;
}

export interface ChainConfig {
  id: string;
  name: string;
  nativeToken: string;
  color: string;
  blockTime: number;
  rpcDefault: string;
  explorerBase: string;
  dexes: DexConfig[];
  flashLoanProviders: string[];
  lendingProtocols: LendingProtocolConfig[];
  coingeckoId: string;
  activityMultiplier: number;
  avgTxPerBlock: number;
  gasPriceGwei: number;
  nativeUSD: number;
}

const aaveV3 = (assets: string[]): LendingProtocolConfig => ({
  id: "aave-v3", name: "Aave", version: "v3",
  supportedAssets: assets, liquidationBonus: 0.05, closeFactor: 0.5,
});

export const CHAINS: ChainConfig[] = [
  {
    id: "ethereum", name: "Ethereum", nativeToken: "ETH", color: "#627EEA",
    blockTime: 12, rpcDefault: "https://eth.llamarpc.com",
    explorerBase: "https://etherscan.io/tx/",
    coingeckoId: "ethereum", activityMultiplier: 1.0,
    avgTxPerBlock: 180, gasPriceGwei: 15, nativeUSD: 3200,
    dexes: [
      { id: "uni-v2", name: "Uniswap v2", fork: "UniV2", router: "0x7a25...488D" },
      { id: "uni-v3", name: "Uniswap v3", fork: "UniV3", router: "0xE592...6564" },
      { id: "sushi", name: "SushiSwap", fork: "UniV2", router: "0xd9e1...F50F" },
      { id: "curve", name: "Curve", fork: "Curve", router: "0xfA9a...7d8e" },
      { id: "balancer", name: "Balancer", fork: "Balancer", router: "0xBA12...2BD8" },
      { id: "pancake-v3", name: "PancakeSwap v3", fork: "UniV3", router: "0x1b81...51A4" },
    ],
    flashLoanProviders: ["balancer", "aave"],
    lendingProtocols: [
      aaveV3(["WETH","USDC","USDT","DAI","WBTC"]),
      { id: "aave-v2", name: "Aave", version: "v2", supportedAssets: ["WETH","USDC","DAI","WBTC"], liquidationBonus: 0.05, closeFactor: 0.5 },
      { id: "compound-v3", name: "Compound", version: "v3", supportedAssets: ["WETH","USDC","WBTC"], liquidationBonus: 0.08, closeFactor: 1.0 },
      { id: "compound-v2", name: "Compound", version: "v2", supportedAssets: ["WETH","USDC","DAI"], liquidationBonus: 0.08, closeFactor: 0.5 },
      { id: "maker", name: "MakerDAO", version: "v1", supportedAssets: ["WETH","WBTC"], liquidationBonus: 0.13, closeFactor: 1.0 },
      { id: "euler", name: "Euler", version: "v2", supportedAssets: ["WETH","USDC","DAI"], liquidationBonus: 0.06, closeFactor: 0.5 },
    ],
  },
  {
    id: "polygon", name: "Polygon", nativeToken: "MATIC", color: "#8247E5",
    blockTime: 2, rpcDefault: "https://polygon-rpc.com",
    explorerBase: "https://polygonscan.com/tx/",
    coingeckoId: "matic-network", activityMultiplier: 0.7,
    avgTxPerBlock: 60, gasPriceGwei: 25, nativeUSD: 0.72,
    dexes: [
      { id: "quick-v2", name: "QuickSwap v2", fork: "UniV2", router: "0xa5E0...DFf2" },
      { id: "quick-v3", name: "QuickSwap v3", fork: "Algebra", router: "0xf5b5...11A8" },
      { id: "sushi", name: "SushiSwap", fork: "UniV2", router: "0x1b02...506F" },
      { id: "uni-v3", name: "Uniswap v3", fork: "UniV3", router: "0xE592...6564" },
      { id: "dfyn", name: "DFYN", fork: "UniV2", router: "0xA102...3429" },
      { id: "apeswap", name: "ApeSwap", fork: "UniV2", router: "0xC0788...A607" },
      { id: "meshswap", name: "Meshswap", fork: "UniV2", router: "0x10f4...e8aA" },
    ],
    flashLoanProviders: ["balancer", "aave"],
    lendingProtocols: [
      aaveV3(["WMATIC","WETH","USDC","DAI","WBTC"]),
      { id: "compound-v3", name: "Compound", version: "v3", supportedAssets: ["WETH","USDC"], liquidationBonus: 0.08, closeFactor: 1.0 },
      { id: "silo", name: "Silo", version: "v1", supportedAssets: ["WMATIC","WETH","USDC"], liquidationBonus: 0.07, closeFactor: 0.5 },
    ],
  },
  {
    id: "arbitrum", name: "Arbitrum", nativeToken: "ETH", color: "#28A0F0",
    blockTime: 0.25, rpcDefault: "https://arb1.arbitrum.io/rpc",
    explorerBase: "https://arbiscan.io/tx/",
    coingeckoId: "ethereum", activityMultiplier: 1.2,
    avgTxPerBlock: 8, gasPriceGwei: 0.1, nativeUSD: 3200,
    dexes: [
      { id: "uni-v3", name: "Uniswap v3", fork: "UniV3", router: "0xE592...6564" },
      { id: "sushi", name: "SushiSwap", fork: "UniV2", router: "0x1b02...506F" },
      { id: "camelot-v2", name: "Camelot v2", fork: "UniV2", router: "0xc873...d8d5" },
      { id: "camelot-v3", name: "Camelot v3", fork: "Algebra", router: "0x1F72...c6F5" },
      { id: "gmx", name: "GMX", fork: "UniV2", router: "0xabBc...c1Ba" },
      { id: "zyber", name: "Zyberswap", fork: "Algebra", router: "0xFa58...7d80" },
    ],
    flashLoanProviders: ["balancer", "aave"],
    lendingProtocols: [
      aaveV3(["WETH","USDC","USDT","DAI","WBTC","ARB"]),
      { id: "compound-v3", name: "Compound", version: "v3", supportedAssets: ["WETH","USDC","ARB"], liquidationBonus: 0.08, closeFactor: 1.0 },
      { id: "radiant", name: "Radiant", version: "v2", supportedAssets: ["WETH","USDC","DAI","WBTC"], liquidationBonus: 0.075, closeFactor: 0.5 },
      { id: "silo", name: "Silo", version: "v1", supportedAssets: ["WETH","ARB","USDC"], liquidationBonus: 0.07, closeFactor: 0.5 },
    ],
  },
  {
    id: "base", name: "Base", nativeToken: "ETH", color: "#0052FF",
    blockTime: 2, rpcDefault: "https://mainnet.base.org",
    explorerBase: "https://basescan.org/tx/",
    coingeckoId: "ethereum", activityMultiplier: 0.9,
    avgTxPerBlock: 40, gasPriceGwei: 0.05, nativeUSD: 3200,
    dexes: [
      { id: "uni-v3", name: "Uniswap v3", fork: "UniV3", router: "0x2626...481D" },
      { id: "aerodrome", name: "Aerodrome", fork: "Solidly", router: "0xcF77...8A43" },
      { id: "baseswap", name: "BaseSwap", fork: "UniV2", router: "0x3274...F827" },
      { id: "swapbased", name: "SwapBased", fork: "UniV2", router: "0xaaa3...11B8" },
      { id: "rocketswap", name: "RocketSwap", fork: "UniV2", router: "0x4cf7...A41a" },
    ],
    flashLoanProviders: ["balancer", "aave"],
    lendingProtocols: [
      aaveV3(["WETH","USDC","cbETH"]),
      { id: "compound-v3", name: "Compound", version: "v3", supportedAssets: ["WETH","USDC"], liquidationBonus: 0.08, closeFactor: 1.0 },
      { id: "moonwell", name: "Moonwell", version: "v1", supportedAssets: ["WETH","USDC","cbETH"], liquidationBonus: 0.07, closeFactor: 0.5 },
    ],
  },
  {
    id: "optimism", name: "Optimism", nativeToken: "ETH", color: "#FF0420",
    blockTime: 2, rpcDefault: "https://mainnet.optimism.io",
    explorerBase: "https://optimistic.etherscan.io/tx/",
    coingeckoId: "ethereum", activityMultiplier: 0.6,
    avgTxPerBlock: 12, gasPriceGwei: 0.05, nativeUSD: 3200,
    dexes: [
      { id: "uni-v3", name: "Uniswap v3", fork: "UniV3", router: "0xE592...6564" },
      { id: "velo-v2", name: "Velodrome v2", fork: "Solidly", router: "0xa062...12A8" },
      { id: "beethoven", name: "Beethoven X", fork: "Balancer", router: "0xBA12...2BD8" },
      { id: "zip", name: "ZIP swap", fork: "UniV2", router: "0xE6Df...58B7" },
    ],
    flashLoanProviders: ["balancer", "aave"],
    lendingProtocols: [
      aaveV3(["WETH","USDC","DAI","OP"]),
      { id: "sonne", name: "Sonne Finance", version: "v1", supportedAssets: ["WETH","USDC","OP"], liquidationBonus: 0.08, closeFactor: 0.5 },
      { id: "exactly", name: "Exactly", version: "v1", supportedAssets: ["WETH","USDC","OP"], liquidationBonus: 0.06, closeFactor: 0.5 },
    ],
  },
  {
    id: "bnb", name: "BNB Chain", nativeToken: "BNB", color: "#F0B90B",
    blockTime: 3, rpcDefault: "https://bsc-dataseed.binance.org",
    explorerBase: "https://bscscan.com/tx/",
    coingeckoId: "binancecoin", activityMultiplier: 1.1,
    avgTxPerBlock: 120, gasPriceGwei: 3, nativeUSD: 580,
    dexes: [
      { id: "pancake-v2", name: "PancakeSwap v2", fork: "UniV2", router: "0x10ED...4E56" },
      { id: "pancake-v3", name: "PancakeSwap v3", fork: "UniV3", router: "0x1b81...51A4" },
      { id: "biswap", name: "BiSwap", fork: "UniV2", router: "0x3a6d...0e6e" },
      { id: "apeswap", name: "ApeSwap", fork: "UniV2", router: "0xcF0F...3607" },
      { id: "babyswap", name: "BabySwap", fork: "UniV2", router: "0x8317...e0a1" },
      { id: "mdex", name: "MDEX", fork: "UniV2", router: "0x7DAe...A7c4" },
    ],
    flashLoanProviders: ["aave"],
    lendingProtocols: [
      { id: "venus", name: "Venus", version: "v3", supportedAssets: ["WBNB","USDC","USDT","BTCB"], liquidationBonus: 0.10, closeFactor: 0.5 },
      { id: "radiant", name: "Radiant", version: "v2", supportedAssets: ["WBNB","USDC","BTCB"], liquidationBonus: 0.075, closeFactor: 0.5 },
      { id: "alpaca", name: "Alpaca Finance", version: "v1", supportedAssets: ["WBNB","USDC","BUSD"], liquidationBonus: 0.05, closeFactor: 0.5 },
    ],
  },
  {
    id: "avalanche", name: "Avalanche", nativeToken: "AVAX", color: "#E84142",
    blockTime: 2, rpcDefault: "https://api.avax.network/ext/bc/C/rpc",
    explorerBase: "https://snowtrace.io/tx/",
    coingeckoId: "avalanche-2", activityMultiplier: 0.5,
    avgTxPerBlock: 25, gasPriceGwei: 25, nativeUSD: 35,
    dexes: [
      { id: "joe-v1", name: "Trader Joe v1", fork: "UniV2", router: "0x60aE...30A4" },
      { id: "joe-v2", name: "Trader Joe v2", fork: "UniV3", router: "0xb4315...26E6" },
      { id: "pangolin", name: "Pangolin", fork: "UniV2", router: "0xE54Ca...62a3" },
      { id: "gmx", name: "GMX", fork: "UniV2", router: "0xabBc...c1Ba" },
      { id: "platypus", name: "Platypus", fork: "Curve", router: "0x66EC...7c7e" },
      { id: "curve", name: "Curve", fork: "Curve", router: "0xfA9a...7d8e" },
    ],
    flashLoanProviders: ["aave"],
    lendingProtocols: [
      aaveV3(["WAVAX","WETH","USDC","DAI","WBTC"]),
      { id: "benqi", name: "Benqi", version: "v1", supportedAssets: ["WAVAX","USDC","WETH"], liquidationBonus: 0.10, closeFactor: 0.5 },
      { id: "joe-lending", name: "Trader Joe Lending", version: "v1", supportedAssets: ["WAVAX","USDC","WETH"], liquidationBonus: 0.08, closeFactor: 0.5 },
    ],
  },
];

export const getChain = (id: string) => CHAINS.find((c) => c.id === id) ?? CHAINS[0];