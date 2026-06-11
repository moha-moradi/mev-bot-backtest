import { createFileRoute, Link } from "@tanstack/react-router";
import { useEffect } from "react";
import { useSimulationStore } from "@/store/simulationStore";
import { SectionCard } from "@/components/shared/SectionCard";
import { EmptyState } from "@/components/shared/EmptyState";
import { StrategyBadge } from "@/components/shared/StrategyBadge";
import { ChainDot } from "@/components/shared/ChainDot";
import { getChain } from "@/lib/chains";
import { format, formatDistanceToNow } from "date-fns";
import { History as HistoryIcon, Trash2, ExternalLink } from "lucide-react";

export const Route = createFileRoute("/history")({
  head: () => ({ meta: [{ title: "History — MEVSCOPE" }] }),
  component: HistoryPage,
});

function HistoryPage() {
  const history = useSimulationStore((s) => s.history);
  const removeRun = useSimulationStore((s) => s.removeRun);
  const loadHistory = useSimulationStore((s) => s.loadHistory);

  useEffect(() => { void loadHistory(); }, [loadHistory]);

  if (!history.length) {
    return (
      <div className="p-4">
        <SectionCard title="History">
          <EmptyState icon={HistoryIcon} title="No simulation runs yet" body="Past simulation runs will appear here." action={<Link to="/configure" className="rounded-md bg-[var(--acc-green)] px-3 py-1.5 font-mono text-xs font-semibold text-black">Configure first run</Link>} />
        </SectionCard>
      </div>
    );
  }

  return (
    <div className="p-4">
      <SectionCard title={`History (${history.length})`}>
        <div className="overflow-auto rounded-md border border-[var(--line)]">
          <table className="w-full font-mono text-xs">
            <thead className="bg-[var(--surface-2)] text-[10px] uppercase tracking-[0.14em] text-[var(--ink-dim)]">
              <tr>
                <th className="px-3 py-2 text-left">Run ID</th>
                <th className="px-3 py-2 text-left">When</th>
                <th className="px-3 py-2 text-left">Chain</th>
                <th className="px-3 py-2 text-left">Window</th>
                <th className="px-3 py-2 text-left">Strategies</th>
                <th className="px-3 py-2 text-right">Opps</th>
                <th className="px-3 py-2 text-right">Net</th>
                <th className="px-3 py-2 text-right">Duration</th>
                <th className="px-3 py-2"></th>
              </tr>
            </thead>
            <tbody>
              {history.map((r) => {
                const c = getChain(r.chainId);
                return (
                  <tr key={r.id} className="border-t border-[var(--line)]/60 hover:bg-[var(--surface-2)]/50">
                    <td className="px-3 py-2 text-[var(--ink)]">{r.id}</td>
                    <td className="px-3 py-2 text-[var(--ink-dim)]">{format(r.startedAt, "MMM d HH:mm")} <span className="text-[var(--ink-mute)]">({formatDistanceToNow(r.startedAt, { addSuffix: true })})</span></td>
                    <td className="px-3 py-2"><span className="inline-flex items-center gap-1.5"><ChainDot color={c.color} /> {c.name}</span></td>
                    <td className="px-3 py-2 text-[var(--ink-dim)]">{r.windowSummary}</td>
                    <td className="px-3 py-2"><div className="flex flex-wrap gap-1">{r.enabledStrategies.map((s) => <StrategyBadge key={s} id={s} size="xs" />)}</div></td>
                    <td className="px-3 py-2 text-right">{r.opportunities}</td>
                    <td className="px-3 py-2 text-right text-[var(--acc-green)]">{r.netProfit.toFixed(4)} {c.nativeToken}</td>
                    <td className="px-3 py-2 text-right text-[var(--ink-dim)]">{Math.round(r.durationMs / 1000)}s</td>
                    <td className="px-3 py-2 text-right">
                      <div className="flex items-center justify-end gap-2">
                        <Link to="/report" className="text-[var(--ink-dim)] hover:text-[var(--ink)]"><ExternalLink className="h-3.5 w-3.5" /></Link>
                        <button onClick={() => removeRun(r.id)} className="text-[var(--ink-dim)] hover:text-[var(--acc-red)]"><Trash2 className="h-3.5 w-3.5" /></button>
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </SectionCard>
    </div>
  );
}