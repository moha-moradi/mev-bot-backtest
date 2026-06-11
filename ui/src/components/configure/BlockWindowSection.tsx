import { useSimulationStore } from "@/store/simulationStore";
import { SectionCard } from "@/components/shared/SectionCard";
import { cn } from "@/lib/utils";
import { CheckCircle2 } from "lucide-react";
import { format } from "date-fns";

const MODES = [
  { id: "days", label: "Last N days" },
  { id: "range", label: "Block range" },
  { id: "block", label: "Single block" },
] as const;

export function BlockWindowSection() {
  const config = useSimulationStore((s) => s.config);
  const chain = useSimulationStore((s) => s.selectedChain);
  const setWindowMode = useSimulationStore((s) => s.setWindowMode);
  const setLastDays = useSimulationStore((s) => s.setLastDays);
  const setRpc = useSimulationStore((s) => s.setRpc);

  const estBlocks = Math.round((config.lastDays * 86400) / chain.blockTime);
  const estTx = estBlocks * chain.avgTxPerBlock;
  const today = new Date();
  const from = new Date(Date.now() - config.lastDays * 86400 * 1000);

  return (
    <SectionCard title="01 · Block window" accent={chain.color}>
      <div className="inline-flex rounded-md border border-[var(--line)] bg-[var(--surface-2)] p-0.5 mb-4">
        {MODES.map((m) => (
          <button
            key={m.id}
            onClick={() => setWindowMode(m.id)}
            className={cn(
              "px-3 py-1.5 font-mono text-[11px] uppercase tracking-[0.14em] rounded-sm transition-colors",
              config.windowMode === m.id ? "bg-[var(--panel)] text-[var(--ink)]" : "text-[var(--ink-dim)] hover:text-[var(--ink)]",
            )}
          >
            {m.label}
          </button>
        ))}
      </div>

      {config.windowMode === "days" && (
        <div className="space-y-4">
          <div>
            <label className="font-mono text-[10px] uppercase tracking-[0.18em] text-[var(--ink-dim)]">Days</label>
            <input
              type="number" min={1} max={90} value={config.lastDays}
              onChange={(e) => setLastDays(parseInt(e.target.value || "30"))}
              className="mt-1 w-32 rounded-md border border-[var(--line)] bg-[var(--surface-2)] px-3 py-2 font-mono text-sm focus:border-[var(--acc-green)]/50 focus:outline-none"
            />
          </div>
          <div className="grid grid-cols-3 gap-3 rounded-md border border-[var(--line)] bg-[var(--surface-2)] p-3">
            <Stat label="Est. blocks" value={estBlocks.toLocaleString()} />
            <Stat label="Est. transactions" value={estTx.toLocaleString()} />
            <Stat label="Date range" value={`${format(from, "MMM d")} → ${format(today, "MMM d")}`} />
          </div>
        </div>
      )}

      {config.windowMode === "range" && (
        <div className="grid grid-cols-2 gap-3">
          <Input label="From block" placeholder="19,800,000" />
          <Input label="To block" placeholder="19,842,000" />
          <button className="col-span-2 rounded-md border border-[var(--line)] px-3 py-2 font-mono text-xs text-[var(--ink)] hover:bg-[var(--panel)]">Fetch range info</button>
        </div>
      )}

      {config.windowMode === "block" && (
        <Input label="Block number or hash" placeholder="19,842,301 or 0xabc…" />
      )}

      <div className="mt-5 border-t border-[var(--line)] pt-4">
        <label className="font-mono text-[10px] uppercase tracking-[0.18em] text-[var(--ink-dim)]">RPC endpoint</label>
        <div className="mt-1 flex gap-2">
          <input
            value={config.rpc}
            onChange={(e) => setRpc(e.target.value)}
            className="flex-1 rounded-md border border-[var(--line)] bg-[var(--surface-2)] px-3 py-2 font-mono text-xs focus:border-[var(--acc-green)]/50 focus:outline-none"
          />
          <button className="inline-flex items-center gap-1.5 rounded-md border border-[var(--line)] px-3 py-2 font-mono text-xs text-[var(--ink)] hover:bg-[var(--panel)]">
            Test
            <span className="inline-flex items-center gap-1 text-[var(--acc-green)]">
              <CheckCircle2 className="h-3 w-3" /> 42ms
            </span>
          </button>
        </div>
      </div>
    </SectionCard>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <div className="font-mono text-[9px] uppercase tracking-[0.18em] text-[var(--ink-dim)]">{label}</div>
      <div className="mt-1 font-mono text-sm text-[var(--ink)]">{value}</div>
    </div>
  );
}

function Input({ label, placeholder }: { label: string; placeholder?: string }) {
  return (
    <div>
      <label className="font-mono text-[10px] uppercase tracking-[0.18em] text-[var(--ink-dim)]">{label}</label>
      <input placeholder={placeholder} className="mt-1 w-full rounded-md border border-[var(--line)] bg-[var(--surface-2)] px-3 py-2 font-mono text-sm focus:border-[var(--acc-green)]/50 focus:outline-none" />
    </div>
  );
}