import { useState } from "react";
import { ChevronDown, Check } from "lucide-react";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { CHAINS } from "@/lib/chains";
import { useSimulationStore } from "@/store/simulationStore";
import { ChainDot } from "@/components/shared/ChainDot";
import { cn } from "@/lib/utils";

export function ChainSelector({ compact }: { compact?: boolean }) {
  const chain = useSimulationStore((s) => s.selectedChain);
  const setChain = useSimulationStore((s) => s.setChain);
  const running = useSimulationStore((s) => s.pipeline.status === "running");
  const [open, setOpen] = useState(false);
  const [pending, setPending] = useState<string | null>(null);

  const confirm = () => {
    if (pending) { setChain(pending); setPending(null); }
  };

  return (
    <>
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <button
            className={cn(
              "w-full flex items-center justify-between gap-2 rounded-md border border-[var(--line)] bg-[var(--surface-2)] px-3 py-2 text-left transition-colors hover:border-[var(--ink-mute)]",
              compact && "justify-center px-2",
            )}
          >
            <span className="flex items-center gap-2 min-w-0">
              <ChainDot color={chain.color} />
              {!compact && (
                <span className="min-w-0">
                  <span className="block font-mono text-[10px] uppercase tracking-[0.18em] text-[var(--ink-dim)]">Chain</span>
                  <span className="block truncate font-mono text-sm text-[var(--ink)]">{chain.name} · {chain.nativeToken}</span>
                </span>
              )}
            </span>
            {!compact && <ChevronDown className="h-3.5 w-3.5 text-[var(--ink-dim)]" />}
          </button>
        </PopoverTrigger>
        <PopoverContent align="start" className="w-72 p-1 bg-[var(--surface)] border-[var(--line)]">
          <div className="px-2 py-1.5 font-mono text-[10px] uppercase tracking-[0.18em] text-[var(--ink-dim)]">Switch chain</div>
          <ul className="max-h-80 overflow-auto">
            {CHAINS.map((c) => {
              const active = c.id === chain.id;
              return (
                <li key={c.id}>
                  <button
                    onClick={() => {
                      if (active) { setOpen(false); return; }
                      if (running) setPending(c.id);
                      else setChain(c.id);
                      setOpen(false);
                    }}
                    className={cn(
                      "w-full flex items-center justify-between gap-3 rounded-sm px-2 py-2 text-left text-sm hover:bg-[var(--panel)]",
                      active && "bg-[var(--panel)]",
                    )}
                  >
                    <span className="flex items-center gap-2">
                      <ChainDot color={c.color} />
                      <span className="font-mono text-[var(--ink)]">{c.name}</span>
                      <span className="font-mono text-xs text-[var(--ink-dim)]">{c.nativeToken}</span>
                    </span>
                    {active && <Check className="h-3.5 w-3.5 text-[var(--acc-green)]" />}
                  </button>
                </li>
              );
            })}
          </ul>
        </PopoverContent>
      </Popover>
      {pending && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" onClick={() => setPending(null)}>
          <div className="rounded-xl border border-[var(--line)] bg-[var(--surface)] p-6 max-w-sm" onClick={(e) => e.stopPropagation()}>
            <h3 className="font-mono text-sm uppercase tracking-[0.18em]">Reset simulation?</h3>
            <p className="mt-2 text-xs text-[var(--ink-dim)]">A simulation is running. Switching chain will cancel it and reset your DEX list and parameters.</p>
            <div className="mt-4 flex gap-2 justify-end">
              <button onClick={() => setPending(null)} className="rounded-md border border-[var(--line)] px-3 py-1.5 text-xs hover:bg-[var(--panel)]">Cancel</button>
              <button onClick={confirm} className="rounded-md bg-[var(--acc-green)] px-3 py-1.5 text-xs font-mono font-semibold text-black hover:opacity-90">Confirm</button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}