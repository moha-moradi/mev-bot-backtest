import { useSimulationStore } from "@/store/simulationStore";
import { Field, NumInput, Segmented } from "../Field";
import { PnlFormulaCard } from "../PnlFormulaCard";

export function SandwichParams() {
  const cfg = useSimulationStore((s) => s.config.strategies.sandwich);
  const chain = useSimulationStore((s) => s.selectedChain);
  const setParam = useSimulationStore((s) => s.setStrategyParam);
  const gross = cfg.frontRunSize * (cfg.maxVictimSlippage / 100);
  const gas = 0.00021 * cfg.gasMultiplier * chain.gasPriceGwei;
  const fee = parseFloat(cfg.feeTier) / 100 * cfg.frontRunSize * 2;
  const net = gross - gas - fee;
  return (
    <div className="grid grid-cols-2 gap-4">
      <Field label="Front-run size" sub="Capital deployed ahead of victim tx">
        <NumInput value={cfg.frontRunSize} step="0.1" onChange={(e) => setParam("sandwich", { frontRunSize: parseFloat(e.target.value) })} />
      </Field>
      <Field label="Max victim slippage %" sub="Only target txns with slippage ≥ this value">
        <NumInput value={cfg.maxVictimSlippage} step="0.1" onChange={(e) => setParam("sandwich", { maxVictimSlippage: parseFloat(e.target.value) })} />
      </Field>
      <Field label="Target DEX fee tier" sub="Uniswap v3 fee tier of victim pool">
        <Segmented options={[{value:"0.05",label:"0.05%"},{value:"0.3",label:"0.3%"},{value:"1",label:"1%"}]} value={cfg.feeTier} onChange={(v) => setParam("sandwich", { feeTier: v as "0.05"|"0.3"|"1" })} />
      </Field>
      <Field label="Sandwich gas multiplier" sub="Gas premium to guarantee front-run ordering">
        <NumInput value={cfg.gasMultiplier} step="0.1" onChange={(e) => setParam("sandwich", { gasMultiplier: parseFloat(e.target.value) })} />
      </Field>
      <Field label={`Min net profit (${chain.nativeToken})`}>
        <NumInput value={cfg.minNetProfit} step="0.01" onChange={(e) => setParam("sandwich", { minNetProfit: parseFloat(e.target.value) })} />
      </Field>
      <PnlFormulaCard
        title="Live P&L model"
        lines={[
          { label: "Gross capture", value: `${gross.toFixed(5)} ${chain.nativeToken}` },
          { label: "Gas (× mult)", value: `−${gas.toFixed(5)}` },
          { label: "DEX fees (×2)", value: `−${fee.toFixed(5)}` },
        ]}
        net={`${net.toFixed(5)} ${chain.nativeToken}`}
      />
    </div>
  );
}