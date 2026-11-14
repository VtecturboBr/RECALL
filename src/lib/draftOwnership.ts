/**
 * Protect local recovery caches from two editor WebViews overwriting/deleting
 * each other's snapshots. SQLite CAS is still the authoritative save barrier.
 */
const EDITOR_OWNER: string = (() => {
  try {
    const key = 'recall-editor-owner-v1';
    let id = sessionStorage.getItem(key);
    if (!id) {
      id = crypto.randomUUID();
      sessionStorage.setItem(key, id);
    }
    return id;
  } catch { return 'temporary-' + Math.random().toString(36).slice(2); }
})();

export function saveOwnedDraft(key: string, payload: Record<string, unknown>): void {
  const previous = localStorage.getItem(key);
  if (previous) {
    try {
      const snapshot = JSON.parse(previous) as { __owner?: string };
      if (snapshot.__owner !== EDITOR_OWNER && previous !== JSON.stringify(payload)) {
        const previousOwner = snapshot.__owner || 'legacy';
        // One retained recovery copy per competing owner per document.
        const recoveryKey = 'recall-draft-recovery:' + key + ':' + previousOwner;
        localStorage.setItem(recoveryKey, previous);
      }
    } catch {
      // Preserve malformed cache as a separate non-destructive recovery copy.
      localStorage.setItem('recall-draft-recovery:' + key + ':unreadable', previous);
    }
  }
  localStorage.setItem(key, JSON.stringify({ ...payload, __owner: EDITOR_OWNER, __updatedAt: Date.now() }));
}

export function discardOwnedDraft(key: string): void {
  const raw = localStorage.getItem(key);
  if (!raw) return;
  try {
    const owner = (JSON.parse(raw) as { __owner?: string }).__owner;
    // Never delete a newer competing WebView's draft.
    if (owner && owner !== EDITOR_OWNER) return;
  } catch { return; }
  localStorage.removeItem(key);
}
