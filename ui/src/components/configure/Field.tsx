import { cn } from "@/lib/utils";
import type { ReactNode } from "react";

export function Field({ label, sub, children, className }: { label: string; sub?: string; children: ReactNode; className?: string }) {
  return (
    <div className={cn("space-y-1", className)}>
      <label className="font-mono text-[10px] uppercase tracking-[0.18em] text-[var(--ink-dim)]">{label}</label>
      {children}
      {sub && <p className="text-[10px] text-[var(--ink-mute)]">{sub}</p>}
    </div>
  );
}

export function NumInput(props: React.InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      type="number"
      {...props}
      className={cn("w-full rounded-md border border-[var(--line)] bg-[var(--surface-2)] px-2.5 py-1.5 font-mono text-xs focus:border-[var(--acc-green)]/50 focus:outline-none", props.className)}
    />
  );
}

export function Segmented<T extends string>({ options, value, onChange }: { options: { value: T; label: string }[]; value: T; onChange: (v: T) => void }) {
  return (
    <div className="inline-flex rounded-md border border-[var(--line)] bg-[var(--surface-2)] p-0.5">
      {options.map((o) => (
        <button key={o.value} onClick={() => onChange(o.value)} className={cn("px-2.5 py-1 font-mono text-[11px] rounded-sm", value === o.value ? "bg-[var(--panel)] text-[var(--ink)]" : "text-[var(--ink-dim)]")}>
          {o.label}
        </button>
      ))}
    </div>
  );
}

export function Chips<T extends string>({ options, value, onToggle }: { options: T[]; value: T[]; onToggle: (v: T) => void }) {
  return (
    <div className="flex flex-wrap gap-1.5">
      {options.map((o) => {
        const on = value.includes(o);
        return (
          <button
            key={o}
            onClick={() => onToggle(o)}
            className={cn(
              "rounded-sm px-2 py-1 font-mono text-[10px] uppercase tracking-[0.14em] transition-colors",
              on ? "bg-[var(--acc-green)]/15 text-[var(--acc-green)]" : "bg-[var(--surface-2)] text-[var(--ink-dim)] hover:text-[var(--ink)]",
            )}
            style={on ? { boxShadow: "inset 0 0 0 1px var(--acc-green)" } : { boxShadow: "inset 0 0 0 1px var(--line)" }}
          >
            {o}
          </button>
        );
      })}
    </div>
  );
}