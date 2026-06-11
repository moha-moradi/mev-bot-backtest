import { useSimulationStore } from "@/store/simulationStore";
import { SectionCard } from "../shared/SectionCard";
import { RefreshCw, Info } from "lucide-react";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { formatUSD } from "@/lib/formatters";

export function AutoEconomics() {
  const chain = useSimulationStore((s) => s.selectedChain);
  const ap = useSimulationStore((s) => s.autoParams);
  const resolve = useSimulationStore((s) => s.resolveAutoParams);
  const simulationMode = useSimulationStore((s) => s.simulationMode);
  const rows = [
    { label: "Gas price", value: `${ap.gasPrice} gwei`, formula: simulationMode === "api" ? `From API chain config (${chain.name}: ${ap.gasPrice} gwei)` : `Mock per chain (${chain.name}: ${ap.gasPrice} gwei)` },
    { label: "Builder tip", value: ap.builderTipFormula, formula: "10% of expected profit per opportunity" },
    { label: `Min profit threshold (${chain.nativeToken})`, value: ap.minProfitFormula, formula: "2 × estimated gas cost per MEV tx" },
    { label: "Native token price", value: formatUSD(ap.nativeTokenUSD), formula: simulationMode === "api" ? `From API chain config (coingeckoId: ${chain.coingeckoId})` : `Mocked USD from coingeckoId: ${chain.coingeckoId}` },
  ];
  return (
    <SectionCard title="04 · Economics" subtitle="Auto-resolved · no manual overrides here" action={
      <button onClick={() => resolve(chain)} className="inline-flex items-center gap-1 rounded-md border border-[var(--line)] px-2 py-1 font-mono text-[10px] uppercase tracking-[0.14em] text-[var(--ink-dim)] hover:text-[var(--ink)]">
        <RefreshCw className="h-3 w-3" /> Refresh
      </button>
    }>
      <TooltipProvider delayDuration={100}>
        <div className="divide-y divide-[var(--line)]">
          {rows.map((r) => (
            <div key={r.label} className="flex items-center justify-between py-2.5 first:pt-0 last:pb-0">
              <div className="flex items-center gap-2">
                <span className="font-mono text-xs text-[var(--ink-dim)]">{r.label}</span>
                <Tooltip>
                  <TooltipTrigger><Info className="h-3 w-3 text-[var(--ink-mute)]" /></TooltipTrigger>
                  <TooltipContent className="bg-[var(--surface)] border-[var(--line)] font-mono text-xs max-w-xs">{r.formula}</TooltipContent>
                </Tooltip>
              </div>
              <div className="flex items-center gap-2">
                <span className="font-mono text-xs text-[var(--ink)]">{r.value}</span>
                <span className="rounded-sm bg-[var(--acc-green)]/10 px-1.5 py-0.5 font-mono text-[9px] uppercase tracking-[0.14em] text-[var(--acc-green)]">Auto</span>
              </div>
            </div>
          ))}
        </div>
      </TooltipProvider>
    </SectionCard>
  );
}