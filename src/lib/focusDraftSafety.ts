/** Return true only for the exact snapshot successfully persisted by SQLite. */
export function shouldClearFocusDraft(savedRaw: string | null, currentRaw: string | null): boolean {
  return savedRaw !== null && currentRaw === savedRaw;
}
