import { useSimulationStore } from "@/store/simulationStore";
import { TerminalPanel } from "../shared/TerminalPanel";
import { STRATEGY_BY_ID } from "@/lib/strategyDefs";
import type { StrategyId } from "@/lib/chains";
import { Copy } from "lucide-react";
import { useState } from "react";

export function CliPreview() {
  const chain = useSimulationStore((s) => s.selectedChain);
  const config = useSimulationStore((s) => s.config);
  const simulationMode = useSimulationStore((s) => s.simulationMode);
  const enabled = (Object.keys(config.strategies) as StrategyId[]).filter((k) => config.strategies[k].enabled);
  const dexes = config.dexes;
  const liq = config.strategies.liquidation;
  const [copied, setCopied] = useState(false);

  const Flag = ({ k, v, vColor }: { k: string; v: string; vColor?: string }) => (
    <div className="whitespace-pre-wrap">
      <span className="text-[var(--acc-blue)]">  --{k} </span>
      <span style={{ color: vColor || "var(--acc-green)" }}>{v}</span>
      <span className="text-[var(--ink-dim)]"> \</span>
    </div>
  );

  const cliText = [
    "mevscope simulate \\",
    `  --chain ${chain.id} \\`,
    config.windowMode === "days" ? `  --last-days ${config.lastDays} \\` : config.windowMode === "range" ? `  --from-block ${config.fromBlock} --to-block ${config.toBlock} \\` : `  --block ${config.singleBlock} \\`,
    `  --strategies ${enabled.join(",")} \\`,
    `  --dexes ${dexes.join(",")} \\`,
    liq.enabled ? `  --lending ${liq.protocol} \\` : null,
    `  --flash-loan ${config.flashLoanProvider} \\`,
    `  --rpc ${config.rpc} \\`,
    `  --workers auto`,
  ].filter(Boolean).join("\n");

  const apiStrategies = enabled.filter((s) => ["arb", "jit", "jitarb", "sandwich"].includes(s));
  const windowConfig = config.windowMode === "days" ? { mode: "days", last_days: config.lastDays }
    : config.windowMode === "range" ? { mode: "range", from_block: config.fromBlock, to_block: config.toBlock }
    : config.windowMode === "block" ? { mode: "single", single_block: config.singleBlock ? Number(config.singleBlock) : undefined }
    : { mode: "blocks", last_days: 100 };
  const reqBody = JSON.stringify({
    chain: chain.id,
    rpc_url: config.rpc || undefined,
    window: windowConfig,
    strategies: apiStrategies,
    flash_loan_provider: config.flashLoanProvider,
  }, null, 2);
  const previewTitle = simulationMode === "api" ? "API request" : "CLI preview";
  const copy = () => {
    navigator.clipboard.writeText(simulationMode === "api" ? reqBody : cliText);
    setCopied(true);
    setTimeout(() => setCopied(false), 1200);
  };

  if (simulationMode === "api") {
    return (
      <TerminalPanel
        title={previewTitle}
        action={<button onClick={copy} className="inline-flex items-center gap-1 font-mono text-[10px] uppercase tracking-[0.14em] text-[var(--ink-dim)] hover:text-[var(--ink)]"><Copy className="h-3 w-3" /> {copied ? "Copied" : "Copy"}</button>}
      >
        <pre className="text-[10px] leading-relaxed text-[var(--ink)] overflow-x-auto">{reqBody}</pre>
      </TerminalPanel>
    );
  }

  return (
    <TerminalPanel
      title={previewTitle}
      action={<button onClick={copy} className="inline-flex items-center gap-1 font-mono text-[10px] uppercase tracking-[0.14em] text-[var(--ink-dim)] hover:text-[var(--ink)]"><Copy className="h-3 w-3" /> {copied ? "Copied" : "Copy"}</button>}
    >
      <div className="text-[var(--ink)]">mevscope simulate <span className="text-[var(--ink-dim)]">\</span></div>
      <Flag k="chain" v={chain.id} vColor={chain.color} />
      {config.windowMode === "days" && <Flag k="last-days" v={String(config.lastDays)} />}
      {config.windowMode === "range" && <Flag k="from-block" v={`${config.fromBlock ?? "—"} --to-block ${config.toBlock ?? "—"}`} />}
      {config.windowMode === "block" && <Flag k="block" v={config.singleBlock ?? "—"} />}
      <div className="whitespace-pre-wrap">
        <span className="text-[var(--acc-blue)]">  --strategies </span>
        {enabled.map((s, i) => (
          <span key={s} style={{ color: STRATEGY_BY_ID[s].color }}>{s}{i < enabled.length - 1 ? "," : ""}</span>
        ))}
        <span className="text-[var(--ink-dim)]"> \</span>
      </div>
      <Flag k="dexes" v={dexes.join(",")} />
      {liq.enabled && <Flag k="lending" v={liq.protocol} vColor="var(--acc-pink)" />}
      <Flag k="flash-loan" v={config.flashLoanProvider} vColor="var(--acc-purple)" />
      <Flag k="rpc" v={config.rpc} />
      <div className="whitespace-pre-wrap cursor-blink">
        <span className="text-[var(--acc-blue)]">  --workers </span>
        <span className="text-[var(--acc-green)]">auto</span>
      </div>
    </TerminalPanel>
  );
}