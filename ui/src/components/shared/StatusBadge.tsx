import { cn } from "@/lib/utils";

export function StatusBadge({ label, color = "var(--acc-green)", pulse, className }: { label: string; color?: string; pulse?: boolean; className?: string }) {
  return (
    <span className={cn("inline-flex items-center gap-1.5 font-mono text-[10px] uppercase tracking-[0.18em] text-[var(--ink)]", className)}>
      <span
        className={cn("h-1.5 w-1.5 rounded-full", pulse && "pulse-dot")}
        style={{ backgroundColor: color, color }}
      />
      {label}
    </span>
  );
}