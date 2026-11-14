import type { BoardItem, BoardKind } from './types';

export const BOARD_DRAFT_PREFIX = 'recall-board-draft:';
export const BOARD_RECOVERY_PREFIX = 'recall-draft-recovery:' + BOARD_DRAFT_PREFIX;
const SUPPORTED_KINDS: readonly string[] = ['note','link','table','image'];

export interface BoardDraftSnapshot {
  boardId: string;
  kind: BoardKind;
  content: string;
  revision: number;
  x: number;
  y: number;
  width: number;
  height: number;
  __owner?: string;
  __updatedAt?: number;
}

export interface BoardRecoveryEntry {
  key: string;
  raw: string;
  itemId: string;
  snapshot: BoardDraftSnapshot | null;
  status: 'conflict' | 'orphan' | 'competing' | 'unreadable' | 'pending';
}

function isValidNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value);
}

/** Accept historical (v10.0) snapshots without boardId/kind only when the live card identifies them. */
export function parseBoardDraft(raw: string, live: BoardItem | undefined): BoardDraftSnapshot | null {
  if (raw.length > 250_000) return null;
  let data: unknown;
  try { data = JSON.parse(raw); } catch { return null; }
  if (data === null || typeof data !== 'object' || Array.isArray(data)) return null;
  const record = data as Record<string, unknown>;
  const boardId = record.boardId ?? live?.board_id;
  const kind = record.kind ?? live?.kind;
  if (typeof boardId !== 'string' || boardId.length === 0 || boardId.length > 256
      || typeof kind !== 'string' || !SUPPORTED_KINDS.includes(kind)
      || typeof record.content !== 'string' || record.content.length > 100_000
      || !Number.isSafeInteger(record.revision) || (record.revision as number) < 0
      || !isValidNumber(record.x) || !isValidNumber(record.y)
      || !isValidNumber(record.width) || !isValidNumber(record.height)
      || Math.abs(record.x) > 10_000_000 || Math.abs(record.y) > 10_000_000
      || record.width < 160 || record.width > 1600
      || record.height < 110 || record.height > 1600) {
    return null;
  }
  // A stored kind or board ID may not relabel a real live card.
  if (live && (boardId !== live.board_id || kind !== live.kind)) return null;
  return {
    boardId,
    kind: kind as BoardKind,
    content: record.content,
    revision: record.revision as number,
    x: record.x,
    y: record.y,
    width: record.width,
    height: record.height,
    ...(typeof record.__owner === 'string' ? {__owner: record.__owner} : {}),
    ...(isValidNumber(record.__updatedAt) ? {__updatedAt: record.__updatedAt} : {})
  };
}

export function sameBoardDraftFields(snapshot: BoardDraftSnapshot, item: BoardItem): boolean {
  return snapshot.x === item.x && snapshot.y === item.y
    && snapshot.width === item.width && snapshot.height === item.height
    && snapshot.content === item.content;
}

export function boardDraftState(
  raw: string | null,
  item: BoardItem
): 'absent' | 'invalid' | 'clean' | 'restore' | 'conflict' {
  if (!raw) return 'absent';
  const snapshot = parseBoardDraft(raw,item);
  if (!snapshot) return 'invalid';
  if (snapshot.revision !== item.revision) return 'conflict';
  return sameBoardDraftFields(snapshot,item) ? 'clean' : 'restore';
}

function extractId(key: string): {id: string; archived: boolean} | null {
  if (key.startsWith(BOARD_DRAFT_PREFIX)) {
    const id=key.slice(BOARD_DRAFT_PREFIX.length);
    return id.length>0 && !id.includes(':') ? {id,archived:false} : null;
  }
  if (key.startsWith(BOARD_RECOVERY_PREFIX)) {
    const tail=key.slice(BOARD_RECOVERY_PREFIX.length);
    const separator=tail.indexOf(':');
    return separator>0 ? {id:tail.slice(0,separator),archived:true} : null;
  }
  return null;
}

/** Never modifies storage. Recovery is intentionally an explicit, user-controlled operation. */
export function listBoardRecovery(
  entries: ReadonlyArray<{key: string; raw: string}>,
  boardId: string,
  liveCards: readonly BoardItem[],
  maxEntries = 100,
  dirtyIds: ReadonlySet<string> = new Set(),
  ownerId?: string
): BoardRecoveryEntry[] {
  const live = new Map(liveCards.map(card=>[card.id,card]));
  const found: BoardRecoveryEntry[] = [];
  const cap = Math.max(0, Math.min(250, Math.floor(maxEntries)));
  for (const entry of entries) {
    if (found.length >= cap) break;
    const descriptor = extractId(entry.key);
    if (!descriptor) continue;
    const card = live.get(descriptor.id);
    const snapshot = parseBoardDraft(entry.raw,card);
    // Older snapshots lacking boardId cannot be reliably located after deletion.
    if (snapshot?.boardId !== boardId) {
      // Corrupt drafts linked to a currently live card should remain visible.
      if (!snapshot && card?.board_id === boardId) {
        found.push({key:entry.key,raw:entry.raw,itemId:descriptor.id,
          snapshot:null,status:'unreadable'});
      }
      continue;
    }
    if (descriptor.archived || (snapshot.__owner && ownerId && snapshot.__owner !== ownerId)) {
      found.push({key:entry.key,raw:entry.raw,itemId:descriptor.id,
        snapshot,status:'competing'});
    } else if (!card) {
      found.push({key:entry.key,raw:entry.raw,itemId:descriptor.id,
        snapshot,status:'orphan'});
    } else if (snapshot.revision !== card.revision) {
      found.push({key:entry.key,raw:entry.raw,itemId:descriptor.id,
        snapshot,status:'conflict'});
    } else if (dirtyIds.has(descriptor.id) && !sameBoardDraftFields(snapshot,card)) {
      found.push({key:entry.key,raw:entry.raw,itemId:descriptor.id,
        snapshot,status:'pending'});
    }
  }
  return found.sort((a,b)=>(b.snapshot?.__updatedAt ?? 0)-(a.snapshot?.__updatedAt ?? 0));
}

/** Only clear a confirmed, unchanged snapshot; no newer tab/draft can be discarded. */
export function shouldDiscardExactBoardDraft(currentRaw: string | null, expectedRaw: string | null): boolean {
  return currentRaw !== null && expectedRaw !== null && currentRaw === expectedRaw;
}
