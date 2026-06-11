import { useSimulationStore } from "@/store/simulationStore";
import { SectionCard } from "../shared/SectionCard";
import { cn } from "@/lib/utils";

export function DexGrid() {
  const chain = useSimulationStore((s) => s.selectedChain);
  const dexes = useSimulationStore((s) => s.config.dexes);
  const toggle = useSimulationStore((s) => s.toggleDex);
  return (
    <SectionCard title="03 · DEX / fork filter" subtitle={`${chain.dexes.length} venues available on ${chain.name}`}>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
        {chain.dexes.map((d) => {
          const on = dexes.includes(d.id);
          return (
            <button
              key={d.id}
              onClick={() => toggle(d.id)}
              className={cn(
                "flex items-center justify-between rounded-md border bg-[var(--surface-2)] px-3 py-2 text-left transition-colors",
                on ? "border-[var(--acc-green)]/40" : "border-[var(--line)] opacity-60 hover:opacity-90",
              )}
            >
              <div className="min-w-0">
                <div className="flex items-center gap-2">
                  <span className="font-mono text-xs text-[var(--ink)]">{d.name}</span>
                  <span className="rounded-sm bg-[var(--panel)] px-1.5 py-0.5 font-mono text-[9px] uppercase tracking-[0.14em] text-[var(--ink-dim)]">{d.fork}</span>
                </div>
                <div className="mt-0.5 font-mono text-[10px] text-[var(--ink-mute)] truncate">{d.router}</div>
              </div>
              <span className={cn("h-1.5 w-1.5 rounded-full", on ? "bg-[var(--acc-green)]" : "bg-[var(--ink-mute)]")} />
            </button>
          );
        })}
      </div>
    </SectionCard>
  );
}