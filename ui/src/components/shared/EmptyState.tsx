import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

export function EmptyState({ icon: Icon, title, body, action }: { icon: LucideIcon; title: string; body?: string; action?: ReactNode }) {
  return (
    <div className="flex flex-col items-center justify-center gap-3 py-16 text-center">
      <Icon className="h-8 w-8 text-[var(--ink-mute)]" />
      <div className="font-mono text-sm text-[var(--ink)]">{title}</div>
      {body && <p className="max-w-sm text-xs text-[var(--ink-dim)]">{body}</p>}
      {action}
    </div>
  );
}