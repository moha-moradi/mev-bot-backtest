import { createFileRoute, Link } from "@tanstack/react-router";
import { useSimulationStore } from "@/store/simulationStore";
import { SectionCard } from "@/components/shared/SectionCard";
import { PipelineStep } from "@/components/pipeline/PipelineStep";
import { LiveLog } from "@/components/pipeline/LiveLog";
import { Square, ArrowRight, Play, GitBranch } from "lucide-react";
import { EmptyState } from "@/components/shared/EmptyState";

export const Route = createFileRoute("/pipeline")({
  head: () => ({ meta: [{ title: "Pipeline — MEVSCOPE" }] }),
  component: PipelinePage,
});

function PipelinePage() {
  const p = useSimulationStore((s) => s.pipeline);
  const cancel = useSimulationStore((s) => s.cancelSimulation);
  const start = useSimulationStore((s) => s.startSimulation);
  const chain = useSimulationStore((s) => s.selectedChain);

  if (p.status === "idle" && p.stages.length === 0) {
    return (
      <div className="p-4">
        <SectionCard title="Pipeline">
          <EmptyState
            icon={GitBranch}
            title="No simulation in progress"
            body="Configure a run and execute it to see the pipeline."
            action={
              <button onClick={() => void start()} className="inline-flex items-center gap-1.5 rounded-md bg-[var(--acc-green)] px-3 py-1.5 font-mono text-xs font-semibold text-black">
                <Play className="h-3 w-3" /> Start now
              </button>
            }
          />
        </SectionCard>
      </div>
    );
  }

  return (
    <div className="grid grid-cols-1 lg:grid-cols-[1.4fr_1fr] gap-4 p-4">
      <div className="space-y-4">
        <SectionCard
          title="Pipeline"
          subtitle={`${chain.name} · ${p.stages.filter(s => s.status === "done").length}/${p.stages.filter(s => s.status !== "skipped").length} stages complete`}
          action={
            <div className="flex items-center gap-3">
              <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.14em] text-[var(--ink-dim)]">
                <span>{Math.floor(p.elapsed / 1000)}s elapsed</span>
                <span>·</span>
                <span>ETA {Math.ceil(p.eta / 1000)}s</span>
              </div>
              {p.status === "running" ? (
                <button onClick={cancel} className="inline-flex items-center gap-1 rounded-md border border-[var(--acc-red)]/40 px-2 py-1 font-mono text-[10px] uppercase tracking-[0.14em] text-[var(--acc-red)] hover:bg-[var(--acc-red)]/10">
                  <Square className="h-3 w-3" /> Abort
                </button>
              ) : p.status === "done" ? (
                <Link to="/report" className="inline-flex items-center gap-1 rounded-md bg-[var(--acc-green)] px-2.5 py-1 font-mono text-[10px] uppercase tracking-[0.14em] text-black">
                  View report <ArrowRight className="h-3 w-3" />
                </Link>
              ) : null}
            </div>
          }
        >
          <div className="mb-4 h-1 w-full overflow-hidden rounded-full bg-[var(--surface-2)]">
            <div className="h-full bg-[var(--acc-green)] transition-all" style={{ width: `${p.progress}%` }} />
          </div>
          <div className="space-y-2">
            {p.stages.map((s, i) => <PipelineStep key={s.id} stage={s} index={i} />)}
          </div>
        </SectionCard>
      </div>
      <div>
        <LiveLog />
      </div>
    </div>
  );
}