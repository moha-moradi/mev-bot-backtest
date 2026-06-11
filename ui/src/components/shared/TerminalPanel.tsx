import { cn } from "@/lib/utils";
import type { ReactNode } from "react";

export function TerminalPanel({ title, action, children, className, height }: { title?: string; action?: ReactNode; children: ReactNode; className?: string; height?: string }) {
  return (
    <div className={cn("rounded-xl border border-[var(--line)] overflow-hidden", className)} style={{ backgroundColor: "#080b0f" }}>
      {(title || action) && (
        <div className="flex items-center justify-between border-b border-[var(--line)] px-4 py-2">
          <div className="flex items-center gap-2">
            <div className="flex gap-1.5">
              <span className="h-2.5 w-2.5 rounded-full bg-[#3b3f47]" />
              <span className="h-2.5 w-2.5 rounded-full bg-[#3b3f47]" />
              <span className="h-2.5 w-2.5 rounded-full bg-[#3b3f47]" />
            </div>
            {title && <span className="ml-2 font-mono text-[10px] uppercase tracking-[0.18em] text-[var(--ink-dim)]">{title}</span>}
          </div>
          {action}
        </div>
      )}
      <div className="p-4 font-mono text-xs leading-relaxed overflow-auto" style={{ height }}>
        {children}
      </div>
    </div>
  );
}