import { AlertTriangle } from "lucide-react";

export function WarningBanner({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex gap-2 rounded-md border border-[var(--acc-amber)]/30 bg-[var(--acc-amber)]/5 px-3 py-2 text-xs text-[var(--acc-amber)]">
      <AlertTriangle className="h-3.5 w-3.5 mt-0.5 shrink-0" />
      <div className="leading-relaxed">{children}</div>
    </div>
  );
}