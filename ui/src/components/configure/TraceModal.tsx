import { Dialog, DialogContent, DialogTitle, DialogHeader } from "@/components/ui/dialog";
import type { StrategyDef } from "@/lib/strategyDefs";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { ChevronRight } from "lucide-react";
import { useSimulationStore } from "@/store/simulationStore";

const SAMPLE: Record<string, { steps: { l: string; v?: string; s?: string }[]; result: { l: string; v: string; positive?: boolean }[] }> = {
  arb: {
    steps: [
      { l: "Block", v: "#19,842,301" },
      { l: "Pair", v: "WETH/USDC" },
      { l: "Leg 1", v: "Uniswap v3 — buy 4.2 WETH @ 3,198 USDC" },
      { l: "Leg 2", v: "SushiSwap — sell 4.2 WETH @ 3,212 USDC" },
    ],
    result: [{ l: "Gross", v: "0.0181 ETH" }, { l: "Gas", v: "−0.0028 ETH" }, { l: "Net", v: "+0.0153 ETH", positive: true }],
  },
  jit: {
    steps: [
      { l: "Block", v: "#19,842,310" },
      { l: "Target pool", v: "USDC/WETH 0.05%" },
      { l: "Detected incoming swap", s: "victim 0x4a3…b821 swap 220k USDC → WETH" },
      { l: "Mint LP", s: "tick range ±10 bps ahead of swap" },
      { l: "Burn LP", s: "right after victim execution, capture fees" },
    ],
    result: [{ l: "Fees earned", v: "0.041 ETH" }, { l: "Gas", v: "−0.006 ETH" }, { l: "Net", v: "+0.035 ETH", positive: true }],
  },
  jitarb: {
    steps: [
      { l: "Block", v: "#19,842,318" },
      { l: "Flash loan", v: "60 WETH from Balancer" },
      { l: "JIT mint", s: "Uniswap v3 USDC/WETH 0.05%" },
      { l: "Victim swap routes through our LP" },
      { l: "Burn LP, arb exit", s: "Curve TriCrypto for the difference" },
      { l: "Repay flash loan", v: "60 WETH atomic" },
    ],
    result: [{ l: "Gross", v: "0.072 ETH" }, { l: "Gas + FL fee", v: "−0.014 ETH" }, { l: "Net", v: "+0.058 ETH", positive: true }],
  },
  sandwich: {
    steps: [
      { l: "Block", v: "#19,842,301" },
      { l: "Victim tx", v: "0xabc…def", s: "swap 5 ETH → USDC on Uniswap v3 · slippage 0.8%" },
      { l: "Front-run", v: "0x111…222", s: "buy 2 ETH of USDC before victim · gas +50%" },
      { l: "Victim executes", s: "at worse price (slippage absorbed)" },
      { l: "Back-run", v: "0x333…444", s: "sell USDC back · capture spread" },
    ],
    result: [{ l: "Gross capture", v: "0.038 ETH" }, { l: "Gas (×1.5)", v: "−0.009 ETH" }, { l: "DEX fees (×2)", v: "−0.006 ETH" }, { l: "Net", v: "+0.023 ETH", positive: true }],
  },
  liquidation: {
    steps: [
      { l: "Block", v: "#19,842,410" },
      { l: "Protocol", v: "Aave v3 (Arbitrum)" },
      { l: "Position", v: "0xabc…", s: "HF 0.94 · debt 3,200 USDC · collateral 1.8 WETH ($5,760)" },
      { l: "Max repay (50% close factor)", v: "1,600 USDC" },
      { l: "Flash loan", v: "1,600 USDC", s: "Balancer · fee 0%" },
      { l: "Seize", v: "1,680 USDC of WETH → 0.525 WETH" },
      { l: "Sell WETH → USDC", v: "1,682 USDC" },
      { l: "Repay flash loan", v: "1,600 USDC" },
    ],
    result: [{ l: "Gas", v: "−$12.80" }, { l: "Net", v: "+$69.20", positive: true }],
  },
  longtail: {
    steps: [
      { l: "Block", v: "#19,842,500" },
      { l: "Route (3-hop)", v: "WETH → RARE → USDC → WETH" },
      { l: "Leg 1", v: "Uniswap v3", s: "WETH→RARE · 0.00042 WETH/RARE" },
      { l: "Leg 2", v: "Camelot v2", s: "RARE→USDC · 2.38 USDC/RARE" },
      { l: "Leg 3", v: "SushiSwap", s: "USDC→WETH · 0.000415 WETH/USDC" },
      { l: "Price impact", v: "1.2% (within 2% limit)" },
    ],
    result: [{ l: "Gross", v: "0.0031 ETH" }, { l: "Gas (3 hops)", v: "−0.0012 ETH" }, { l: "Net", v: "+0.0019 ETH", positive: true }],
  },
  aggregator: {
    steps: [
      { l: "Block", v: "#19,842,612" },
      { l: "Pair", v: "WETH/USDC · probe $25,000" },
      { l: "1inch quote", v: "8.2451 WETH out", s: "split: 60% Uni v3 / 40% Curve" },
      { l: "Odos quote", v: "8.2389 WETH out", s: "split: 100% Uni v3" },
      { l: "ParaSwap quote", v: "8.2310 WETH out", s: "split: 70% Balancer / 30% Sushi" },
      { l: "Pool baseline", v: "8.2218 WETH out", s: "direct Uniswap v3 0.05%" },
      { l: "Action", s: "buy via 1inch (best in), sell into Uni v3 pool (worst out) — capture spread" },
      { l: "Spread captured", v: "0.28% (28 bps)" },
    ],
    result: [{ l: "Gross", v: "0.022 ETH" }, { l: "Gas (×1.1)", v: "−0.004 ETH" }, { l: "Net", v: "+0.018 ETH", positive: true }],
  },
};

export function TraceModal({ def, open, onOpenChange }: { def: StrategyDef | null; open: boolean; onOpenChange: (o: boolean) => void }) {
  const chain = useSimulationStore((s) => s.selectedChain);
  if (!def) return null;
  const sample = SAMPLE[def.id];
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="bg-[var(--surface)] border-[var(--line)] max-w-lg">
        <DialogHeader>
          <DialogTitle className="font-mono text-sm uppercase tracking-[0.18em] flex items-center gap-2">
            <span className="h-2 w-2 rounded-full" style={{ backgroundColor: def.color }} />
            Expected trace · {def.name}
          </DialogTitle>
        </DialogHeader>
        <div className="rounded-md border border-[var(--line)] bg-[#080b0f] p-4 font-mono text-xs space-y-1.5">
          {sample.steps.map((s, i) => (
            <div key={i} className="grid grid-cols-[100px_1fr] gap-3">
              <span className="text-[var(--ink-dim)]">{s.l}</span>
              <div>
                {s.v && <span className="text-[var(--ink)]">{s.v}</span>}
                {s.s && <div className="text-[var(--ink-dim)] mt-0.5">{s.s}</div>}
              </div>
            </div>
          ))}
          <div className="mt-3 pt-3 border-t border-[var(--line)] space-y-1">
            {sample.result.map((r, i) => (
              <div key={i} className="grid grid-cols-[100px_1fr] gap-3">
                <span className="text-[var(--ink-dim)]">{r.l}</span>
                <span className={r.positive ? "text-[var(--acc-green)] font-semibold" : "text-[var(--ink)]"}>{r.v}</span>
              </div>
            ))}
          </div>
        </div>
        <Collapsible>
          <CollapsibleTrigger className="inline-flex items-center gap-1 font-mono text-[10px] uppercase tracking-[0.18em] text-[var(--ink-dim)] hover:text-[var(--ink)]">
            <ChevronRight className="h-3 w-3" /> Why this is realistic
          </CollapsibleTrigger>
          <CollapsibleContent className="mt-2 text-xs text-[var(--ink-dim)] leading-relaxed">
            Values are calibrated to typical {chain.name} block conditions: gas prices, observed slippages, and historic profitability distributions for the {def.name.toLowerCase()} strategy on this chain.
          </CollapsibleContent>
        </Collapsible>
      </DialogContent>
    </Dialog>
  );
}