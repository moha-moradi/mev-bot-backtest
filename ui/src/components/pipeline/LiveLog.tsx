import { useSimulationStore } from "@/store/simulationStore";
import { TerminalPanel } from "../shared/TerminalPanel";
import { useEffect, useRef } from "react";

const TAG_COLOR: Record<string, string> = {
  SCAN: "var(--acc-blue)", LIQ: "var(--acc-pink)", LONGTAIL: "var(--acc-cyan)",
  DONE: "var(--acc-green)", SKIP: "var(--ink-mute)", FLASH: "var(--acc-purple)",
  RPC: "var(--ink)", FILTER: "var(--ink)", REPLAY: "var(--ink)", PROFIT: "var(--ink)", AGG: "var(--ink)",
};

export function LiveLog() {
  const logs = useSimulationStore((s) => s.pipeline.logs);
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => { ref.current?.scrollTo({ top: ref.current.scrollHeight }); }, [logs.length]);
  return (
    <TerminalPanel title="Live log" height="calc(100vh - 12rem)">
      <div ref={ref} className="h-full overflow-auto">
        {logs.length === 0 ? (
          <div className="text-[var(--ink-dim)]">Waiting for simulation to start…<span className="cursor-blink" /></div>
        ) : logs.map((l, i) => (
          <div key={i} className="flex gap-2">
            <span className="text-[var(--ink-mute)]">[{l.ts}]</span>
            <span style={{ color: TAG_COLOR[l.tag] || "var(--ink)" }} className="font-semibold">[{l.tag}]</span>
            <span className={l.color === "ok" ? "text-[var(--acc-green)]" : l.color === "muted" ? "text-[var(--ink-mute)]" : "text-[var(--ink)]"}>{l.text}</span>
          </div>
        ))}
      </div>
    </TerminalPanel>
  );
}