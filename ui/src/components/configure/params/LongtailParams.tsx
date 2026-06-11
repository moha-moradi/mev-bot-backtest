import { useSimulationStore } from "@/store/simulationStore";
import { Field, NumInput, Segmented, Chips } from "../Field";
import { PnlFormulaCard } from "../PnlFormulaCard";
import { Switch } from "@/components/ui/switch";
import { BRIDGE_PROTOCOLS } from "@/lib/formatters";

export function LongtailParams() {
  const cfg = useSimulationStore((s) => s.config.strategies.longtail);
  const chain = useSimulationStore((s) => s.selectedChain);
  const setParam = useSimulationStore((s) => s.setStrategyParam);
  const profit = 0.003 * (cfg.routeDepth / 2);
  const impact = profit * (cfg.maxPriceImpact / 100);
  const bridge = cfg.crossChain ? profit * (cfg.bridgeFee / 100) : 0;
  const gas = cfg.routeDepth * 0.0005 * chain.gasPriceGwei;
  const net = profit - impact - bridge - gas;
  return (
    <div className="grid grid-cols-2 gap-4">
      <Field label="Route depth" sub="Max intermediate tokens in arbitrage route">
        <Segmented options={[{value:"2",label:"2-hop"},{value:"3",label:"3-hop"},{value:"4",label:"4-hop"}]} value={String(cfg.routeDepth)} onChange={(v) => setParam("longtail", { routeDepth: parseInt(v) as 2|3|4 })} />
      </Field>
      <Field label="Min liquidity per pool (USD)" sub="Skip pools below this TVL">
        <NumInput value={cfg.minLiquidity} step="1000" onChange={(e) => setParam("longtail", { minLiquidity: parseFloat(e.target.value) })} />
      </Field>
      <Field label="Max price impact %" sub="Reject routes above this">
        <NumInput value={cfg.maxPriceImpact} step="0.1" onChange={(e) => setParam("longtail", { maxPriceImpact: parseFloat(e.target.value) })} />
      </Field>
      <Field label="Include cross-chain routes" sub="Simulate routes bridging to other chains">
        <div className="flex items-center gap-2 h-9"><Switch checked={cfg.crossChain} onCheckedChange={(v) => setParam("longtail", { crossChain: v })} /><span className="font-mono text-xs text-[var(--ink-dim)]">{cfg.crossChain ? "On" : "Off"}</span></div>
      </Field>
      {cfg.crossChain && (
        <>
          <Field label="Bridge protocols" className="col-span-2">
            <Chips options={BRIDGE_PROTOCOLS} value={cfg.bridgeProtocols} onToggle={(b) => setParam("longtail", { bridgeProtocols: cfg.bridgeProtocols.includes(b) ? cfg.bridgeProtocols.filter((x) => x !== b) : [...cfg.bridgeProtocols, b] })} />
          </Field>
          <Field label="Bridge fee assumption %">
            <NumInput value={cfg.bridgeFee} step="0.05" onChange={(e) => setParam("longtail", { bridgeFee: parseFloat(e.target.value) })} />
          </Field>
        </>
      )}
      <Field label="Token universe" sub="Major: top 20 · Mid-cap: top 100 · Long-tail: all">
        <Segmented options={[{value:"major",label:"Major"},{value:"midcap",label:"Mid-cap"},{value:"longtail",label:"Long-tail"}]} value={cfg.tokenUniverse} onChange={(v) => setParam("longtail", { tokenUniverse: v as "major"|"midcap"|"longtail" })} />
      </Field>
      <Field label={`Min profit after bridge + gas (${chain.nativeToken})`}>
        <NumInput value={cfg.minProfitAfterBridge} step="0.001" onChange={(e) => setParam("longtail", { minProfitAfterBridge: parseFloat(e.target.value) })} />
      </Field>
      <div className="col-span-2">
        <PnlFormulaCard
          title="Live P&L model"
          lines={[
            { label: "Route profit", value: `${profit.toFixed(5)} ${chain.nativeToken}` },
            { label: "Price impact", value: `−${impact.toFixed(5)}` },
            { label: "Bridge cost", value: `−${bridge.toFixed(5)}` },
            { label: `Gas (${cfg.routeDepth} hops)`, value: `−${gas.toFixed(5)}` },
          ]}
          net={`${net.toFixed(5)} ${chain.nativeToken}`}
        />
      </div>
    </div>
  );
}