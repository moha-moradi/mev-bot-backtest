import { Link, useRouterState } from "@tanstack/react-router";
import { Play, Loader2 } from "lucide-react";
import { useSimulationStore } from "@/store/simulationStore";
import { StatusBadge } from "@/components/shared/StatusBadge";

const LABELS: Record<string, string> = {
  "/configure": "Configure", "/pipeline": "Pipeline", "/opportunities": "Opportunities",
  "/report": "Report", "/history": "History", "/": "Configure",
};

export function Header() {
  const pathname = useRouterState({ select: (s) => s.location.pathname });
  const chain = useSimulationStore((s) => s.selectedChain);
  const status = useSimulationStore((s) => s.pipeline.status);
  const start = useSimulationStore((s) => s.startSimulation);
  const simulationMode = useSimulationStore((s) => s.simulationMode);

  const page = LABELS[pathname] || "Configure";

  return (
    <header className="sticky top-0 z-20 flex h-12 items-center justify-between border-b border-[var(--line)] bg-[var(--page)]/80 backdrop-blur px-4">
      <div className="flex items-center gap-3 min-w-0">
        <div className="h-5 w-0.5" style={{ backgroundColor: chain.color }} />
        <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-[var(--ink-dim)]">{chain.name}</span>
        <span className="text-[var(--ink-mute)]">/</span>
        <span className="font-mono text-sm text-[var(--ink)] truncate">{page}</span>
      </div>
      <div className="flex items-center gap-3">
        <button
          onClick={() => useSimulationStore.getState().setSimulationMode(simulationMode === "mock" ? "api" : "mock")}
          className="rounded-sm px-1.5 py-0.5 font-mono text-[9px] uppercase tracking-[0.14em] border cursor-pointer"
          style={{
            backgroundColor: simulationMode === "api" ? "var(--acc-green)" : "var(--surface-2)",
            color: simulationMode === "api" ? "black" : "var(--ink-dim)",
            borderColor: simulationMode === "api" ? "var(--acc-green)" : "var(--line)",
          }}
          title="Toggle simulation mode"
        >
          {simulationMode}
        </button>
        <StatusBadge label={status === "running" ? "Running" : "Idle"} color={status === "running" ? "var(--acc-amber)" : "var(--acc-green)"} pulse={status === "running"} />
        <Link
          to="/pipeline"
          onClick={() => { if (status !== "running") void start(); }}
          className="inline-flex items-center gap-2 rounded-md bg-[var(--acc-green)] px-3 py-1.5 font-mono text-xs font-semibold text-black transition-transform hover:opacity-90 active:scale-[0.98]"
        >
          {status === "running" ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Play className="h-3.5 w-3.5" />}
          New simulation
        </Link>
      </div>
    </header>
  );
}