import { ArrowLeftRight, Zap, Layers, Sandwich, Droplets, Route, Combine } from "lucide-react";
import type { StrategyId } from "./chains";

export interface StrategyDef {
  id: StrategyId;
  name: string;
  short: string;
  description: string;
  icon: typeof ArrowLeftRight;
  color: string;
  colorVar: string;
  defaultEnabled: boolean;
  warning?: string;
}

export const STRATEGY_DEFS: StrategyDef[] = [
  {
    id: "arb", name: "Arbitrage", short: "ARB",
    description: "Cross-DEX price difference capture on same chain.",
    icon: ArrowLeftRight, color: "#38bdf8", colorVar: "var(--acc-blue)",
    defaultEnabled: true,
  },
  {
    id: "jit", name: "JIT Liquidity", short: "JIT",
    description: "Just-in-time LP provision around large incoming swaps.",
    icon: Zap, color: "#f59e0b", colorVar: "var(--acc-amber)",
    defaultEnabled: true,
  },
  {
    id: "jitarb", name: "JIT + Arb with Flash Loan", short: "JIT+ARB",
    description: "JIT entry combined with arbitrage exit, funded by flash loan.",
    icon: Layers, color: "#a78bfa", colorVar: "var(--acc-purple)",
    defaultEnabled: true,
  },
  {
    id: "sandwich", name: "Sandwich", short: "SANDWICH",
    description: "Front-run and back-run a victim swap to extract slippage value.",
    icon: Sandwich, color: "#fb923c", colorVar: "var(--acc-orange)",
    defaultEnabled: false,
    warning: "Sandwich attacks are adversarial and harmful to retail users. This strategy is included for research and defense analysis only.",
  },
  {
    id: "longtail", name: "Long-tail Arbitrage", short: "LONG-TAIL",
    description: "Detect price dislocations in low-liquidity token pairs across DEXes and chains.",
    icon: Route, color: "#22d3ee", colorVar: "var(--acc-cyan)",
    defaultEnabled: false,
  },
  {
    id: "aggregator", name: "Aggregator Arbitrage", short: "AGG",
    description: "Arb DEX aggregator quotes (1inch, Odos, ParaSwap, 0x, KyberSwap, OpenOcean) against direct pool prices and across aggregators.",
    icon: Combine, color: "#2dd4bf", colorVar: "var(--acc-teal)",
    defaultEnabled: false,
  },
];

export const STRATEGY_BY_ID = Object.fromEntries(STRATEGY_DEFS.map((s) => [s.id, s])) as Record<StrategyId, StrategyDef>;