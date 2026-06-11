import { truncHash } from "@/lib/formatters";
import { cn } from "@/lib/utils";

export function MonoHash({ hash, href, className }: { hash: string; href?: string; className?: string }) {
  const node = <span className={cn("font-mono text-xs text-[var(--ink)]", className)}>{truncHash(hash)}</span>;
  return href ? (
    <a href={href} target="_blank" rel="noopener noreferrer" className="hover:text-[var(--acc-green)] transition-colors">
      {node}
    </a>
  ) : node;
}