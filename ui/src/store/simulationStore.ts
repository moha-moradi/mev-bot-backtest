import { create } from "zustand";
import { CHAINS, getChain, type ChainConfig, type StrategyId } from "@/lib/chains";
import { generateOpportunities, aggregate, type Opportunity, type SummaryMetrics, type StrategyMetrics, type DexMetrics, type LiquidationAnalytics, type LongtailAnalytics } from "@/lib/mockData";
import { simulate as apiSimulate, subscribeToStatus, getCompletedResults, deleteHistoryRun, getHistory, getExportJsonUrl, getExportCsvUrl } from "@/lib/api/endpoints";
import { adaptCompletedResults, adaptHistoryEntry, API_STAGE_LABELS, API_STAGE_SUB } from "@/lib/api/adapters";
import type { SimulateRequest } from "@/lib/api/types";

export type WindowMode = "days" | "range" | "block";

export interface StrategyConfigs {
  arb: { enabled: boolean; minSpread: number; maxHops: 1 | 2 | 3 | 4; tokenWhitelist: string[] };
  jit: { enabled: boolean; minTVL: number; tickWidth: number; targetPools: string[] };
  jitarb: { enabled: boolean; flashProvider: "balancer" | "aave"; maxLoanSize: number; minSpreadAfterFees: number };
  sandwich: { enabled: boolean; frontRunSize: number; maxVictimSlippage: number; feeTier: "0.05" | "0.3" | "1"; gasMultiplier: number; minNetProfit: number };
  liquidation: { enabled: boolean; protocol: string; minHealthFactor: number; minCollateralUSD: number; maxRepayAmount: number; collateralPreference: string[]; useFlashLoan: boolean; flashProvider: "balancer" | "aave" };
  longtail: { enabled: boolean; routeDepth: 2 | 3 | 4; minLiquidity: number; maxPriceImpact: number; crossChain: boolean; bridgeProtocols: string[]; bridgeFee: number; tokenUniverse: "major" | "midcap" | "longtail"; minProfitAfterBridge: number };
  aggregator: { enabled: boolean; aggregators: string[]; minSpread: number; probeSizeUSD: number; maxSplits: 1 | 2 | 3; includeRfq: boolean; gasMultiplier: number };
}

export interface PipelineStage {
  id: string; label: string; sub: string; status: "pending" | "running" | "done" | "skipped";
  startedAt?: number; finishedAt?: number; result?: string;
}
export interface LogLine { ts: string; tag: string; text: string; color?: string }

export interface SimulationRun {
  id: string;
  startedAt: number;
  durationMs: number;
  chainId: string;
  windowSummary: string;
  enabledStrategies: StrategyId[];
  opportunities: number;
  netProfit: number;
  summary: SummaryMetrics;
  byStrategy: Record<string, StrategyMetrics>;
  byDex: DexMetrics[];
  liquidationAnalytics: LiquidationAnalytics | null;
  longtailAnalytics: LongtailAnalytics | null;
  opps: Opportunity[];
  config: SimulationState["config"];
  autoParams: SimulationState["autoParams"];
}

export interface SimulationState {
  simulationMode: "mock" | "api";
  selectedChain: ChainConfig;
  config: {
    windowMode: WindowMode;
    lastDays: number;
    fromBlock: number | null;
    toBlock: number | null;
    singleBlock: string | null;
    rpc: string;
    strategies: StrategyConfigs;
    dexes: string[];
    flashLoanProvider: "balancer" | "aave";
  };
  autoParams: {
    gasPrice: number;
    builderTipFormula: string;
    minProfitFormula: string;
    nativeTokenUSD: number;
  };
  pipeline: {
    status: "idle" | "running" | "done" | "error";
    currentStage: number;
    stages: PipelineStage[];
    logs: LogLine[];
    progress: number;
    elapsed: number;
    eta: number;
    startedAt?: number;
    runId?: string;
  };
  results: {
    opportunities: Opportunity[];
    summary: SummaryMetrics | null;
    byStrategy: Record<string, StrategyMetrics>;
    byDex: DexMetrics[];
    liquidationAnalytics: LiquidationAnalytics | null;
    longtailAnalytics: LongtailAnalytics | null;
  };
  reportMetric: "roi" | "raw";
  history: SimulationRun[];
  setSimulationMode: (mode: "mock" | "api") => void;
  setChain: (id: string) => void;
  setWindowMode: (m: WindowMode) => void;
  setLastDays: (n: number) => void;
  setRpc: (rpc: string) => void;
  toggleStrategy: (id: StrategyId, on?: boolean) => void;
  setStrategyParam: <K extends StrategyId>(id: K, patch: Partial<StrategyConfigs[K]>) => void;
  toggleDex: (id: string) => void;
  setFlashProvider: (p: "balancer" | "aave") => void;
  setReportMetric: (m: "roi" | "raw") => void;
  startSimulation: () => Promise<void>;
  cancelSimulation: () => void;
  resolveAutoParams: (c: ChainConfig) => void;
  removeRun: (id: string) => Promise<void>;
  loadHistory: () => Promise<void>;
  getExportJsonUrl: (runId: string) => string;
  getExportCsvUrl: (runId: string) => string;
}

const defaultStrategyConfigs = (chain: ChainConfig): StrategyConfigs => ({
  arb: { enabled: true, minSpread: 0.3, maxHops: 2, tokenWhitelist: ["WETH", "USDC", "USDT", "DAI", "WBTC", chain.nativeToken] },
  jit: { enabled: true, minTVL: 500_000, tickWidth: 20, targetPools: chain.dexes.filter((d) => d.fork === "UniV3").map((d) => d.id) },
  jitarb: { enabled: true, flashProvider: "balancer", maxLoanSize: 50, minSpreadAfterFees: 0.5 },
  sandwich: { enabled: false, frontRunSize: 1.0, maxVictimSlippage: 0.5, feeTier: "0.3", gasMultiplier: 1.5, minNetProfit: 0.01 },
  liquidation: { enabled: false, protocol: chain.lendingProtocols[0]?.id || "aave-v3", minHealthFactor: 1.0, minCollateralUSD: 500, maxRepayAmount: 5, collateralPreference: chain.lendingProtocols[0]?.supportedAssets.slice(0, 2) || [], useFlashLoan: true, flashProvider: "balancer" },
  longtail: { enabled: false, routeDepth: 3, minLiquidity: 10_000, maxPriceImpact: 2.0, crossChain: false, bridgeProtocols: [], bridgeFee: 0.1, tokenUniverse: "midcap", minProfitAfterBridge: 0.005 },
  aggregator: { enabled: false, aggregators: ["1inch", "Odos", "ParaSwap", "0x", "KyberSwap"], minSpread: 0.15, probeSizeUSD: 25_000, maxSplits: 2, includeRfq: true, gasMultiplier: 1.1 },
});

const resolveAuto = (chain: ChainConfig) => ({
  gasPrice: chain.gasPriceGwei,
  builderTipFormula: "10% of expected profit per opportunity",
  minProfitFormula: "2 × estimated gas cost per MEV tx",
  nativeTokenUSD: chain.nativeUSD,
});

const initialChain = CHAINS[0];
let abortToken = { cancelled: false };
let sseCleanup: (() => void) | null = null;

function buildWindowConfig(config: SimulationState["config"]): SimulateRequest["window"] {
  if (config.windowMode === "days") return { mode: "days", last_days: config.lastDays };
  if (config.windowMode === "range" && config.fromBlock && config.toBlock) return { mode: "range", from_block: config.fromBlock, to_block: config.toBlock };
  if (config.windowMode === "block" && config.singleBlock) return { mode: "single", single_block: parseInt(config.singleBlock, 10) };
  return { mode: "blocks", last_days: 100 };
}

async function startApiSimulation(ctx: {
  config: SimulationState["config"]; selectedChain: ChainConfig; autoParams: SimulationState["autoParams"];
  enabled: StrategyId[]; selectedDexes: { id: string; name: string; fork: string }[];
  startedAt: number; log: (tag: string, text: string, color?: string) => void;
}) {
  const { config, selectedChain, enabled, startedAt, log } = ctx;

  const apiStages = ["rpc_fetch", "tx_filter", "revm_replay", "opportunity_scan", "profitability", "aggregation"];
  const stages: PipelineStage[] = apiStages.map((id) => ({
    id,
    label: API_STAGE_LABELS[id] ?? id.toUpperCase(),
    sub: API_STAGE_SUB[id] ?? "",
    status: "pending" as const,
  }));

  useSimulationStore.setState((s) => ({ pipeline: { ...s.pipeline, status: "running", currentStage: 0, stages, logs: [], progress: 0, elapsed: 0, eta: 0, startedAt, runId: undefined } }));

  try {
    const body: SimulateRequest = {
      chain: selectedChain.id,
      rpc_url: config.rpc || undefined,
      strategies: enabled.filter((s) => ["arb", "jit", "jitarb", "sandwich"].includes(s)),
      window: buildWindowConfig(config),
      flash_loan_provider: config.flashLoanProvider,
    };

    const simRes = await apiSimulate(body);
    const runId = simRes.run_id;

    useSimulationStore.setState((s) => ({ pipeline: { ...s.pipeline, runId } }));
    log("API", `Simulation started: ${runId}`);

    await new Promise<void>((resolve, reject) => {
      sseCleanup = subscribeToStatus(runId, {
        onStageStart: (data) => {
          const now = Date.now();
          useSimulationStore.setState((s) => ({
            pipeline: {
              ...s.pipeline,
              currentStage: data.stage,
              stages: s.pipeline.stages.map((st) =>
                st.id === data.id ? { ...st, status: "running" as const, startedAt: now, sub: data.sub } : st
              ),
            },
          }));
          log(data.id.toUpperCase(), `${data.label} starting…`);
        },
        onStageEnd: (data) => {
          useSimulationStore.setState((s) => ({
            pipeline: {
              ...s.pipeline,
              stages: s.pipeline.stages.map((st) =>
                st.id === data.id ? { ...st, status: "done" as const, finishedAt: Date.now(), result: data.result } : st
              ),
            },
          }));
          log(data.id.toUpperCase(), `${API_STAGE_LABELS[data.id] ?? data.id} done: ${data.result}`, "ok");
        },
        onProgress: (data) => {
          useSimulationStore.setState((s) => ({
            pipeline: {
              ...s.pipeline,
              progress: data.total_blocks > 0 ? (data.blocks_processed / data.total_blocks) * 100 : 0,
              elapsed: Date.now() - startedAt,
            },
          }));
        },
        onLog: (data) => {
          log(data.tag, data.text);
        },
        onComplete: async (data) => {
          log("DONE", `Pipeline complete (${data.duration_ms}ms)`, "ok");

          try {
            const results = await getCompletedResults(runId);
            if (!results) {
              log("ERROR", "Results not available after completion", "error");
              useSimulationStore.setState((s) => ({ pipeline: { ...s.pipeline, status: "error" } }));
              reject(new Error("Results not available"));
              return;
            }

            const adapted = adaptCompletedResults(results, selectedChain.id);
            const durationMs = results.duration_ms ?? (Date.now() - startedAt);

            const run: SimulationRun = {
              id: runId,
              startedAt,
              durationMs,
              chainId: selectedChain.id,
              windowSummary: config.windowMode === "days" ? `Last ${config.lastDays} days` : config.windowMode === "range" ? `Blocks ${config.fromBlock ?? "—"}–${config.toBlock ?? "—"}` : `Block ${config.singleBlock ?? "—"}`,
              enabledStrategies: enabled,
              opportunities: adapted.opportunities.length,
              netProfit: adapted.summary?.netProfit ?? 0,
              summary: adapted.summary!,
              byStrategy: adapted.byStrategy,
              byDex: adapted.byDex,
              liquidationAnalytics: null,
              longtailAnalytics: null,
              opps: adapted.opportunities,
              config: useSimulationStore.getState().config,
              autoParams: useSimulationStore.getState().autoParams,
            };

            useSimulationStore.setState((s) => ({
              pipeline: { ...s.pipeline, status: "done", progress: 100, elapsed: Date.now() - startedAt, eta: 0 },
              results: {
                opportunities: adapted.opportunities,
                summary: adapted.summary,
                byStrategy: adapted.byStrategy,
                byDex: adapted.byDex,
                liquidationAnalytics: null,
                longtailAnalytics: null,
              },
              history: [run, ...s.history].slice(0, 30),
            }));
            resolve();
          } catch (err) {
            const msg = err instanceof Error ? err.message : "Failed to fetch results";
            log("ERROR", msg, "error");
            useSimulationStore.setState((s) => ({ pipeline: { ...s.pipeline, status: "error" } }));
            reject(err);
          }
        },
        onError: (data) => {
          log("ERROR", `${data.error}`, "error");
          useSimulationStore.setState((s) => ({
            pipeline: { ...s.pipeline, status: "error" },
          }));
          reject(new Error(data.error));
        },
      });
    });
  } catch (err) {
    if (abortToken.cancelled) return;
    const msg = err instanceof Error ? err.message : "Simulation failed";
    log("ERROR", msg, "error");
    useSimulationStore.setState((s) => ({ pipeline: { ...s.pipeline, status: "error" } }));
  }
}

async function startMockSimulation(ctx: {
  config: SimulationState["config"]; selectedChain: ChainConfig; autoParams: SimulationState["autoParams"];
  enabled: StrategyId[]; usesFL: boolean; usesLiq: boolean; selectedDexes: { id: string; name: string; fork: string }[];
  startedAt: number; log: (tag: string, text: string, color?: string) => void;
}) {
  const { config, selectedChain, autoParams, enabled, usesFL, usesLiq, selectedDexes, startedAt, log } = ctx;

  const stages: PipelineStage[] = [
    { id: "rpc", label: "RPC FETCH", sub: `Fetching ${config.lastDays}-day block window from ${selectedChain.name} RPC`, status: "pending" },
    { id: "filter", label: "TX FILTER", sub: `Filtering DEX swaps${usesLiq ? " + lending events" : ""} across ${selectedDexes.length} DEXes${usesLiq ? ` and ${selectedChain.lendingProtocols.length} protocols` : ""}`, status: "pending" },
    { id: "replay", label: "REVM REPLAY", sub: "Replaying transactions (REVM · 8 threads)", status: "pending" },
    { id: "scan", label: "OPPORTUNITY SCAN", sub: `Scanning: ${enabled.join(", ").toUpperCase()}`, status: "pending" },
    { id: "flash", label: "FLASH LOAN SIM", sub: `Simulating flash-loan contracts (${config.flashLoanProvider})`, status: usesFL ? "pending" : "skipped" },
    { id: "liq", label: "LIQUIDATION SIM", sub: usesLiq ? `Simulating seize/repay on ${config.strategies.liquidation.protocol}` : "not applicable", status: usesLiq ? "pending" : "skipped" },
    { id: "profit", label: "PROFITABILITY CHECK", sub: "Checking net profit ≥ auto threshold per strategy", status: "pending" },
    { id: "agg", label: "AGGREGATION", sub: `Computing P&L across ${enabled.length} strategies · building report`, status: "pending" },
  ];

  useSimulationStore.setState((s) => ({ pipeline: { ...s.pipeline, status: "running", currentStage: 0, stages, logs: [], progress: 0, elapsed: 0, eta: 9000, startedAt, runId: undefined } }));

  const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));
  const stageTimes = [900, 1100, 1600, 2200, 900, 1100, 800, 700];
  const totalActive = stages.reduce((acc, s, i) => acc + (s.status === "skipped" ? 0 : stageTimes[i]), 0);
  let elapsed = 0;

  for (let i = 0; i < stages.length; i++) {
    if (abortToken.cancelled) return;
    const stage = stages[i];
    if (stage.status === "skipped") {
      useSimulationStore.setState((s) => ({ pipeline: { ...s.pipeline, currentStage: i, stages: s.pipeline.stages.map((x, idx) => idx === i ? { ...x, status: "skipped" } : x) } }));
      log("SKIP", `${stage.label} — not applicable`, "muted");
      continue;
    }
    useSimulationStore.setState((s) => ({ pipeline: { ...s.pipeline, currentStage: i, stages: s.pipeline.stages.map((x, idx) => idx === i ? { ...x, status: "running", startedAt: Date.now() } : x) } }));
    log(stage.id.toUpperCase(), `${stage.label} starting…`);
    const dur = stageTimes[i];
    const ticks = 3;
    for (let t = 0; t < ticks; t++) {
      if (abortToken.cancelled) return;
      await sleep(dur / ticks);
      elapsed += dur / ticks;
      const progress = Math.min(100, (elapsed / totalActive) * 100);
      const eta = Math.max(0, totalActive - elapsed);
      if (stage.id === "scan") {
        const strat = enabled[t % enabled.length];
        if (strat === "sandwich") log("SCAN", `Found sandwich candidate: victim 0x${Math.random().toString(16).slice(2, 10)}… slippage ${(Math.random() * 1.5 + 0.3).toFixed(2)}%`);
        else if (strat === "liquidation") log("LIQ", `Position 0x${Math.random().toString(16).slice(2, 8)}… HF=${(0.85 + Math.random() * 0.1).toFixed(2)} · profit +$${(40 + Math.random() * 80).toFixed(0)}`);
        else if (strat === "longtail") log("LONGTAIL", `Route WETH→${["RARE","PEPE","MOG"][t % 3]}→USDC profitable: +${(Math.random() * 0.005).toFixed(5)} ETH`);
        else if (strat === "aggregator") log("AGG", `Quote spread ${(Math.random()*0.5+0.1).toFixed(2)}% · ${["1inch","Odos","ParaSwap","0x","KyberSwap"][t % 5]} vs pool · WETH/USDC`);
        else log("SCAN", `${strat.toUpperCase()} candidate at block #${(19800000 + Math.floor(Math.random()*1000)).toLocaleString()}`);
      }
      useSimulationStore.setState((s) => ({ pipeline: { ...s.pipeline, progress, elapsed: Date.now() - startedAt, eta } }));
    }
    useSimulationStore.setState((s) => ({ pipeline: { ...s.pipeline, stages: s.pipeline.stages.map((x, idx) => idx === i ? { ...x, status: "done", finishedAt: Date.now() } : x) } }));
    log(stage.id.toUpperCase(), `${stage.label} done.`, "ok");
  }

  const minProfit = (autoParams.gasPrice * 1e-9) * 2 * 250000;
  const opps = generateOpportunities({
    lastDays: config.lastDays,
    chain: selectedChain,
    enabledStrategies: enabled,
    dexes: selectedDexes,
    lendingProtocols: selectedChain.lendingProtocols,
    minProfit,
  });
  const agg = aggregate(opps, selectedChain, selectedDexes);
  const run: SimulationRun = {
    id: `run-${Date.now()}`,
    startedAt,
    durationMs: Date.now() - startedAt,
    chainId: selectedChain.id,
    windowSummary: config.windowMode === "days" ? `Last ${config.lastDays} days` : config.windowMode === "range" ? `Blocks ${config.fromBlock ?? "—"}–${config.toBlock ?? "—"}` : `Block ${config.singleBlock ?? "—"}`,
    enabledStrategies: enabled,
    opportunities: opps.length,
    netProfit: agg.summary.netProfit,
    summary: agg.summary,
    byStrategy: agg.byStrategy,
    byDex: agg.byDex,
    liquidationAnalytics: agg.liquidationAnalytics,
    longtailAnalytics: agg.longtailAnalytics,
    opps,
    config: useSimulationStore.getState().config,
    autoParams: useSimulationStore.getState().autoParams,
  };
  useSimulationStore.setState((s) => ({
    pipeline: { ...s.pipeline, status: "done", progress: 100, elapsed: Date.now() - startedAt, eta: 0 },
    results: { opportunities: opps, summary: agg.summary, byStrategy: agg.byStrategy, byDex: agg.byDex, liquidationAnalytics: agg.liquidationAnalytics, longtailAnalytics: agg.longtailAnalytics },
    history: [run, ...s.history].slice(0, 30),
  }));
  log("DONE", `Simulation complete · ${opps.length} opportunities · net profit ${agg.summary.netProfit.toFixed(4)} ${selectedChain.nativeToken}`, "ok");
}

export const useSimulationStore = create<SimulationState>((set, get) => ({
  simulationMode: (import.meta.env.VITE_SIMULATION_MODE as "mock" | "api") ?? "mock",
  selectedChain: initialChain,
  config: {
    windowMode: "days",
    lastDays: 30,
    fromBlock: null,
    toBlock: null,
    singleBlock: null,
    rpc: initialChain.rpcDefault,
    strategies: defaultStrategyConfigs(initialChain),
    dexes: initialChain.dexes.slice(0, 4).map((d) => d.id),
    flashLoanProvider: "balancer",
  },
  autoParams: resolveAuto(initialChain),
  pipeline: { status: "idle", currentStage: 0, stages: [], logs: [], progress: 0, elapsed: 0, eta: 0 },
  results: { opportunities: [], summary: null, byStrategy: {}, byDex: [], liquidationAnalytics: null, longtailAnalytics: null },
  reportMetric: "raw",
  history: [],

  setSimulationMode: (mode) => set({ simulationMode: mode }),
  setChain: (id) => {
    const chain = getChain(id);
    set({
      selectedChain: chain,
      config: {
        ...get().config,
        rpc: chain.rpcDefault,
        strategies: defaultStrategyConfigs(chain),
        dexes: chain.dexes.slice(0, 4).map((d) => d.id),
      },
      autoParams: resolveAuto(chain),
    });
  },
  setWindowMode: (m) => set({ config: { ...get().config, windowMode: m } }),
  setLastDays: (n) => set({ config: { ...get().config, lastDays: Math.max(1, Math.min(90, n)) } }),
  setRpc: (rpc) => set({ config: { ...get().config, rpc } }),
  toggleStrategy: (id, on) =>
    set((s) => ({
      config: {
        ...s.config,
        strategies: {
          ...s.config.strategies,
          [id]: { ...s.config.strategies[id], enabled: on ?? !s.config.strategies[id].enabled },
        },
      },
    })),
  setStrategyParam: (id, patch) =>
    set((s) => ({
      config: {
        ...s.config,
        strategies: { ...s.config.strategies, [id]: { ...s.config.strategies[id], ...patch } },
      },
    })),
  toggleDex: (id) =>
    set((s) => {
      const has = s.config.dexes.includes(id);
      return { config: { ...s.config, dexes: has ? s.config.dexes.filter((d) => d !== id) : [...s.config.dexes, id] } };
    }),
  setFlashProvider: (p) => set({ config: { ...get().config, flashLoanProvider: p } }),
  setReportMetric: (m) => set({ reportMetric: m }),
  resolveAutoParams: (c) => set({ autoParams: resolveAuto(c) }),
  cancelSimulation: () => {
    abortToken.cancelled = true;
    if (sseCleanup) { sseCleanup(); sseCleanup = null; }
    set((s) => ({ pipeline: { ...s.pipeline, status: "idle" } }));
  },
  removeRun: async (id) => {
    const { simulationMode } = get();
    if (simulationMode === "api") {
      try { await deleteHistoryRun(id); } catch { /* ignore */ }
    }
    set((s) => ({ history: s.history.filter((r) => r.id !== id) }));
  },
  loadHistory: async () => {
    const { simulationMode } = get();
    if (simulationMode !== "api") return;
    try {
      const entries = await getHistory();
      const history = entries.map(adaptHistoryEntry);
      set({ history });
    } catch { /* API not available */ }
  },
  getExportJsonUrl: (runId: string) => getExportJsonUrl(runId),
  getExportCsvUrl: (runId: string) => getExportCsvUrl(runId),

  startSimulation: async () => {
    abortToken = { cancelled: false };
    const { config, selectedChain, autoParams, simulationMode } = get();
    const enabled = (Object.keys(config.strategies) as StrategyId[]).filter((s) => config.strategies[s].enabled);
    const usesFL = enabled.includes("jitarb") || (enabled.includes("liquidation") && config.strategies.liquidation.useFlashLoan);
    const usesLiq = enabled.includes("liquidation");
    const selectedDexes = selectedChain.dexes.filter((d) => config.dexes.includes(d.id));
    const startedAt = Date.now();

    const log = (tag: string, text: string, color?: string) => {
      const ts = new Date().toISOString().slice(11, 23);
      set((s) => ({ pipeline: { ...s.pipeline, logs: [...s.pipeline.logs, { ts, tag, text, color }].slice(-200) } }));
    };

    if (simulationMode === "api") {
      await startApiSimulation({ config, selectedChain, autoParams, enabled, selectedDexes, startedAt, log });
    } else {
      await startMockSimulation({ config, selectedChain, autoParams, enabled, usesFL, usesLiq, selectedDexes, startedAt, log });
    }
  },
}));
