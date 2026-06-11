import { useSimulationStore } from "@/store/simulationStore";
import { Field, NumInput, Segmented, Chips } from "../Field";
import { Switch } from "@/components/ui/switch";
import { PnlFormulaCard } from "../PnlFormulaCard";
import { AGGREGATORS } from "@/lib/formatters";

export function AggregatorParams() {
  const cfg = useSimulationStore((s) => s.config.strategies.aggregator);
  const chain = useSimulationStore((s) => s.selectedChain);
  const setParam = useSimulationStore((s) => s.setStrategyParam);

  const grossUSD = cfg.probeSizeUSD * (cfg.minSpread / 100);
  const gasUSD = chain.gasPriceGwei * 250000 * 1e-9 * chain.nativeUSD * cfg.gasMultiplier;
  const netUSD = grossUSD - gasUSD;
  const netNative = netUSD / Math.max(1, chain.nativeUSD);

  return (
    <div className="grid grid-cols-2 gap-4">
      <Field label="Aggregators to monitor" className="col-span-2" sub="Compare quotes across these aggregators + direct DEX pools">
        <Chips
          options={AGGREGATORS}
          value={cfg.aggregators}
          onToggle={(a) => setParam("aggregator", { aggregators: cfg.aggregators.includes(a) ? cfg.aggregators.filter((x) => x !== a) : [...cfg.aggregators, a] })}
        />
      </Field>
      <Field label="Min spread % vs best quote" sub="Reject opportunities below this aggregator-vs-pool gap">
        <NumInput value={cfg.minSpread} step="0.05" onChange={(e) => setParam("aggregator", { minSpread: parseFloat(e.target.value) })} />
      </Field>
      <Field label="Probe size (USD)" sub="Quote request size sent to each aggregator">
        <NumInput value={cfg.probeSizeUSD} step="1000" onChange={(e) => setParam("aggregator", { probeSizeUSD: parseFloat(e.target.value) })} />
      </Field>
      <Field label="Max route splits" sub="Aggregator splits one trade across N venues">
        <Segmented options={[{value:"1",label:"1"},{value:"2",label:"2"},{value:"3",label:"3"}]} value={String(cfg.maxSplits)} onChange={(v) => setParam("aggregator", { maxSplits: parseInt(v) as 1|2|3 })} />
      </Field>
      <Field label="Gas multiplier" sub="Inflate gas estimate to win inclusion">
        <NumInput value={cfg.gasMultiplier} step="0.1" onChange={(e) => setParam("aggregator", { gasMultiplier: parseFloat(e.target.value) })} />
      </Field>
      <Field label="Include RFQ / private orderflow" sub="CowSwap, Bebop, 1inch Fusion, Hashflow" className="col-span-2">
        <div className="flex items-center gap-2 h-9"><Switch checked={cfg.includeRfq} onCheckedChange={(v) => setParam("aggregator", { includeRfq: v })} /><span className="font-mono text-xs text-[var(--ink-dim)]">{cfg.includeRfq ? "On" : "Off"}</span></div>
      </Field>
      <div className="col-span-2">
        <PnlFormulaCard
          title="Live P&L model"
          lines={[
            { label: `Gross (${cfg.minSpread}% × $${cfg.probeSizeUSD.toLocaleString()})`, value: `$${grossUSD.toFixed(2)}` },
            { label: "Gas (×" + cfg.gasMultiplier + ")", value: `−$${gasUSD.toFixed(2)}` },
          ]}
          net={`${netNative.toFixed(5)} ${chain.nativeToken}  ·  $${netUSD.toFixed(2)}`}
        />
      </div>
    </div>
  );
}