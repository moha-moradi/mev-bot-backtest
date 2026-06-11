import { useSimulationStore } from "@/store/simulationStore";
import { SectionCard } from "../shared/SectionCard";
import { STRATEGY_BY_ID, STRATEGY_DEFS } from "@/lib/strategyDefs";
import { ResponsiveContainer, ComposedChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, Legend, AreaChart, Area, PieChart, Pie, Cell, BarChart, ScatterChart, Scatter, ZAxis } from "recharts";
import { useMemo, useState } from "react";
import type { StrategyId } from "@/lib/chains";
import { cn } from "@/lib/utils";

const tooltipStyle = { backgroundColor: "#0d1017", border: "1px solid #1e2330", borderRadius: 6, fontFamily: "var(--font-mono)", fontSize: 11 };

export function WaterfallChart() {
  const byStrategy = useSimulationStore((s) => s.results.byStrategy);
  const data = Object.values(byStrategy).map((s) => ({
    name: STRATEGY_BY_ID[s.strategy as StrategyId].short,
    color: STRATEGY_BY_ID[s.strategy as StrategyId].color,
    Gross: +s.grossRevenue.toFixed(5),
    Gas: -+(s.gasFees * 0.7).toFixed(5),
    FL: -+(s.gasFees * 0.1).toFixed(5),
    Tip: -+(s.gasFees * 0.2).toFixed(5),
    Net: +s.netProfit.toFixed(5),
  }));
  return (
    <SectionCard title="Profit waterfall by strategy">
      <div className="h-80">
        <ResponsiveContainer width="100%" height="100%">
          <ComposedChart data={data} margin={{ top: 10, right: 16, left: 0, bottom: 0 }}>
            <CartesianGrid stroke="#1e2330" vertical={false} />
            <XAxis dataKey="name" stroke="#64748b" tick={{ fontFamily: "var(--font-mono)", fontSize: 10 }} />
            <YAxis stroke="#64748b" tick={{ fontFamily: "var(--font-mono)", fontSize: 10 }} />
            <Tooltip contentStyle={tooltipStyle} />
            <Legend wrapperStyle={{ fontFamily: "var(--font-mono)", fontSize: 10 }} />
            <Bar dataKey="Gross" stackId="a" fill="#38bdf8">{data.map((d, i) => <Cell key={i} fill={d.color} />)}</Bar>
            <Bar dataKey="Gas" stackId="b" fill="#f87171" />
            <Bar dataKey="FL" stackId="b" fill="#a78bfa" />
            <Bar dataKey="Tip" stackId="b" fill="#f59e0b" />
            <Bar dataKey="Net" stackId="c" fill="transparent" stroke="#00ff94" strokeWidth={2} />
          </ComposedChart>
        </ResponsiveContainer>
      </div>
    </SectionCard>
  );
}

export function CumulativeChart() {
  const opps = useSimulationStore((s) => s.results.opportunities);
  const chain = useSimulationStore((s) => s.selectedChain);
  const data = useMemo(() => {
    const enabled = Array.from(new Set(opps.map((o) => o.strategy)));
    const sorted = [...opps].sort((a, b) => a.blockNumber - b.blockNumber);
    const acc: Record<string, number> = {};
    enabled.forEach((s) => (acc[s] = 0));
    return sorted.map((o, i) => {
      if (o.result === "profitable") acc[o.strategy] = (acc[o.strategy] || 0) + o.netProfit;
      return { block: o.blockNumber, idx: i, ...acc };
    }).filter((_, i, arr) => i % Math.max(1, Math.floor(arr.length / 80)) === 0);
  }, [opps]);

  const enabledStrats = Array.from(new Set(opps.map((o) => o.strategy)));

  return (
    <SectionCard title={`Cumulative profit · ${chain.nativeToken}`}>
      <div className="h-72">
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart data={data} margin={{ top: 10, right: 16, left: 0, bottom: 0 }}>
            <defs>
              {enabledStrats.map((s) => (
                <linearGradient key={s} id={`grad-${s}`} x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor={STRATEGY_BY_ID[s].color} stopOpacity={0.3} />
                  <stop offset="95%" stopColor={STRATEGY_BY_ID[s].color} stopOpacity={0} />
                </linearGradient>
              ))}
            </defs>
            <CartesianGrid stroke="#1e2330" vertical={false} />
            <XAxis dataKey="block" stroke="#64748b" tick={{ fontFamily: "var(--font-mono)", fontSize: 9 }} />
            <YAxis stroke="#64748b" tick={{ fontFamily: "var(--font-mono)", fontSize: 10 }} />
            <Tooltip contentStyle={tooltipStyle} />
            {enabledStrats.map((s) => (
              <Area key={s} type="monotone" dataKey={s} stroke={STRATEGY_BY_ID[s].color} fill={`url(#grad-${s})`} strokeWidth={1.5} />
            ))}
          </AreaChart>
        </ResponsiveContainer>
      </div>
    </SectionCard>
  );
}

export function StrategyDonut() {
  const byStrategy = useSimulationStore((s) => s.results.byStrategy);
  const chain = useSimulationStore((s) => s.selectedChain);
  const [mode, setMode] = useState<"profit" | "count">("profit");
  const data = Object.values(byStrategy).map((s) => ({
    name: STRATEGY_BY_ID[s.strategy as StrategyId].short, color: STRATEGY_BY_ID[s.strategy as StrategyId].color,
    value: mode === "profit" ? Math.max(0, s.netProfit) : s.count,
    raw: s.netProfit, count: s.count,
  }));
  const total = data.reduce((a, d) => a + d.value, 0);
  return (
    <SectionCard title="Strategy share" action={
      <div className="inline-flex rounded-md border border-[var(--line)] bg-[var(--surface-2)] p-0.5">
        {(["profit", "count"] as const).map((m) => (
          <button key={m} onClick={() => setMode(m)} className={cn("px-2 py-1 font-mono text-[10px] uppercase tracking-[0.14em] rounded-sm", mode === m ? "bg-[var(--panel)] text-[var(--ink)]" : "text-[var(--ink-dim)]")}>by {m}</button>
        ))}
      </div>
    }>
      <div className="h-56">
        <ResponsiveContainer width="100%" height="100%">
          <PieChart>
            <Pie data={data} dataKey="value" innerRadius={55} outerRadius={85} stroke="#0a0b0d" strokeWidth={2}>
              {data.map((d, i) => <Cell key={i} fill={d.color} />)}
            </Pie>
            <Tooltip contentStyle={tooltipStyle} />
          </PieChart>
        </ResponsiveContainer>
      </div>
      <div className="mt-2 space-y-1">
        {data.map((d) => (
          <div key={d.name} className="flex items-center justify-between text-xs font-mono">
            <span className="flex items-center gap-2"><span className="h-1.5 w-1.5 rounded-full" style={{ backgroundColor: d.color }} /><span className="text-[var(--ink)]">{d.name}</span></span>
            <span className="text-[var(--ink-dim)]">{d.count} · {d.raw.toFixed(4)} {chain.nativeToken} · {total ? ((d.value / total) * 100).toFixed(1) : 0}%</span>
          </div>
        ))}
      </div>
    </SectionCard>
  );
}

export function DexTable() {
  const byDex = useSimulationStore((s) => s.results.byDex);
  const chain = useSimulationStore((s) => s.selectedChain);
  return (
    <SectionCard title="DEX performance">
      <div className="overflow-auto rounded-md border border-[var(--line)]">
        <table className="w-full font-mono text-xs">
          <thead className="bg-[var(--surface-2)] text-[10px] uppercase tracking-[0.14em] text-[var(--ink-dim)]">
            <tr><th className="px-3 py-2 text-left">DEX</th><th className="px-3 py-2 text-left">Fork</th><th className="px-3 py-2 text-right">Tx count</th><th className="px-3 py-2 text-right">Opps</th><th className="px-3 py-2 text-right">Profitable</th><th className="px-3 py-2 text-right">Revenue ({chain.nativeToken})</th><th className="px-3 py-2 text-right">Avg/opp</th></tr>
          </thead>
          <tbody>
            {byDex.map((d) => (
              <tr key={d.dex} className="border-t border-[var(--line)]/60">
                <td className="px-3 py-2 text-[var(--ink)]">{d.dex}</td>
                <td className="px-3 py-2 text-[var(--ink-dim)]">{d.fork}</td>
                <td className="px-3 py-2 text-right">{d.txCount}</td>
                <td className="px-3 py-2 text-right">{d.opportunities}</td>
                <td className="px-3 py-2 text-right">{d.profitable}</td>
                <td className="px-3 py-2 text-right text-[var(--acc-green)]">{d.revenue.toFixed(4)}</td>
                <td className="px-3 py-2 text-right">{d.avgProfit.toFixed(5)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </SectionCard>
  );
}

export function LiquidationAnalyticsView() {
  const data = useSimulationStore((s) => s.results.liquidationAnalytics);
  if (!data) return null;
  const chartData = data.hfDistribution;
  const colors = ["#f87171", "#fb923c", "#f59e0b", "#fbbf24"];
  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
      <SectionCard title="Liquidation by protocol">
        <div className="overflow-auto rounded-md border border-[var(--line)]">
          <table className="w-full font-mono text-xs">
            <thead className="bg-[var(--surface-2)] text-[10px] uppercase tracking-[0.14em] text-[var(--ink-dim)]">
              <tr><th className="px-3 py-2 text-left">Protocol</th><th className="px-3 py-2 text-right">Scanned</th><th className="px-3 py-2 text-right">Targeted</th><th className="px-3 py-2 text-right">Profitable</th><th className="px-3 py-2 text-right">Debt repaid</th><th className="px-3 py-2 text-right">Net</th></tr>
            </thead>
            <tbody>
              {data.byProtocol.map((r) => (
                <tr key={r.protocol} className="border-t border-[var(--line)]/60">
                  <td className="px-3 py-2 text-[var(--ink)]">{r.protocol}</td>
                  <td className="px-3 py-2 text-right">{r.scanned}</td>
                  <td className="px-3 py-2 text-right">{r.targeted}</td>
                  <td className="px-3 py-2 text-right">{r.profitable}</td>
                  <td className="px-3 py-2 text-right">{r.debtRepaid.toFixed(0)}</td>
                  <td className="px-3 py-2 text-right text-[var(--acc-green)]">{r.netProfit.toFixed(4)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </SectionCard>
      <SectionCard title="Health factor distribution">
        <div className="h-64">
          <ResponsiveContainer width="100%" height="100%">
            <BarChart data={chartData} margin={{ top: 10, right: 16, left: 0, bottom: 0 }}>
              <CartesianGrid stroke="#1e2330" vertical={false} />
              <XAxis dataKey="bucket" stroke="#64748b" tick={{ fontFamily: "var(--font-mono)", fontSize: 10 }} />
              <YAxis stroke="#64748b" tick={{ fontFamily: "var(--font-mono)", fontSize: 10 }} />
              <Tooltip contentStyle={tooltipStyle} />
              <Bar dataKey="count" radius={[4, 4, 0, 0]}>{chartData.map((_, i) => <Cell key={i} fill={colors[i]} />)}</Bar>
            </BarChart>
          </ResponsiveContainer>
        </div>
      </SectionCard>
    </div>
  );
}

export function LongtailAnalyticsView() {
  const data = useSimulationStore((s) => s.results.longtailAnalytics);
  if (!data) return null;
  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
      <SectionCard title="Top long-tail routes">
        <div className="overflow-auto rounded-md border border-[var(--line)] max-h-72">
          <table className="w-full font-mono text-xs">
            <thead className="bg-[var(--surface-2)] text-[10px] uppercase tracking-[0.14em] text-[var(--ink-dim)] sticky top-0">
              <tr><th className="px-3 py-2 text-left">Route</th><th className="px-3 py-2 text-right">Hops</th><th className="px-3 py-2 text-right">Avg impact</th><th className="px-3 py-2 text-right">Execs</th><th className="px-3 py-2 text-right">Total net</th></tr>
            </thead>
            <tbody>
              {data.topRoutes.map((r) => (
                <tr key={r.route.join("-")} className="border-t border-[var(--line)]/60">
                  <td className="px-3 py-2 text-[var(--ink)]">{r.route.join(" → ")}</td>
                  <td className="px-3 py-2 text-right">{r.hops}</td>
                  <td className="px-3 py-2 text-right">{(r.avgImpact * 100).toFixed(2)}%</td>
                  <td className="px-3 py-2 text-right">{r.executions}</td>
                  <td className="px-3 py-2 text-right text-[var(--acc-green)]">{r.totalProfit.toFixed(5)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </SectionCard>
      <SectionCard title="Hops vs profit per execution">
        <div className="h-72">
          <ResponsiveContainer width="100%" height="100%">
            <ScatterChart margin={{ top: 10, right: 16, left: 0, bottom: 10 }}>
              <CartesianGrid stroke="#1e2330" />
              <XAxis type="number" dataKey="hops" name="Hops" domain={[1.5, 4.5]} stroke="#64748b" tick={{ fontFamily: "var(--font-mono)", fontSize: 10 }} />
              <YAxis type="number" dataKey="profit" name="Profit" stroke="#64748b" tick={{ fontFamily: "var(--font-mono)", fontSize: 10 }} />
              <ZAxis type="number" dataKey="profit" range={[40, 280]} />
              <Tooltip contentStyle={tooltipStyle} cursor={{ stroke: "#1e2330" }} />
              <Scatter data={data.scatterData} fill="#22d3ee" opacity={0.7} />
            </ScatterChart>
          </ResponsiveContainer>
        </div>
      </SectionCard>
    </div>
  );
}

export const ALL_STRATEGY_DEFS = STRATEGY_DEFS;