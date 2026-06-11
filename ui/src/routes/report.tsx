import { createFileRoute, Link } from "@tanstack/react-router";
import { motion } from "framer-motion";
import { useSimulationStore } from "@/store/simulationStore";
import { HeroMetrics } from "@/components/report/HeroMetrics";
import { StrategyComparisonTable } from "@/components/report/StrategyComparisonTable";
import { WaterfallChart, CumulativeChart, StrategyDonut, DexTable, LiquidationAnalyticsView, LongtailAnalyticsView } from "@/components/report/Charts";
import { SectionCard } from "@/components/shared/SectionCard";
import { MetricCard } from "@/components/shared/MetricCard";
import { EmptyState } from "@/components/shared/EmptyState";
import { FileBarChart2, Download, FileJson, FileSpreadsheet, Play } from "lucide-react";
import { format } from "date-fns";
import { StrategyBadge } from "@/components/shared/StrategyBadge";
import type { StrategyId } from "@/lib/chains";
import { formatNative, formatUSD } from "@/lib/formatters";

export const Route = createFileRoute("/report")({
  head: () => ({ meta: [{ title: "Report — MEVSCOPE" }] }),
  component: ReportPage,
});

function ReportPage() {
  const summary = useSimulationStore((s) => s.results.summary);
  const chain = useSimulationStore((s) => s.selectedChain);
  const config = useSimulationStore((s) => s.config);
  const pipeline = useSimulationStore((s) => s.pipeline);
  const start = useSimulationStore((s) => s.startSimulation);
  const opps = useSimulationStore((s) => s.results.opportunities);
  const simulationMode = useSimulationStore((s) => s.simulationMode);
  const getExportJsonUrl = useSimulationStore((s) => s.getExportJsonUrl);
  const getExportCsvUrl = useSimulationStore((s) => s.getExportCsvUrl);

  if (!summary || !opps.length) {
    return (
      <div className="p-4">
        <SectionCard title="Report">
          <EmptyState icon={FileBarChart2} title="No report yet" body="Run a simulation to generate a report." action={<button onClick={() => void start()} className="inline-flex items-center gap-1.5 rounded-md bg-[var(--acc-green)] px-3 py-1.5 font-mono text-xs font-semibold text-black"><Play className="h-3 w-3" />Run simulation</button>} />
        </SectionCard>
      </div>
    );
  }

  const enabled = Object.keys(useSimulationStore.getState().config.strategies).filter((k) => useSimulationStore.getState().config.strategies[k as StrategyId].enabled) as StrategyId[];
  const hasLiq = simulationMode === "mock" && enabled.includes("liquidation");
  const hasLT = simulationMode === "mock" && enabled.includes("longtail");
  const fade = { initial: { opacity: 0, y: 8 }, animate: { opacity: 1, y: 0 } };

  return (
    <div className="p-4 space-y-4">
      <div className="flex flex-wrap items-center gap-3 rounded-xl border border-[var(--line)] bg-[var(--surface)] px-4 py-3">
        <div className="flex items-center gap-2">
          <span className="h-2 w-2 rounded-full" style={{ backgroundColor: chain.color, boxShadow: `0 0 10px ${chain.color}` }} />
          <span className="font-mono text-sm text-[var(--ink)]">{chain.name}</span>
        </div>
        <span className="font-mono text-xs text-[var(--ink-dim)]">Last {config.lastDays} days · {summary.total} opps · {format(pipeline.startedAt || Date.now(), "MMM d HH:mm")} · {Math.round((pipeline.elapsed || 0) / 1000)}s</span>
        <div className="ml-auto flex flex-wrap gap-1.5">
          {enabled.map((s) => <StrategyBadge key={s} id={s} />)}
        </div>
      </div>

      <motion.div {...fade} transition={{ duration: 0.3 }}><HeroMetrics /></motion.div>
      <motion.div {...fade} transition={{ duration: 0.3, delay: 0.05 }}><StrategyComparisonTable /></motion.div>
      <motion.div {...fade} transition={{ duration: 0.3, delay: 0.10 }}><WaterfallChart /></motion.div>
      <motion.div {...fade} transition={{ duration: 0.3, delay: 0.15 }} className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <CumulativeChart />
        <StrategyDonut />
      </motion.div>
      <motion.div {...fade} transition={{ duration: 0.3, delay: 0.20 }}><DexTable /></motion.div>
      {hasLiq && <motion.div {...fade} transition={{ duration: 0.3, delay: 0.25 }}><LiquidationAnalyticsView /></motion.div>}
      {hasLT && <motion.div {...fade} transition={{ duration: 0.3, delay: 0.30 }}><LongtailAnalyticsView /></motion.div>}

      <SectionCard title="Auto-resolved economics">
        <div className="grid grid-cols-2 md:grid-cols-3 gap-3">
          <MetricCard label="Gas spent" value={formatNative(summary.totalCost * 0.7, chain, 4)} accent="var(--acc-red)" />
          <MetricCard label="FL fees" value={formatNative(summary.totalCost * 0.1, chain, 5)} accent="var(--acc-purple)" />
          <MetricCard label="Builder tips" value={formatNative(summary.totalCost * 0.2, chain, 4)} accent="var(--acc-amber)" />
          <MetricCard label="Best single opp" value={formatNative(summary.bestSingleOpp, chain, 4)} accent="var(--acc-green)" />
          <MetricCard label="Auto gas price" value={`${chain.gasPriceGwei} gwei`} />
          <MetricCard label="Native price" value={formatUSD(chain.nativeUSD)} />
        </div>
      </SectionCard>

      <div className="flex flex-wrap gap-2 justify-end pt-2">
        {simulationMode === "api" && pipeline.runId ? (
          <>
            <a href={getExportJsonUrl(pipeline.runId)} download className={btnCls}><FileJson className="h-3 w-3" /> Export JSON</a>
            <a href={getExportCsvUrl(pipeline.runId)} download className={btnCls}><FileSpreadsheet className="h-3 w-3" /> Export CSV</a>
          </>
        ) : (
          <>
            <span className={btnCls}><FileJson className="h-3 w-3" /> Export JSON</span>
            <span className={btnCls}><FileSpreadsheet className="h-3 w-3" /> Export CSV</span>
          </>
        )}
        <span className={btnCls}><Download className="h-3 w-3" /> Export PDF</span>
        <Link to="/configure" className="inline-flex items-center gap-1.5 rounded-md bg-[var(--acc-green)] px-3 py-1.5 font-mono text-xs font-semibold text-black"><Play className="h-3 w-3" /> Run again →</Link>
      </div>
    </div>
  );
}

const btnCls = "inline-flex items-center gap-1.5 rounded-md border border-[var(--line)] px-3 py-1.5 font-mono text-xs text-[var(--ink-dim)] hover:text-[var(--ink)] hover:bg-[var(--panel)] no-underline";