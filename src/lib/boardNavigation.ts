/** Pure, offline navigation math. No DOM, storage or external services. */
export interface BoardRect { x: number; y: number; width: number; height: number }
export interface BoardBounds { left: number; top: number; width: number; height: number }
export interface BoardViewport { panX: number; panY: number; zoom: number }
export interface BoardSavedView { centerX: number; centerY: number; zoom: number }
export interface BoardReferencePreview { title: string; preview: string; available: boolean }

const MIN_ZOOM = 0.1;
const MAX_ZOOM = 2.2;
const MAX_CENTER = 10_000_000;

export function clampBoardZoom(value: number): number {
  return Number.isFinite(value) ? Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, value)) : 1;
}

export function boardBounds(items: readonly BoardRect[], padding = 120): BoardBounds {
  if (items.length === 0) return { left: -500, top: -340, width: 1000, height: 680 };
  const pad = Math.max(0, Number.isFinite(padding) ? padding : 0);
  let left = Infinity, top = Infinity, right = -Infinity, bottom = -Infinity;
  for (const item of items) {
    if (![item.x, item.y, item.width, item.height].every(Number.isFinite)) continue;
    left = Math.min(left, item.x);
    top = Math.min(top, item.y);
    right = Math.max(right, item.x + Math.max(0, item.width));
    bottom = Math.max(bottom, item.y + Math.max(0, item.height));
  }
  if (!Number.isFinite(left)) return { left: -500, top: -340, width: 1000, height: 680 };
  return {
    left: left - pad,
    top: top - pad,
    width: Math.max(1, right - left + 2 * pad),
    height: Math.max(1, bottom - top + 2 * pad)
  };
}

export function fitBoardView(items: readonly BoardRect[], viewportWidth: number, viewportHeight: number, padding = 70): BoardViewport {
  if (items.length === 0) return { panX: 0, panY: 0, zoom: 1 };
  const width = Math.max(1, Number.isFinite(viewportWidth) ? viewportWidth : 1);
  const height = Math.max(1, Number.isFinite(viewportHeight) ? viewportHeight : 1);
  const bounds = boardBounds(items, 0);
  const safePadding = Math.max(0, Number.isFinite(padding) ? padding : 0);
  const scale = clampBoardZoom(Math.min(
    Math.max(1, width - 2 * safePadding) / bounds.width,
    Math.max(1, height - 2 * safePadding) / bounds.height,
    1.5
  ));
  const cx = bounds.left + bounds.width / 2;
  const cy = bounds.top + bounds.height / 2;
  return { zoom: scale, panX: width / 2 - cx * scale, panY: height / 2 - cy * scale };
}

export function focusBoardItem(item: BoardRect, viewportWidth: number, viewportHeight: number, currentZoom: number): BoardViewport {
  const zoom = clampBoardZoom(Math.max(0.9, Math.min(1.4, currentZoom)));
  const cx = item.x + item.width / 2;
  const cy = item.y + item.height / 2;
  return {
    zoom,
    panX: viewportWidth / 2 - cx * zoom,
    panY: viewportHeight / 2 - cy * zoom
  };
}

export function saveBoardView(view: BoardViewport, width: number, height: number): BoardSavedView {
  const zoom = clampBoardZoom(view.zoom);
  return {
    centerX: (width / 2 - view.panX) / zoom,
    centerY: (height / 2 - view.panY) / zoom,
    zoom
  };
}

export function restoreBoardView(value: unknown, width: number, height: number): BoardViewport | null {
  if (!value || typeof value !== 'object') return null;
  const raw = value as Partial<BoardSavedView>;
  if (typeof raw.zoom !== 'number' || typeof raw.centerX !== 'number' || typeof raw.centerY !== 'number') return null;
  if (![raw.zoom, raw.centerX, raw.centerY].every(Number.isFinite)
      || raw.zoom < MIN_ZOOM || raw.zoom > MAX_ZOOM
      || Math.abs(raw.centerX) > MAX_CENTER || Math.abs(raw.centerY) > MAX_CENTER) return null;
  return {
    zoom: raw.zoom,
    panX: width / 2 - raw.centerX * raw.zoom,
    panY: height / 2 - raw.centerY * raw.zoom
  };
}

export function normalizeBoardText(value: string): string {
  return value.normalize('NFD').replace(/[\u0300-\u036f]/g, '').toLocaleLowerCase('pt-BR').trim();
}

export function searchBoardCards<T extends BoardRect & { id: string; content: string; kind: string }>(
  cards: readonly T[],
  references: Readonly<Record<string, BoardReferencePreview>>,
  query: string,
  maxResults = 40
): T[] {
  const needle = normalizeBoardText(query);
  if (!needle) return [];
  const limit = Math.max(0, Math.min(250, Math.floor(maxResults)));
  const matching: T[] = [];
  for (const card of cards) {
    const linked = references[card.id];
    const source = linked
      ? linked.available ? linked.title + ' ' + linked.preview : 'indisponivel'
      : card.content;
    if (normalizeBoardText(source).includes(needle)) {
      matching.push(card);
      if (matching.length >= limit) break;
    }
  }
  return matching;
}

export function miniRect(
  item: BoardRect, bounds: BoardBounds, width: number, height: number
): BoardRect {
  return {
    x: (item.x - bounds.left) * width / bounds.width,
    y: (item.y - bounds.top) * height / bounds.height,
    width: Math.max(3, item.width * width / bounds.width),
    height: Math.max(3, item.height * height / bounds.height)
  };
}

export function visibleWorldRect(view: BoardViewport, width: number, height: number): BoardRect {
  const zoom = clampBoardZoom(view.zoom);
  return { x: -view.panX / zoom, y: -view.panY / zoom, width: width / zoom, height: height / zoom };
}
