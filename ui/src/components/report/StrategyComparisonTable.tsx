import { useSimulationStore } from "@/store/simulationStore";
import { SectionCard } from "../shared/SectionCard";
import { STRATEGY_BY_ID } from "@/lib/strategyDefs";
import { formatNative, formatUSD, formatPct } from "@/lib/formatters";
import { Link } from "@tanstack/react-router";
import { cn } from "@/lib/utils";

export function StrategyComparisonTable() {
  const byStrategy = useSimulationStore((s) => s.results.byStrategy);
  const chain = useSimulationStore((s) => s.selectedChain);
  const metric = useSimulationStore((s) => s.reportMetric);
  const setMetric = useSimulationStore((s) => s.setReportMetric);
  const config = useSimulationStore((s) => s.config);

  const rows = Object.values(byStrategy).sort((a, b) => metric === "roi" ? b.roi - a.roi : b.netProfit - a.netProfit);
  const best = rows[0]?.strategy;

  const totals = rows.reduce((acc, r) => ({
    count: acc.count + r.count, profitable: acc.profitable + r.profitable,
    gross: acc.gross + r.grossRevenue, gas: acc.gas + r.gasFees, net: acc.net + r.netProfit,
    netUSD: acc.netUSD + r.netProfitUSD,
  }), { count: 0, profitable: 0, gross: 0, gas: 0, net: 0, netUSD: 0 });

  return (
    <SectionCard
      title={`Strategy performance — last ${config.lastDays} days`}
      action={
        <div className="inline-flex rounded-md border border-[var(--line)] bg-[var(--surface-2)] p-0.5">
          {(["roi", "raw"] as const).map((m) => (
            <button key={m} onClick={() => setMetric(m)} className={cn("px-2.5 py-1 font-mono text-[10px] uppercase tracking-[0.14em] rounded-sm", metric === m ? "bg-[var(--panel)] text-[var(--ink)]" : "text-[var(--ink-dim)]")}>
              {m === "roi" ? "ROI %" : "Raw profit"}
            </button>
          ))}
        </div>
      }
    >
      <div className="overflow-auto rounded-md border border-[var(--line)]">
        <table className="w-full font-mono text-xs">
          <thead className="text-[10px] uppercase tracking-[0.14em] text-[var(--ink-dim)] bg-[var(--surface-2)]">
            <tr>
              <th className="px-3 py-2 text-left">#</th>
              <th className="px-3 py-2 text-left">Strategy</th>
              <th className="px-3 py-2 text-right">Opps</th>
              <th className="px-3 py-2 text-right">Profitable</th>
              <th className="px-3 py-2 text-right">Success</th>
              <th className="px-3 py-2 text-right">Gross</th>
              <th className="px-3 py-2 text-right">Gas + fees</th>
              <th className="px-3 py-2 text-right">Net ({chain.nativeToken})</th>
              <th className="px-3 py-2 text-right">Net (USD)</th>
              <th className="px-3 py-2 text-right">ROI %</th>
              <th className="px-3 py-2 text-right">Avg/opp</th>
              <th className="px-3 py-2 text-right">Best opp</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((r, i) => {
              const def = STRATEGY_BY_ID[r.strategy];
              const isBest = r.strategy === best;
              return (
                <tr key={r.strategy}
                    className={cn("border-t border-[var(--line)]/60 hover:bg-[var(--surface-2)]/50")}
                    style={{ borderLeft: `2px solid ${def.color}`, backgroundColor: isBest ? `${def.color}08` : undefined }}>
                  <td className="px-3 py-2 text-[var(--ink-dim)]">{i + 1}</td>
                  <td className="px-3 py-2"><Link to="/opportunities" className="inline-flex items-center gap-2 hover:text-[var(--acc-green)]"><span className="h-1.5 w-1.5 rounded-full" style={{ backgroundColor: def.color }} />{def.name}</Link></td>
                  <td className="px-3 py-2 text-right">{r.count}</td>
                  <td className="px-3 py-2 text-right">{r.profitable}</td>
                  <td className="px-3 py-2 text-right">{formatPct(r.profitable / Math.max(1, r.count))}</td>
                  <td className="px-3 py-2 text-right">{r.grossRevenue.toFixed(4)}</td>
                  <td className="px-3 py-2 text-right text-[var(--acc-red)]">−{r.gasFees.toFixed(4)}</td>
                  <td className="px-3 py-2 text-right text-[var(--acc-green)]">{r.netProfit.toFixed(4)}</td>
                  <td className="px-3 py-2 text-right text-[var(--acc-green)]">{formatUSD(r.netProfitUSD)}</td>
                  <td className="px-3 py-2 text-right">{r.roi.toFixed(1)}%</td>
                  <td className="px-3 py-2 text-right">{r.avgPerOpp.toFixed(5)}</td>
                  <td className="px-3 py-2 text-right">{r.bestOpp.toFixed(4)}</td>
                </tr>
              );
            })}
            <tr className="border-t border-[var(--line)] bg-[var(--surface-2)] font-semibold">
              <td colSpan={2} className="px-3 py-2 text-[var(--ink-dim)] uppercase tracking-[0.14em] text-[10px]">Total</td>
              <td className="px-3 py-2 text-right">{totals.count}</td>
              <td className="px-3 py-2 text-right">{totals.profitable}</td>
              <td className="px-3 py-2 text-right">{formatPct(totals.profitable / Math.max(1, totals.count))}</td>
              <td className="px-3 py-2 text-right">{totals.gross.toFixed(4)}</td>
              <td className="px-3 py-2 text-right text-[var(--acc-red)]">−{totals.gas.toFixed(4)}</td>
              <td className="px-3 py-2 text-right text-[var(--acc-green)]">{totals.net.toFixed(4)}</td>
              <td className="px-3 py-2 text-right text-[var(--acc-green)]">{formatUSD(totals.netUSD)}</td>
              <td colSpan={3}></td>
            </tr>
          </tbody>
        </table>
      </div>
      {rows.length >= 2 && (
        <div className="mt-3 rounded-md border border-[var(--line)] bg-[var(--surface-2)] p-3 text-xs text-[var(--ink-dim)]">
          <span className="font-mono uppercase tracking-[0.14em] text-[10px] text-[var(--acc-green)] mr-2">Insight</span>
          {generateInsight(rows, chain.nativeToken)}
        </div>
      )}
    </SectionCard>
  );
}

function generateInsight(rows: { strategy: string; count: number; netProfit: number; avgPerOpp: number }[], native: string) {
  if (rows.length < 2) return "Run more strategies to compare.";
  const top = rows[0], second = rows[1];
  const ratio = top.count && second.count ? (top.count / second.count).toFixed(1) : "—";
  return `${capitalize(top.strategy)} generated ${ratio}× more opportunities than ${second.strategy}, with avg ${top.avgPerOpp.toFixed(5)} ${native} per execution vs ${second.avgPerOpp.toFixed(5)} ${native}.`;
}
const capitalize = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);