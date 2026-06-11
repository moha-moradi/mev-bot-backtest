import { useSimulationStore } from "@/store/simulationStore";
import { Field, NumInput } from "../Field";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";

export function JitArbParams() {
  const cfg = useSimulationStore((s) => s.config.strategies.jitarb);
  const setParam = useSimulationStore((s) => s.setStrategyParam);
  return (
    <div className="grid grid-cols-2 gap-4">
      <Field label="Flash loan provider" className="col-span-2">
        <RadioGroup value={cfg.flashProvider} onValueChange={(v) => setParam("jitarb", { flashProvider: v as "balancer"|"aave" })} className="flex gap-3">
          {[["balancer","Balancer v2"],["aave","Aave v3"]].map(([v,l]) => (
            <label key={v} className="flex items-center gap-2 rounded-md border border-[var(--line)] bg-[var(--surface-2)] px-3 py-1.5 cursor-pointer">
              <RadioGroupItem value={v} className="border-[var(--ink-mute)]" />
              <span className="font-mono text-xs">{l}</span>
            </label>
          ))}
        </RadioGroup>
      </Field>
      <Field label="Max loan size">
        <NumInput value={cfg.maxLoanSize} onChange={(e) => setParam("jitarb", { maxLoanSize: parseFloat(e.target.value) })} />
      </Field>
      <Field label="Min spread after fees %">
        <NumInput value={cfg.minSpreadAfterFees} step="0.1" onChange={(e) => setParam("jitarb", { minSpreadAfterFees: parseFloat(e.target.value) })} />
      </Field>
    </div>
  );
}