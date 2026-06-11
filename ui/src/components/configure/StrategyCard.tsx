import { useState, type ReactNode } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { ChevronDown, FileSearch2 } from "lucide-react";
import { Switch } from "@/components/ui/switch";
import { useSimulationStore } from "@/store/simulationStore";
import type { StrategyDef } from "@/lib/strategyDefs";
import { WarningBanner } from "@/components/shared/WarningBanner";
import { cn } from "@/lib/utils";

export function StrategyCard({ def, children, onShowTrace }: { def: StrategyDef; children?: ReactNode; onShowTrace: () => void }) {
  const enabled = useSimulationStore((s) => s.config.strategies[def.id].enabled);
  const toggle = useSimulationStore((s) => s.toggleStrategy);
  const [open, setOpen] = useState(false);
  const Icon = def.icon;

  return (
    <div
      className={cn("rounded-xl border bg-[var(--surface)] overflow-hidden transition-opacity", !enabled && "opacity-60")}
      style={{ borderColor: "var(--line)", borderLeft: `2px solid ${def.color}` }}
    >
      <div className="p-4">
        <div className="flex items-start gap-3">
          <div className="flex h-9 w-9 items-center justify-center rounded-md" style={{ backgroundColor: `${def.color}14`, color: def.color, boxShadow: `inset 0 0 0 1px ${def.color}30` }}>
            <Icon className="h-4 w-4" />
          </div>
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2">
              <h3 className="font-mono text-sm text-[var(--ink)]">{def.name}</h3>
              <span className="font-mono text-[9px] uppercase tracking-[0.18em]" style={{ color: def.color }}>{def.short}</span>
            </div>
            <p className="mt-0.5 text-xs text-[var(--ink-dim)] leading-relaxed">{def.description}</p>
          </div>
          <Switch checked={enabled} onCheckedChange={(v) => toggle(def.id, v)} />
        </div>

        {def.warning && enabled && <div className="mt-3"><WarningBanner>{def.warning}</WarningBanner></div>}

        <div className="mt-3 flex items-center gap-2">
          <button
            onClick={onShowTrace}
            className="inline-flex items-center gap-1.5 rounded-sm border border-[var(--line)] px-2 py-1 font-mono text-[10px] uppercase tracking-[0.14em] text-[var(--ink-dim)] hover:text-[var(--ink)] hover:bg-[var(--panel)]"
          >
            <FileSearch2 className="h-3 w-3" /> Expected trace
          </button>
          {enabled && children && (
            <button
              onClick={() => setOpen(!open)}
              className="ml-auto inline-flex items-center gap-1 font-mono text-[10px] uppercase tracking-[0.14em] text-[var(--ink-dim)] hover:text-[var(--ink)]"
            >
              {open ? "Hide" : "Advanced"}
              <ChevronDown className={cn("h-3 w-3 transition-transform", open && "rotate-180")} />
            </button>
          )}
        </div>
      </div>

      <AnimatePresence initial={false}>
        {open && enabled && children && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.2, ease: "easeInOut" }}
            className="overflow-hidden border-t border-[var(--line)]"
          >
            <div className="p-4 bg-[var(--surface-2)]/40">{children}</div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}