import { useSimulationStore } from "@/store/simulationStore";
import { Field, NumInput, Chips } from "../Field";
import { PnlFormulaCard } from "../PnlFormulaCard";
import { Switch } from "@/components/ui/switch";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";

export function LiquidationParams() {
  const cfg = useSimulationStore((s) => s.config.strategies.liquidation);
  const chain = useSimulationStore((s) => s.selectedChain);
  const setParam = useSimulationStore((s) => s.setStrategyParam);
  const proto = chain.lendingProtocols.find((p) => p.id === cfg.protocol) ?? chain.lendingProtocols[0];
  const seize = cfg.maxRepayAmount * (1 + (proto?.liquidationBonus ?? 0.05));
  const flashFee = cfg.useFlashLoan ? cfg.maxRepayAmount * 0.0009 : 0;
  const gas = 0.00021 * chain.gasPriceGwei;
  const net = seize - cfg.maxRepayAmount - flashFee - gas;
  return (
    <div className="grid grid-cols-2 gap-4">
      <Field label="Lending protocol" className="col-span-2">
        <Select value={cfg.protocol} onValueChange={(v) => setParam("liquidation", { protocol: v })}>
          <SelectTrigger className="bg-[var(--surface-2)] border-[var(--line)] font-mono text-xs h-9">
            <SelectValue />
          </SelectTrigger>
          <SelectContent className="bg-[var(--surface)] border-[var(--line)]">
            {chain.lendingProtocols.map((p) => (
              <SelectItem key={p.id} value={p.id} className="font-mono text-xs">{p.name} {p.version}</SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>
      <Field label="Min health factor threshold" sub="Only target positions with HF ≤ this value">
        <NumInput value={cfg.minHealthFactor} step="0.05" onChange={(e) => setParam("liquidation", { minHealthFactor: parseFloat(e.target.value) })} />
      </Field>
      <Field label="Min collateral value (USD)" sub="Skip dust positions below this size">
        <NumInput value={cfg.minCollateralUSD} step="100" onChange={(e) => setParam("liquidation", { minCollateralUSD: parseFloat(e.target.value) })} />
      </Field>
      <Field label={`Max repay amount (${chain.nativeToken})`} sub="Close factor applied automatically">
        <NumInput value={cfg.maxRepayAmount} step="0.5" onChange={(e) => setParam("liquidation", { maxRepayAmount: parseFloat(e.target.value) })} />
      </Field>
      <Field label="Collateral preference" sub="Prefer to seize these assets">
        <Chips options={proto?.supportedAssets ?? []} value={cfg.collateralPreference} onToggle={(a) => setParam("liquidation", { collateralPreference: cfg.collateralPreference.includes(a) ? cfg.collateralPreference.filter((x) => x !== a) : [...cfg.collateralPreference, a] })} />
      </Field>
      <Field label="Use flash loan for repay" sub="Avoid capital lock-up">
        <div className="flex items-center gap-2 h-9"><Switch checked={cfg.useFlashLoan} onCheckedChange={(v) => setParam("liquidation", { useFlashLoan: v })} /><span className="font-mono text-xs text-[var(--ink-dim)]">{cfg.useFlashLoan ? "Enabled" : "Disabled"}</span></div>
      </Field>
      {cfg.useFlashLoan && (
        <Field label="Flash loan provider">
          <RadioGroup value={cfg.flashProvider} onValueChange={(v) => setParam("liquidation", { flashProvider: v as "balancer"|"aave" })} className="flex gap-2">
            {[["balancer","Balancer"],["aave","Aave"]].map(([v,l]) => (
              <label key={v} className="flex items-center gap-2 rounded-md border border-[var(--line)] bg-[var(--surface-2)] px-2 py-1 cursor-pointer">
                <RadioGroupItem value={v} /><span className="font-mono text-xs">{l}</span>
              </label>
            ))}
          </RadioGroup>
        </Field>
      )}
      <div className="col-span-2">
        <PnlFormulaCard
          title="Live P&L model"
          lines={[
            { label: `Seize (×${(1 + (proto?.liquidationBonus ?? 0.05)).toFixed(3)})`, value: `${seize.toFixed(2)} USDC` },
            { label: "Repay debt", value: `−${cfg.maxRepayAmount.toFixed(2)} USDC` },
            { label: "Flash fee", value: `−${flashFee.toFixed(4)}` },
            { label: "Gas", value: `−${gas.toFixed(5)} ${chain.nativeToken}` },
          ]}
          net={`${net.toFixed(2)} USDC equiv`}
        />
      </div>
    </div>
  );
}