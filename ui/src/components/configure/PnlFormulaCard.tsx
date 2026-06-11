export function PnlFormulaCard({ title, lines, net }: { title: string; lines: { label: string; value: string; sub?: string }[]; net: string }) {
  return (
    <div className="rounded-md border border-[var(--line)] bg-[var(--surface-2)] p-3">
      <div className="font-mono text-[10px] uppercase tracking-[0.18em] text-[var(--ink-dim)]">{title}</div>
      <div className="mt-2 space-y-1.5">
        {lines.map((l) => (
          <div key={l.label} className="flex items-baseline justify-between gap-2 font-mono text-xs">
            <span className="text-[var(--ink-dim)]">{l.label}</span>
            <span className="text-[var(--ink)]">{l.value}</span>
          </div>
        ))}
        <div className="mt-2 pt-2 border-t border-[var(--line)] flex items-baseline justify-between font-mono text-xs">
          <span className="text-[var(--ink-dim)]">Net profit</span>
          <span className="text-[var(--acc-green)] font-semibold">{net}</span>
        </div>
      </div>
    </div>
  );
}