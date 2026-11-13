/** Timer display is a hint. SQLite remains authoritative for logged time. */
export function effectiveElapsedMs(activeMs: number, lastTickAt: number, now: number, active: boolean): number {
  const saved = Number.isFinite(activeMs) ? Math.max(0, activeMs) : 0;
  if (!active || !Number.isFinite(lastTickAt) || !Number.isFinite(now)) return saved;
  return saved + Math.min(20_000, Math.max(0, now - lastTickAt));
}
export function formatElapsed(ms: number): string {
  const total = Math.max(0, Math.floor((Number.isFinite(ms) ? ms : 0) / 1000));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor(total % 3600 / 60);
  const seconds = total % 60;
  const p = (n: number) => String(n).padStart(2, '0');
  return hours ? `${p(hours)}:${p(minutes)}:${p(seconds)}` : `${p(minutes)}:${p(seconds)}`;
}
