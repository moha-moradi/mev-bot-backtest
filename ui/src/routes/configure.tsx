import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { BlockWindowSection } from "@/components/configure/BlockWindowSection";
import { StrategyGrid } from "@/components/configure/StrategyGrid";
import { DexGrid } from "@/components/configure/DexGrid";
import { AutoEconomics } from "@/components/configure/AutoEconomics";
import { CliPreview } from "@/components/configure/CliPreview";
import { useSimulationStore } from "@/store/simulationStore";
import { Play } from "lucide-react";

export const Route = createFileRoute("/configure")({
  head: () => ({ meta: [{ title: "Configure — MEVSCOPE" }] }),
  component: ConfigurePage,
});

function ConfigurePage() {
  const navigate = useNavigate();
  const start = useSimulationStore((s) => s.startSimulation);

  const run = () => { void start(); navigate({ to: "/pipeline" }); };

  return (
    <div className="grid grid-cols-1 lg:grid-cols-[1.6fr_1fr] gap-4 p-4">
      <div className="space-y-4">
        <BlockWindowSection />
        <StrategyGrid />
        <DexGrid />
        <AutoEconomics />
      </div>
      <div className="space-y-4 lg:sticky lg:top-16 lg:self-start">
        <CliPreview />
        <button
          onClick={run}
          className="w-full inline-flex items-center justify-center gap-2 rounded-md bg-[var(--acc-green)] px-4 py-3 font-mono text-sm font-semibold text-black transition-transform hover:opacity-90 active:scale-[0.99]"
        >
          <Play className="h-4 w-4" /> Run simulation →
        </button>
      </div>
    </div>
  );
}