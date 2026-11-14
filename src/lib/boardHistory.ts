import type { BoardItem } from './types';

/**
 * Session-scoped history for already persisted whiteboard card updates.
 *
 * No undo/redo touches local storage or other SQLite rows. The Svelte caller
 * flushes pending edits and uses the original SQLite revision CAS on each
 * update. This module never guesses whether remote data may be overwritten.
 */
export type BoardEditableFields = Pick<BoardItem,'x'|'y'|'width'|'height'|'content'>;
export type BoardHistoryDirection = 'undo' | 'redo';
export interface BoardHistoryAction {
  itemId: string;
  boardId: string;
  before: BoardEditableFields;
  after: BoardEditableFields;
  label: string;
}
export interface BoardHistoryState {
  undo: BoardHistoryAction[];
  redo: BoardHistoryAction[];
}
export const BOARD_HISTORY_LIMIT = 80;

export function emptyBoardHistory(): BoardHistoryState {
  return {undo:[],redo:[]};
}
export function editableBoardFields(card: BoardEditableFields): BoardEditableFields {
  return {
    x:card.x,y:card.y,width:card.width,height:card.height,content:card.content
  };
}
export function sameBoardFields(a: BoardEditableFields,b: BoardEditableFields): boolean {
  return a.x===b.x && a.y===b.y && a.width===b.width
    && a.height===b.height && a.content===b.content;
}
export function describeBoardAction(before: BoardEditableFields,after: BoardEditableFields): string {
  const content=before.content!==after.content;
  const location=before.x!==after.x || before.y!==after.y;
  const size=before.width!==after.width || before.height!==after.height;
  if (content && (location || size)) return 'edição e layout';
  if (content) return 'edição de conteúdo';
  if (size && location) return 'movimentação e tamanho';
  if (size) return 'redimensionamento';
  return 'movimentação';
}
export function createBoardHistoryAction(before: BoardItem,after: BoardItem): BoardHistoryAction | null {
  if (before.id!==after.id || before.board_id!==after.board_id
      || before.kind!==after.kind || sameBoardFields(before,after)) return null;
  return {
    itemId:before.id,boardId:before.board_id,
    before:editableBoardFields(before),
    after:editableBoardFields(after),
    label:describeBoardAction(before,after)
  };
}
export function recordBoardHistory(
  state: BoardHistoryState,action: BoardHistoryAction,limit=BOARD_HISTORY_LIMIT
): BoardHistoryState {
  const safeLimit=Math.max(1,Math.min(200,Number.isFinite(limit)?Math.floor(limit):BOARD_HISTORY_LIMIT));
  return {
    undo:[...state.undo,action].slice(-safeLimit),
    redo:[]
  };
}
export function topBoardHistory(state: BoardHistoryState,direction: BoardHistoryDirection): BoardHistoryAction | null {
  return (direction==='undo'?state.undo:state.redo).at(-1)??null;
}
export function historyDestination(action: BoardHistoryAction,direction: BoardHistoryDirection): BoardEditableFields {
  return editableBoardFields(direction==='undo'?action.before:action.after);
}
/** Check *only* local state. The backend CAS revision is the final authority. */
export function canApplyBoardHistory(
  action: BoardHistoryAction,direction: BoardHistoryDirection,
  boardId:string,current: BoardItem | undefined
): boolean {
  return current!==undefined && action.boardId===boardId
    && current.board_id===boardId && current.id===action.itemId
    && sameBoardFields(current,direction==='undo'?action.after:action.before);
}
/** Called only *after* the database confirms the revision-checked update. */
export function completeBoardHistory(
  state: BoardHistoryState,direction: BoardHistoryDirection
): BoardHistoryState {
  if (direction==='undo') {
    const action=state.undo.at(-1);
    return action?{undo:state.undo.slice(0,-1),redo:[...state.redo,action]}:state;
  }
  const action=state.redo.at(-1);
  return action?{undo:[...state.undo,action],redo:state.redo.slice(0,-1)}:state;
}
export function forgetBoardCardHistory(state:BoardHistoryState,itemId:string):BoardHistoryState {
  return {
    undo:state.undo.filter(action=>action.itemId!==itemId),
    redo:state.redo.filter(action=>action.itemId!==itemId)
  };
}
