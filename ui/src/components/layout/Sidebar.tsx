import { Link, useRouterState } from "@tanstack/react-router";
import { Settings2, GitBranch, Target, FileBarChart2, History } from "lucide-react";
import { ChainSelector } from "./ChainSelector";
import { useSimulationStore } from "@/store/simulationStore";
import { cn } from "@/lib/utils";
import { StatusBadge } from "@/components/shared/StatusBadge";
import { formatUSD } from "@/lib/formatters";

const NAV = [
  { to: "/configure", label: "Configure", icon: Settings2 },
  { to: "/pipeline",  label: "Pipeline",  icon: GitBranch },
  { to: "/opportunities", label: "Opportunities", icon: Target },
  { to: "/report",    label: "Report",    icon: FileBarChart2 },
  { to: "/history",   label: "History",   icon: History },
] as const;

export function Sidebar() {
  const pathname = useRouterState({ select: (s) => s.location.pathname });
  const chain = useSimulationStore((s) => s.selectedChain);

  return (
    <aside className="hidden md:flex flex-col w-[240px] shrink-0 border-r border-[var(--line)] bg-[var(--panel)]">
      <div className="px-4 py-4 border-b border-[var(--line)]">
        <div className="flex items-center gap-2 mb-3">
          <div className="font-mono text-base font-semibold tracking-tight text-[var(--acc-green)]">MEVSCOPE</div>
          <span className="ml-auto rounded-sm px-1.5 py-0.5 font-mono text-[9px] uppercase tracking-[0.18em]" style={{ backgroundColor: `${chain.color}20`, color: chain.color, boxShadow: `inset 0 0 0 1px ${chain.color}40` }}>{chain.name}</span>
        </div>
        <ChainSelector />
      </div>
      <nav className="flex-1 px-2 py-3 space-y-0.5">
        {NAV.map((item) => {
          const active = pathname.startsWith(item.to);
          return (
            <Link
              key={item.to}
              to={item.to}
              className={cn(
                "flex items-center gap-2.5 rounded-md px-2.5 py-2 text-sm transition-colors",
                active
                  ? "bg-[var(--surface)] text-[var(--ink)]"
                  : "text-[var(--ink-dim)] hover:text-[var(--ink)] hover:bg-[var(--surface)]/50",
              )}
              style={active ? { boxShadow: `inset 2px 0 0 ${chain.color}` } : undefined}
            >
              <item.icon className="h-4 w-4" />
              <span className="font-mono text-[13px]">{item.label}</span>
            </Link>
          );
        })}
      </nav>
      <div className="border-t border-[var(--line)] px-4 py-3 space-y-2">
        <div className="flex items-center justify-between">
          <StatusBadge label="Block sync" color={chain.color} pulse />
          <span className="font-mono text-[10px] text-[var(--ink-dim)]">19,842,301</span>
        </div>
        <div className="flex items-center justify-between">
          <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-[var(--ink-dim)]">{chain.nativeToken}</span>
          <span className="font-mono text-[10px] text-[var(--ink)]">{formatUSD(chain.nativeUSD)}</span>
        </div>
        <div className="flex items-center justify-between">
          <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-[var(--ink-dim)]">RPC</span>
          <span className="font-mono text-[10px] text-[var(--acc-green)]">42ms</span>
        </div>
      </div>
    </aside>
  );
}