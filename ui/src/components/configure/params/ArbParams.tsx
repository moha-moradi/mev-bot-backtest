import { useSimulationStore } from "@/store/simulationStore";
import { Field, NumInput, Segmented, Chips } from "../Field";
import { TOKENS } from "@/lib/formatters";

export function ArbParams() {
  const cfg = useSimulationStore((s) => s.config.strategies.arb);
  const setParam = useSimulationStore((s) => s.setStrategyParam);
  return (
    <div className="grid grid-cols-2 gap-4">
      <Field label="Min spread %">
        <NumInput value={cfg.minSpread} step="0.1" onChange={(e) => setParam("arb", { minSpread: parseFloat(e.target.value) })} />
      </Field>
      <Field label="Max hops">
        <Segmented options={[{value:"1",label:"1"},{value:"2",label:"2"},{value:"3",label:"3"},{value:"4",label:"4"}]} value={String(cfg.maxHops)} onChange={(v) => setParam("arb", { maxHops: parseInt(v) as 1|2|3|4 })} />
      </Field>
      <Field label="Token whitelist" className="col-span-2">
        <Chips options={TOKENS.slice(0,6)} value={cfg.tokenWhitelist as string[]} onToggle={(t) => setParam("arb", { tokenWhitelist: cfg.tokenWhitelist.includes(t) ? cfg.tokenWhitelist.filter((x) => x !== t) : [...cfg.tokenWhitelist, t] })} />
      </Field>
    </div>
  );
}