import { createFileRoute, Link } from "@tanstack/react-router";
import { useMemo, useState } from "react";
import { useSimulationStore } from "@/store/simulationStore";
import { SectionCard } from "@/components/shared/SectionCard";
import { MetricCard } from "@/components/shared/MetricCard";
import { StrategyBadge } from "@/components/shared/StrategyBadge";
import { MonoHash } from "@/components/shared/MonoHash";
import { EmptyState } from "@/components/shared/EmptyState";
import { STRATEGY_DEFS } from "@/lib/strategyDefs";
import type { StrategyId } from "@/lib/chains";
import { formatNative, formatAge, formatPct } from "@/lib/formatters";
import { Target, Search, ExternalLink, ChevronRight } from "lucide-react";
import { cn } from "@/lib/utils";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import type { Opportunity } from "@/lib/mockData";

export const Route = createFileRoute("/opportunities")({
  head: () => ({ meta: [{ title: "Opportunities — MEVSCOPE" }] }),
  component: OppPage,
});

type ResultFilter = "all" | "profitable" | "unprofitable";

function OppPage() {
  const opps = useSimulationStore((s) => s.results.opportunities);
  const chain = useSimulationStore((s) => s.selectedChain);
  const simulationMode = useSimulationStore((s) => s.simulationMode);
  const [strategy, setStrategy] = useState<StrategyId | "all">("all");
  const [result, setResult] = useState<ResultFilter>("all");
  const [search, setSearch] = useState("");
  const [active, setActive] = useState<Opportunity | null>(null);

  const filtered = useMemo(() => opps.filter((o) => {
    if (strategy !== "all" && o.strategy !== strategy) return false;
    if (result === "profitable" && o.result !== "profitable") return false;
    if (result === "unprofitable" && o.result === "profitable") return false;
    if (search && !o.txHash.toLowerCase().includes(search.toLowerCase())) return false;
    return true;
  }), [opps, strategy, result, search]);

  if (!opps.length) {
    return (
      <div className="p-4">
        <SectionCard title="Opportunities">
          <EmptyState icon={Target} title="No opportunities yet" body="Run a simulation to populate opportunities." action={<Link to="/configure" className="rounded-md bg-[var(--acc-green)] px-3 py-1.5 font-mono text-xs font-semibold text-black">Configure</Link>} />
        </SectionCard>
      </div>
    );
  }

  const totals = {
    total: opps.length,
    profitable: opps.filter((o) => o.result === "profitable").length,
    gross: opps.reduce((a, o) => a + o.grossRevenue, 0),
    net: opps.filter((o) => o.result === "profitable").reduce((a, o) => a + o.netProfit, 0),
    cost: opps.reduce((a, o) => a + o.gasCost + o.flashLoanFee + o.builderTip, 0),
  };

  return (
    <div className="p-4 space-y-4">
      <div className="grid grid-cols-2 md:grid-cols-5 gap-3">
        <MetricCard label="Total" value={totals.total.toLocaleString()} />
        <MetricCard label="Profitable" value={totals.profitable.toLocaleString()} sub={formatPct(totals.profitable / totals.total)} />
        <MetricCard label="Gross revenue" value={formatNative(totals.gross, chain, 3)} accent="var(--ink)" />
        <MetricCard label="Net profit" value={formatNative(totals.net, chain, 3)} accent="var(--acc-green)" />
        <MetricCard label="Total cost" value={formatNative(totals.cost, chain, 4)} accent="var(--acc-red)" />
      </div>

      <SectionCard title={`Opportunities (${filtered.length})`}>
        <div className="flex flex-wrap items-center gap-2 mb-3">
          <div className="flex flex-wrap gap-1">
            {(["all", ...STRATEGY_DEFS.map((s) => s.id)] as const).map((s) => (
              <button key={s} onClick={() => setStrategy(s as StrategyId | "all")} className={cn("rounded-sm px-2 py-1 font-mono text-[10px] uppercase tracking-[0.14em]", strategy === s ? "bg-[var(--panel)] text-[var(--ink)]" : "bg-[var(--surface-2)] text-[var(--ink-dim)] hover:text-[var(--ink)]")}>
                {s === "all" ? "ALL" : STRATEGY_DEFS.find((d) => d.id === s)?.short}
              </button>
            ))}
          </div>
          <div className="h-4 w-px bg-[var(--line)]" />
          <div className="flex gap-1">
            {(["all", "profitable", "unprofitable"] as const).map((r) => (
              <button key={r} onClick={() => setResult(r)} className={cn("rounded-sm px-2 py-1 font-mono text-[10px] uppercase tracking-[0.14em]", result === r ? "bg-[var(--panel)] text-[var(--ink)]" : "bg-[var(--surface-2)] text-[var(--ink-dim)]")}>{r}</button>
            ))}
          </div>
          <div className="ml-auto flex items-center gap-2 rounded-md border border-[var(--line)] bg-[var(--surface-2)] px-2 py-1">
            <Search className="h-3 w-3 text-[var(--ink-mute)]" />
            <input value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Search tx hash…" className="bg-transparent text-xs font-mono outline-none w-48 placeholder:text-[var(--ink-mute)]" />
          </div>
        </div>

        <div className="overflow-auto rounded-md border border-[var(--line)]">
          <table className="w-full font-mono text-xs">
            <thead>
              <tr className="border-b border-[var(--line)] bg-[var(--surface-2)] text-[10px] uppercase tracking-[0.14em] text-[var(--ink-dim)]">
                <th className="px-3 py-2 text-left">Tx</th>
                <th className="px-3 py-2 text-left">Block</th>
                <th className="px-3 py-2 text-left">Age</th>
                <th className="px-3 py-2 text-left">Strategy</th>
                <th className="px-3 py-2 text-left">Detail</th>
                <th className="px-3 py-2 text-right">Gas</th>
                <th className="px-3 py-2 text-right">Net profit</th>
                <th className="px-3 py-2 text-left">Result</th>
                <th className="px-3 py-2"></th>
              </tr>
            </thead>
            <tbody>
              {filtered.slice(0, 200).map((o) => (
                <tr key={o.id} onClick={() => setActive(o)} className="border-b border-[var(--line)]/60 hover:bg-[var(--surface-2)]/50 cursor-pointer">
                  <td className="px-3 py-2">
                    <MonoHash hash={o.txHash} href={o.explorerUrl} />
                  </td>
                  <td className="px-3 py-2 text-[var(--ink-dim)]">{o.blockNumber.toLocaleString()}</td>
                  <td className="px-3 py-2 text-[var(--ink-dim)]">{formatAge(o.timestamp)}</td>
                  <td className="px-3 py-2"><StrategyBadge id={o.strategy} /></td>
                  <td className="px-3 py-2 text-[var(--ink-dim)] max-w-[280px] truncate">{renderDetail(o)}</td>
                  <td className="px-3 py-2 text-right text-[var(--ink-dim)]">{o.gasCost.toFixed(5)}</td>
                  <td className={cn("px-3 py-2 text-right tabular-nums", o.netProfit > 0 ? "text-[var(--acc-green)]" : "text-[var(--acc-red)]")}>
                    {o.netProfit > 0 ? "+" : ""}{o.netProfit.toFixed(5)}
                  </td>
                  <td className="px-3 py-2">
                    <span className={cn("rounded-sm px-1.5 py-0.5 text-[9px] uppercase tracking-[0.14em]", o.result === "profitable" ? "bg-[var(--acc-green)]/10 text-[var(--acc-green)]" : o.result === "reverted" ? "bg-[var(--acc-red)]/10 text-[var(--acc-red)]" : "bg-[var(--surface-2)] text-[var(--ink-dim)]")}>
                      {o.result === "below_threshold" ? "below" : o.result}
                    </span>
                  </td>
                  <td className="px-3 py-2 text-right"><ChevronRight className="inline h-3 w-3 text-[var(--ink-mute)]" /></td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        {filtered.length > 200 && <p className="mt-2 font-mono text-[10px] text-[var(--ink-dim)]">Showing 200 of {filtered.length}. Refine filters to narrow.</p>}
      </SectionCard>

      <Dialog open={!!active} onOpenChange={(o) => !o && setActive(null)}>
        <DialogContent className="bg-[var(--surface)] border-[var(--line)] max-w-xl">
          {active && <TraceView opp={active} chain={chain.name} explorer={active.explorerUrl} simulationMode={simulationMode} />}
        </DialogContent>
      </Dialog>
    </div>
  );
}

function renderDetail(o: Opportunity) {
  if (o.strategy === "sandwich") return `victim ${o.victimTxHash?.slice(0, 10)}… · slip ${(o.victimSlippage! * 100).toFixed(2)}%`;
  if (o.strategy === "liquidation") return `${o.protocol} · HF ${o.healthFactor?.toFixed(2)} · ${o.debtRepaid?.toFixed(0)} USDC`;
  if (o.strategy === "longtail") return `${(o.route || []).join("→")} · ${o.hopCount} hops`;
  if (o.strategy === "aggregator") return `${o.aggregator} vs ${o.aggregatorAlt} · ${o.tokenPair} · ${((o.spreadBps || 0) / 100).toFixed(2)}%`;
  return `${(o.dexPath || []).join(" → ")} · ${o.tokenPair}`;
}

function TraceView({ opp, chain, explorer, simulationMode }: { opp: Opportunity; chain: string; explorer: string; simulationMode: "mock" | "api" }) {
  return (
    <>
      <DialogHeader>
        <DialogTitle className="font-mono text-sm uppercase tracking-[0.18em] flex items-center gap-2">
          <StrategyBadge id={opp.strategy} /> {opp.simulationTrace.title}
          <a href={explorer} target="_blank" rel="noopener noreferrer" className="ml-auto text-[var(--ink-dim)] hover:text-[var(--ink)]"><ExternalLink className="h-3.5 w-3.5" /></a>
        </DialogTitle>
      </DialogHeader>
      <div className="rounded-md border border-[var(--line)] bg-[#080b0f] p-4 font-mono text-xs space-y-1.5">
        {opp.simulationTrace.steps.map((s, i) => (
          <div key={i} className="grid grid-cols-[110px_1fr] gap-3">
            <span className="text-[var(--ink-dim)]">{s.label}</span>
            <div>
              {s.value && <span className="text-[var(--ink)] break-all">{s.value}</span>}
              {s.sub && <div className="text-[var(--ink-dim)] mt-0.5">{s.sub}</div>}
            </div>
          </div>
        ))}
        <div className="mt-3 pt-3 border-t border-[var(--line)] grid grid-cols-3 gap-3">
          <div><div className="text-[var(--ink-dim)] text-[10px] uppercase">Gross</div><div className="text-[var(--ink)] mt-0.5">{opp.simulationTrace.result.gross.toFixed(5)}</div></div>
          <div><div className="text-[var(--ink-dim)] text-[10px] uppercase">Cost</div><div className="text-[var(--acc-red)] mt-0.5">−{opp.simulationTrace.result.cost.toFixed(5)}</div></div>
          <div><div className="text-[var(--ink-dim)] text-[10px] uppercase">Net</div><div className={opp.simulationTrace.result.net > 0 ? "text-[var(--acc-green)] mt-0.5 font-semibold" : "text-[var(--acc-red)] mt-0.5"}>{opp.simulationTrace.result.net > 0 ? "+" : ""}{opp.simulationTrace.result.net.toFixed(5)}</div></div>
        </div>
      </div>
      {simulationMode === "mock" && <p className="font-mono text-[10px] text-[var(--ink-mute)] mt-2">{chain} · simulated locally with deterministic mock RNG</p>}
    </>
  );
}