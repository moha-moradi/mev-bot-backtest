import { cn } from "@/lib/utils";
import type { ReactNode } from "react";

export function SectionCard({
  title, subtitle, action, children, className, accent,
}: { title?: string; subtitle?: string; action?: ReactNode; children: ReactNode; className?: string; accent?: string }) {
  return (
    <section
      className={cn("rounded-xl border border-[var(--line)] bg-[var(--surface)] overflow-hidden", className)}
      style={accent ? { borderLeft: `2px solid ${accent}` } : undefined}
    >
      {(title || action) && (
        <header className="flex items-center justify-between border-b border-[var(--line)] px-5 py-3">
          <div>
            {title && <h2 className="font-mono text-[11px] uppercase tracking-[0.18em] text-[var(--ink)]">{title}</h2>}
            {subtitle && <p className="mt-0.5 text-xs text-[var(--ink-dim)]">{subtitle}</p>}
          </div>
          {action}
        </header>
      )}
      <div className="p-5">{children}</div>
    </section>
  );
}