import { useSimulationStore } from "@/store/simulationStore";
import { Field, NumInput, Chips } from "../Field";

export function JitParams() {
  const cfg = useSimulationStore((s) => s.config.strategies.jit);
  const chain = useSimulationStore((s) => s.selectedChain);
  const setParam = useSimulationStore((s) => s.setStrategyParam);
  const v3Pools = chain.dexes.filter((d) => d.fork === "UniV3" || d.fork === "Algebra").map((d) => d.id);
  return (
    <div className="grid grid-cols-2 gap-4">
      <Field label="Min pool TVL (USD)">
        <NumInput value={cfg.minTVL} step="50000" onChange={(e) => setParam("jit", { minTVL: parseFloat(e.target.value) })} />
      </Field>
      <Field label="Tick range width (bps)">
        <NumInput value={cfg.tickWidth} onChange={(e) => setParam("jit", { tickWidth: parseFloat(e.target.value) })} />
      </Field>
      <Field label="Target pools" className="col-span-2">
        <Chips options={v3Pools} value={cfg.targetPools} onToggle={(p) => setParam("jit", { targetPools: cfg.targetPools.includes(p) ? cfg.targetPools.filter((x) => x !== p) : [...cfg.targetPools, p] })} />
      </Field>
    </div>
  );
}