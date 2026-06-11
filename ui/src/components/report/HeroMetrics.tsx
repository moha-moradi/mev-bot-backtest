import { MetricCard } from "@/components/shared/MetricCard";
import { useSimulationStore } from "@/store/simulationStore";
import { formatNative, formatUSD, formatPct } from "@/lib/formatters";

export function HeroMetrics() {
  const s = useSimulationStore((st) => st.results.summary);
  const chain = useSimulationStore((st) => st.selectedChain);
  if (!s) return null;
  return (
    <div className="grid grid-cols-2 md:grid-cols-6 gap-3">
      <MetricCard label="Opportunities" value={s.total.toLocaleString()} />
      <MetricCard label="Profitable" value={s.profitable.toLocaleString()} />
      <MetricCard label="Success rate" value={formatPct(s.profitable / Math.max(1, s.total))} accent="var(--acc-green)" />
      <MetricCard label="Gross revenue" value={formatNative(s.grossRevenue, chain, 3)} />
      <MetricCard label={`Net profit (${chain.nativeToken})`} value={formatNative(s.netProfit, chain, 4)} accent="var(--acc-green)" />
      <MetricCard label="Net profit (USD)" value={formatUSD(s.netProfitUSD)} accent="var(--acc-green)" />
    </div>
  );
}