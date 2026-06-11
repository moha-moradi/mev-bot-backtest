export function ChainDot({ color, size = 8 }: { color: string; size?: number }) {
  return <span className="inline-block rounded-full" style={{ width: size, height: size, backgroundColor: color, boxShadow: `0 0 8px ${color}80` }} />;
}