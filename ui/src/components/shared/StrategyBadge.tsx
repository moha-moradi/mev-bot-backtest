import { STRATEGY_BY_ID } from "@/lib/strategyDefs";
import type { StrategyId } from "@/lib/chains";
import { cn } from "@/lib/utils";

export function StrategyBadge({ id, className, size = "sm" }: { id: StrategyId; className?: string; size?: "xs" | "sm" }) {
  const def = STRATEGY_BY_ID[id];
  const padding = size === "xs" ? "px-1.5 py-0.5 text-[9px]" : "px-2 py-0.5 text-[10px]";
  return (
    <span
      className={cn("inline-flex items-center gap-1 rounded-sm font-mono uppercase tracking-[0.14em]", padding, className)}
      style={{
        color: def.color,
        backgroundColor: `${def.color}14`,
        boxShadow: `inset 0 0 0 1px ${def.color}30`,
      }}
    >
      <span className="h-1 w-1 rounded-full" style={{ backgroundColor: def.color }} />
      {def.short}
    </span>
  );
}