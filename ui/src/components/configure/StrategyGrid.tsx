import { useState } from "react";
import { STRATEGY_DEFS } from "@/lib/strategyDefs";
import { StrategyCard } from "./StrategyCard";
import { ArbParams } from "./params/ArbParams";
import { JitParams } from "./params/JitParams";
import { JitArbParams } from "./params/JitArbParams";
import { SandwichParams } from "./params/SandwichParams";
import { LongtailParams } from "./params/LongtailParams";
import { AggregatorParams } from "./params/AggregatorParams";
import { TraceModal } from "./TraceModal";
import { SectionCard } from "../shared/SectionCard";
import { useSimulationStore } from "@/store/simulationStore";
import type { StrategyDef } from "@/lib/strategyDefs";

const API_STRATEGIES = new Set(["arb", "jit", "jitarb", "sandwich"]);
const PARAMS: Record<string, () => React.JSX.Element> = {
  arb: ArbParams, jit: JitParams, jitarb: JitArbParams,
  sandwich: SandwichParams, longtail: LongtailParams,
  aggregator: AggregatorParams,
};

export function StrategyGrid() {
  const [trace, setTrace] = useState<StrategyDef | null>(null);
  const simulationMode = useSimulationStore((s) => s.simulationMode);
  return (
    <SectionCard title="02 · Strategies">
      {simulationMode === "api" && (
        <div className="mb-3 rounded-md border border-[var(--acc-amber)]/30 bg-[var(--acc-amber)]/5 px-3 py-2 font-mono text-[10px] text-[var(--acc-amber)]">
          API mode: only arb, jit, jitarb, and sandwich strategies are supported. Other strategies are simulated locally.
        </div>
      )}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
        {STRATEGY_DEFS.map((def) => {
          const Params = PARAMS[def.id];
          return (
            <StrategyCard key={def.id} def={def} onShowTrace={() => setTrace(def)}>
              <Params />
            </StrategyCard>
          );
        })}
      </div>
      <TraceModal def={trace} open={!!trace} onOpenChange={(o) => !o && setTrace(null)} />
    </SectionCard>
  );
}