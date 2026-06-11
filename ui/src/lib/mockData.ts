import type { ChainConfig, StrategyId } from "./chains";
import { LONGTAIL_TOKENS } from "./formatters";

let seed = 1337;
const rng = () => {
  seed = (seed * 9301 + 49297) % 233280;
  return seed / 233280;
};
export const seedRng = (s = 1337) => { seed = s; };
const between = (a: number, b: number) => a + rng() * (b - a);
const choice = <T,>(arr: T[]) => arr[Math.floor(rng() * arr.length)];
const hex = (n: number) => "0x" + Array.from({ length: n }, () => Math.floor(rng() * 16).toString(16)).join("");

export interface SimulationTraceStep { label: string; value?: string; sub?: string }
export interface SimulationTrace { title: string; steps: SimulationTraceStep[]; result: { gross: number; cost: number; net: number } }

export interface Opportunity {
  id: string;
  txHash: string;
  blockNumber: number;
  timestamp: number;
  strategy: StrategyId;
  grossRevenue: number;
  gasCost: number;
  flashLoanFee: number;
  builderTip: number;
  netProfit: number;
  result: "profitable" | "below_threshold" | "reverted";
  explorerUrl: string;
  dexPath?: string[];
  tokenPair?: string;
  flashLoanProvider?: string;
  flashLoanSize?: number;
  victimTxHash?: string;
  frontRunSize?: number;
  victimSlippage?: number;
  grossCapture?: number;
  protocol?: string;
  borrowerAddress?: string;
  healthFactor?: number;
  debtRepaid?: number;
  collateralSeized?: number;
  collateralAsset?: string;
  liquidationBonus?: number;
  route?: string[];
  hopCount?: number;
  priceImpact?: number;
  crossChain?: boolean;
  bridgeUsed?: string | null;
  aggregator?: string;
  aggregatorAlt?: string;
  aggregatorSplits?: number;
  spreadBps?: number;
  probeSizeUSD?: number;
  simulationTrace: SimulationTrace;
}

export interface StrategyMetrics {
  strategy: StrategyId;
  count: number;
  profitable: number;
  grossRevenue: number;
  gasFees: number;
  netProfit: number;
  netProfitUSD: number;
  roi: number;
  avgPerOpp: number;
  bestOpp: number;
}

export interface DexMetrics {
  dex: string; fork: string; txCount: number; opportunities: number; profitable: number; revenue: number; avgProfit: number;
}
export interface LiquidationAnalytics {
  byProtocol: { protocol: string; scanned: number; targeted: number; profitable: number; debtRepaid: number; collateralSeized: number; netProfit: number }[];
  hfDistribution: { bucket: string; count: number }[];
}
export interface LongtailAnalytics {
  topRoutes: { route: string[]; hops: number; avgImpact: number; executions: number; totalProfit: number }[];
  scatterData: { hops: number; profit: number; route: string }[];
}

export interface SummaryMetrics {
  total: number; profitable: number; grossRevenue: number; netProfit: number; netProfitUSD: number; totalCost: number; bestStrategy: StrategyId | null; bestSingleOpp: number;
}

const PROFIT_PROFILES: Record<StrategyId, { min: number; max: number; profitRate: number; baseGas: number }> = {
  arb:         { min: 0.002, max: 0.05,  profitRate: 0.60, baseGas: 0.003 },
  jit:         { min: 0.005, max: 0.12,  profitRate: 0.50, baseGas: 0.005 },
  jitarb:      { min: 0.008, max: 0.08,  profitRate: 0.52, baseGas: 0.006 },
  sandwich:    { min: 0.001, max: 0.04,  profitRate: 0.45, baseGas: 0.004 },
  liquidation: { min: 0.01,  max: 0.30,  profitRate: 0.70, baseGas: 0.004 },
  longtail:    { min: 0.0005,max: 0.02,  profitRate: 0.35, baseGas: 0.002 },
  aggregator:  { min: 0.001, max: 0.04,  profitRate: 0.55, baseGas: 0.0025 },
};

const buildTrace = (strategy: StrategyId, gross: number, gas: number, net: number, ctx: Record<string, unknown>): SimulationTrace => {
  const block = ctx.blockNumber as number;
  if (strategy === "sandwich") {
    return {
      title: "Sandwich trace",
      steps: [
        { label: "Block", value: `#${block.toLocaleString()}` },
        { label: "Victim tx", value: ctx.victimTxHash as string, sub: `swap on Uniswap v3 · slippage ${((ctx.victimSlippage as number) * 100).toFixed(2)}%` },
        { label: "Front-run", value: hex(8), sub: `buy ${(ctx.frontRunSize as number).toFixed(2)} before victim · gas +50%` },
        { label: "Victim executes", sub: "at worse price (slippage absorbed)" },
        { label: "Back-run", value: hex(8), sub: "sell back · capture spread" },
        { label: "Gross capture", value: `${gross.toFixed(5)}` },
        { label: "Gas (×1.5)", value: `−${(gas * 0.6).toFixed(5)}` },
        { label: "DEX fees (×2)", value: `−${(gas * 0.4).toFixed(5)}` },
      ],
      result: { gross, cost: gas, net },
    };
  }
  if (strategy === "liquidation") {
    return {
      title: "Liquidation trace",
      steps: [
        { label: "Block", value: `#${block.toLocaleString()}` },
        { label: "Protocol", value: ctx.protocol as string },
        { label: "Position", value: ctx.borrowerAddress as string, sub: `health factor ${(ctx.healthFactor as number).toFixed(2)}` },
        { label: "Debt repaid", value: `${(ctx.debtRepaid as number).toFixed(2)} USDC` },
        { label: "Flash loan", value: `${(ctx.debtRepaid as number).toFixed(2)} USDC`, sub: "Balancer · fee 0%" },
        { label: "Seize", value: `${(ctx.collateralSeized as number).toFixed(4)} ${ctx.collateralAsset}` , sub: `bonus ${(((ctx.liquidationBonus as number) || 0) * 100).toFixed(1)}%` },
        { label: "Repay flash loan", sub: "atomic" },
        { label: "Gas", value: `−${gas.toFixed(5)}` },
      ],
      result: { gross, cost: gas, net },
    };
  }
  if (strategy === "longtail") {
    const route = (ctx.route as string[]) || [];
    return {
      title: "Long-tail arb trace",
      steps: [
        { label: "Block", value: `#${block.toLocaleString()}` },
        { label: "Route", value: route.join(" → "), sub: `${ctx.hopCount} hops` },
        ...route.slice(0, -1).map((t, i) => ({
          label: `Leg ${i + 1}`,
          value: `${t} → ${route[i + 1]}`,
          sub: `via ${choice(["Uniswap v3", "SushiSwap", "Camelot v2"])}`,
        })),
        { label: "Price impact", value: `${((ctx.priceImpact as number) * 100).toFixed(2)}%` },
        ...(ctx.crossChain ? [{ label: "Bridge", value: ctx.bridgeUsed as string }] : []),
        { label: "Gross profit", value: gross.toFixed(5) },
        { label: "Gas", value: `−${gas.toFixed(5)}` },
      ],
      result: { gross, cost: gas, net },
    };
  }
  if (strategy === "aggregator") {
    return {
      title: "Aggregator arb trace",
      steps: [
        { label: "Block", value: `#${block.toLocaleString()}` },
        { label: "Token pair", value: ctx.tokenPair as string },
        { label: "Probe size", value: `$${(ctx.probeSizeUSD as number).toLocaleString()}` },
        { label: "Best aggregator", value: ctx.aggregator as string, sub: `${ctx.aggregatorSplits} split(s)` },
        { label: "Alt aggregator", value: ctx.aggregatorAlt as string, sub: `quote ${((ctx.spreadBps as number) / 100).toFixed(2)}% worse` },
        { label: "Execute", sub: `route via ${ctx.aggregator} · sell into ${ctx.aggregatorAlt} pool` },
        { label: "Spread captured", value: `${((ctx.spreadBps as number) / 100).toFixed(3)}%` },
        { label: "Gross profit", value: gross.toFixed(5) },
        { label: "Gas", value: `−${gas.toFixed(5)}` },
      ],
      result: { gross, cost: gas, net },
    };
  }
  // arb / jit / jitarb
  const path = (ctx.dexPath as string[]) || [];
  return {
    title: strategy === "arb" ? "Arbitrage trace" : strategy === "jit" ? "JIT trace" : "JIT+Arb trace",
    steps: [
      { label: "Block", value: `#${block.toLocaleString()}` },
      { label: "Token pair", value: ctx.tokenPair as string },
      { label: "Path", value: path.join(" → ") },
      ...(ctx.flashLoanProvider ? [{ label: "Flash loan", value: `${(ctx.flashLoanSize as number).toFixed(2)} via ${ctx.flashLoanProvider}` }] : []),
      { label: "Gross revenue", value: gross.toFixed(5) },
      { label: "Gas", value: `−${gas.toFixed(5)}` },
    ],
    result: { gross, cost: gas, net },
  };
};

export function generateOpportunities(
  opts: { lastDays: number; chain: ChainConfig; enabledStrategies: StrategyId[]; dexes: { id: string; name: string; fork: string }[]; lendingProtocols: { id: string; name: string }[]; minProfit: number },
): Opportunity[] {
  seedRng(Math.floor(opts.lastDays * 13 + opts.enabledStrategies.length * 7 + opts.chain.id.length * 17));
  const baseCount = Math.round(opts.lastDays * 4 * opts.chain.activityMultiplier);
  const opps: Opportunity[] = [];
  const now = Date.now();
  for (const strat of opts.enabledStrategies) {
    const profile = PROFIT_PROFILES[strat];
    const stratCount = Math.round(baseCount * (strat === "arb" ? 1.2 : strat === "longtail" ? 1.4 : strat === "liquidation" ? 0.4 : 1));
    for (let i = 0; i < stratCount; i++) {
      const gross = between(profile.min, profile.max);
      const gas = profile.baseGas * between(0.7, 1.6);
      const flashFee = strat === "jitarb" || (strat === "liquidation" && rng() > 0.3) ? gross * 0.0009 : 0;
      const builderTip = gross * 0.1;
      const net = gross - gas - flashFee - builderTip;
      const profitable = rng() < profile.profitRate && net > opts.minProfit;
      const reverted = !profitable && rng() < 0.1;
      const result: Opportunity["result"] = profitable ? "profitable" : reverted ? "reverted" : "below_threshold";
      const blockNumber = 19_800_000 - i * 7 - Math.floor(rng() * 100);
      const ts = now - Math.floor(rng() * opts.lastDays * 86400 * 1000);
      const txHash = hex(64);

      const ctx: Record<string, unknown> = { blockNumber };

      const opp: Opportunity = {
        id: `${strat}-${i}-${blockNumber}`,
        txHash,
        blockNumber,
        timestamp: ts,
        strategy: strat,
        grossRevenue: gross,
        gasCost: gas,
        flashLoanFee: flashFee,
        builderTip,
        netProfit: profitable ? net : reverted ? -gas : net,
        result,
        explorerUrl: opts.chain.explorerBase + txHash,
        simulationTrace: { title: "", steps: [], result: { gross, cost: gas, net } },
      };

      if (strat === "arb" || strat === "jit" || strat === "jitarb") {
        const path = [choice(opts.dexes), choice(opts.dexes)].map((d) => d.name);
        opp.dexPath = path;
        opp.tokenPair = `${choice(["WETH","USDC","WBTC","DAI"])}/${choice(["USDC","USDT","DAI"])}`;
        if (strat === "jitarb") {
          opp.flashLoanProvider = choice(["Balancer v2", "Aave v3"]);
          opp.flashLoanSize = between(5, 80);
        }
        ctx.dexPath = path; ctx.tokenPair = opp.tokenPair; ctx.flashLoanProvider = opp.flashLoanProvider; ctx.flashLoanSize = opp.flashLoanSize;
      }
      if (strat === "sandwich") {
        opp.victimTxHash = hex(64);
        opp.frontRunSize = between(0.5, 3.5);
        opp.victimSlippage = between(0.003, 0.02);
        opp.grossCapture = gross;
        Object.assign(ctx, { victimTxHash: opp.victimTxHash, frontRunSize: opp.frontRunSize, victimSlippage: opp.victimSlippage });
      }
      if (strat === "liquidation") {
        const proto = opts.lendingProtocols.length ? choice(opts.lendingProtocols) : { id: "aave-v3", name: "Aave v3" };
        const asset = choice(["WETH", "USDC", "WBTC", "DAI"]);
        opp.protocol = `${proto.name}`;
        opp.borrowerAddress = hex(40);
        opp.healthFactor = between(0.5, 0.99);
        opp.debtRepaid = between(200, 8000);
        opp.collateralAsset = asset;
        opp.liquidationBonus = 0.05 + rng() * 0.05;
        opp.collateralSeized = (opp.debtRepaid * (1 + opp.liquidationBonus)) / (opts.chain.nativeUSD || 3000);
        Object.assign(ctx, { protocol: opp.protocol, borrowerAddress: opp.borrowerAddress, healthFactor: opp.healthFactor, debtRepaid: opp.debtRepaid, collateralSeized: opp.collateralSeized, collateralAsset: asset, liquidationBonus: opp.liquidationBonus });
      }
      if (strat === "longtail") {
        const hops = (2 + Math.floor(rng() * 3)) as 2 | 3 | 4;
        const route = ["WETH"];
        for (let h = 0; h < hops - 1; h++) route.push(choice(LONGTAIL_TOKENS));
        route.push("WETH");
        opp.route = route;
        opp.hopCount = hops;
        opp.priceImpact = between(0.002, 0.025);
        opp.crossChain = rng() < 0.15;
        opp.bridgeUsed = opp.crossChain ? choice(["Stargate", "Across", "Hop"]) : null;
        Object.assign(ctx, { route, hopCount: hops, priceImpact: opp.priceImpact, crossChain: opp.crossChain, bridgeUsed: opp.bridgeUsed });
      }
      if (strat === "aggregator") {
        const aggs = ["1inch", "Odos", "ParaSwap", "0x", "KyberSwap", "OpenOcean"];
        const a = choice(aggs); let b = choice(aggs); while (b === a) b = choice(aggs);
        opp.aggregator = a;
        opp.aggregatorAlt = b;
        opp.aggregatorSplits = 1 + Math.floor(rng() * 3);
        opp.spreadBps = Math.round(between(8, 55));
        opp.probeSizeUSD = Math.round(between(5_000, 120_000));
        opp.tokenPair = `${choice(["WETH","USDC","WBTC","DAI"])}/${choice(["USDC","USDT","DAI"])}`;
        opp.dexPath = [a, b];
        Object.assign(ctx, { aggregator: a, aggregatorAlt: b, aggregatorSplits: opp.aggregatorSplits, spreadBps: opp.spreadBps, probeSizeUSD: opp.probeSizeUSD, tokenPair: opp.tokenPair });
      }

      opp.simulationTrace = buildTrace(strat, gross, gas, opp.netProfit, ctx);
      opps.push(opp);
    }
  }
  return opps.sort((a, b) => b.blockNumber - a.blockNumber);
}

export function aggregate(opps: Opportunity[], chain: ChainConfig, dexes: { id: string; name: string; fork: string }[]): {
  summary: SummaryMetrics;
  byStrategy: Record<string, StrategyMetrics>;
  byDex: DexMetrics[];
  liquidationAnalytics: LiquidationAnalytics | null;
  longtailAnalytics: LongtailAnalytics | null;
} {
  const byStrategy: Record<string, StrategyMetrics> = {};
  let grossRevenue = 0, netProfit = 0, totalCost = 0, profitable = 0, bestOpp = 0;
  let bestStrategy: StrategyId | null = null;
  let bestStrategyTotal = -Infinity;

  for (const o of opps) {
    const s = byStrategy[o.strategy] ?? (byStrategy[o.strategy] = { strategy: o.strategy, count: 0, profitable: 0, grossRevenue: 0, gasFees: 0, netProfit: 0, netProfitUSD: 0, roi: 0, avgPerOpp: 0, bestOpp: 0 });
    s.count++;
    s.grossRevenue += o.grossRevenue;
    s.gasFees += o.gasCost + o.flashLoanFee + o.builderTip;
    if (o.result === "profitable") { s.profitable++; profitable++; s.netProfit += o.netProfit; }
    s.bestOpp = Math.max(s.bestOpp, o.netProfit);
    grossRevenue += o.grossRevenue;
    totalCost += o.gasCost + o.flashLoanFee + o.builderTip;
    if (o.result === "profitable") netProfit += o.netProfit;
    bestOpp = Math.max(bestOpp, o.netProfit);
  }
  Object.values(byStrategy).forEach((s) => {
    s.netProfitUSD = s.netProfit * chain.nativeUSD;
    s.avgPerOpp = s.count ? s.netProfit / s.count : 0;
    // ROI heuristic: net profit / (capital * days) — use net profit / gross as proxy
    s.roi = s.grossRevenue ? (s.netProfit / s.grossRevenue) * 100 : 0;
    if (s.netProfit > bestStrategyTotal) { bestStrategyTotal = s.netProfit; bestStrategy = s.strategy; }
  });

  // DEX metrics
  const byDexMap = new Map<string, DexMetrics>();
  for (const d of dexes) byDexMap.set(d.name, { dex: d.name, fork: d.fork, txCount: 0, opportunities: 0, profitable: 0, revenue: 0, avgProfit: 0 });
  for (const o of opps) {
    if (!o.dexPath) continue;
    for (const dn of o.dexPath) {
      const m = byDexMap.get(dn); if (!m) continue;
      m.opportunities++; m.txCount += 2;
      if (o.result === "profitable") { m.profitable++; m.revenue += o.grossRevenue; }
    }
  }
  const byDex = Array.from(byDexMap.values()).map((m) => ({ ...m, avgProfit: m.profitable ? m.revenue / m.profitable : 0 })).sort((a, b) => b.revenue - a.revenue);

  // Liquidation analytics
  const liqOpps = opps.filter((o) => o.strategy === "liquidation");
  let liquidationAnalytics: LiquidationAnalytics | null = null;
  if (liqOpps.length) {
    const byProto = new Map<string, LiquidationAnalytics["byProtocol"][number]>();
    for (const o of liqOpps) {
      const k = o.protocol || "Unknown";
      const row = byProto.get(k) ?? { protocol: k, scanned: 0, targeted: 0, profitable: 0, debtRepaid: 0, collateralSeized: 0, netProfit: 0 };
      row.scanned += 3; row.targeted++;
      if (o.result === "profitable") { row.profitable++; row.debtRepaid += o.debtRepaid || 0; row.collateralSeized += o.collateralSeized || 0; row.netProfit += o.netProfit; }
      byProto.set(k, row);
    }
    const buckets = [
      { bucket: "0.0–0.5", min: 0, max: 0.5, count: 0 },
      { bucket: "0.5–0.7", min: 0.5, max: 0.7, count: 0 },
      { bucket: "0.7–0.9", min: 0.7, max: 0.9, count: 0 },
      { bucket: "0.9–1.0", min: 0.9, max: 1.01, count: 0 },
    ];
    for (const o of liqOpps) {
      if (o.result !== "profitable") continue;
      const b = buckets.find((x) => (o.healthFactor || 1) >= x.min && (o.healthFactor || 1) < x.max);
      if (b) b.count++;
    }
    liquidationAnalytics = {
      byProtocol: Array.from(byProto.values()).sort((a, b) => b.netProfit - a.netProfit),
      hfDistribution: buckets.map(({ bucket, count }) => ({ bucket, count })),
    };
  }

  // Longtail analytics
  const ltOpps = opps.filter((o) => o.strategy === "longtail");
  let longtailAnalytics: LongtailAnalytics | null = null;
  if (ltOpps.length) {
    const byRoute = new Map<string, LongtailAnalytics["topRoutes"][number]>();
    for (const o of ltOpps) {
      const key = (o.route || []).join("→");
      const row = byRoute.get(key) ?? { route: o.route || [], hops: o.hopCount || 0, avgImpact: 0, executions: 0, totalProfit: 0 };
      row.executions++;
      row.avgImpact = (row.avgImpact * (row.executions - 1) + (o.priceImpact || 0)) / row.executions;
      if (o.result === "profitable") row.totalProfit += o.netProfit;
      byRoute.set(key, row);
    }
    longtailAnalytics = {
      topRoutes: Array.from(byRoute.values()).sort((a, b) => b.totalProfit - a.totalProfit).slice(0, 10),
      scatterData: ltOpps.filter((o) => o.result === "profitable").map((o) => ({ hops: o.hopCount || 2, profit: o.netProfit, route: (o.route || []).join("→") })),
    };
  }

  const summary: SummaryMetrics = {
    total: opps.length,
    profitable,
    grossRevenue,
    netProfit,
    netProfitUSD: netProfit * chain.nativeUSD,
    totalCost,
    bestStrategy,
    bestSingleOpp: bestOpp,
  };
  return { summary, byStrategy, byDex, liquidationAnalytics, longtailAnalytics };
}