import { cn } from "@/lib/utils";
import type { ReactNode } from "react";

export function MetricCard({ label, value, sub, accent, className, mono = true }: { label: string; value: ReactNode; sub?: ReactNode; accent?: string; className?: string; mono?: boolean }) {
  return (
    <div className={cn("rounded-xl border border-[var(--line)] bg-[var(--surface)] px-4 py-3.5", className)}>
      <div className="font-mono text-[10px] uppercase tracking-[0.18em] text-[var(--ink-dim)]">{label}</div>
      <div className={cn("mt-1.5 text-xl font-semibold tracking-tight text-[var(--ink)]", mono && "font-mono")} style={accent ? { color: accent } : undefined}>{value}</div>
      {sub && <div className="mt-1 font-mono text-[10px] text-[var(--ink-dim)]">{sub}</div>}
    </div>
  );
}