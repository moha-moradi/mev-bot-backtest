import { motion } from "framer-motion";
import { Check, Loader2, MinusCircle, Circle } from "lucide-react";
import type { PipelineStage } from "@/store/simulationStore";
import { cn } from "@/lib/utils";

export function PipelineStep({ stage, index }: { stage: PipelineStage; index: number }) {
  const Icon = stage.status === "done" ? Check : stage.status === "running" ? Loader2 : stage.status === "skipped" ? MinusCircle : Circle;
  const color = stage.status === "done" ? "var(--acc-green)" : stage.status === "running" ? "var(--acc-amber)" : stage.status === "skipped" ? "var(--ink-mute)" : "var(--ink-mute)";
  return (
    <motion.div
      initial={{ opacity: 0, x: -8 }}
      animate={{ opacity: 1, x: 0 }}
      transition={{ delay: index * 0.04 }}
      className={cn(
        "flex items-start gap-3 rounded-md border bg-[var(--surface)] p-3.5",
        stage.status === "skipped" ? "border-[var(--line)] opacity-50" : "border-[var(--line)]",
        stage.status === "running" && "ring-1 ring-[var(--acc-amber)]/30",
      )}
    >
      <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md border" style={{ borderColor: `${color}40`, color }}>
        <Icon className={cn("h-3.5 w-3.5", stage.status === "running" && "animate-spin")} />
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center justify-between gap-2">
          <span className="font-mono text-xs text-[var(--ink)]">{String(index + 1).padStart(2, "0")} · {stage.label}</span>
          {stage.status === "skipped" && <span className="font-mono text-[9px] uppercase tracking-[0.18em] text-[var(--ink-mute)]">— skipped</span>}
          {stage.status === "done" && <span className="font-mono text-[10px] text-[var(--acc-green)]">{stage.finishedAt && stage.startedAt ? `${stage.finishedAt - stage.startedAt}ms` : ""}</span>}
        </div>
        <div className="mt-0.5 text-xs text-[var(--ink-dim)]">{stage.sub}</div>
      </div>
    </motion.div>
  );
}