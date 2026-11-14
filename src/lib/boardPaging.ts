import type { BoardPageCursor } from './types';

/** Validate every partial IPC payload before mixing its data with a board. */
export interface KeyedBoardRow { id: string; created_at: number }
export interface KeyedBoardPage<T extends KeyedBoardRow> {
  items: readonly T[];
  next_cursor: BoardPageCursor | null;
}
export const BOARD_PAGE_BATCH_SIZE = 120;

export function compareBoardCursor(a:BoardPageCursor,b:BoardPageCursor):number {
  if(a.created_at!==b.created_at)return a.created_at<b.created_at?-1:1;
  return a.id===b.id?0:(a.id<b.id?-1:1);
}

export function validateBoardPage<T extends KeyedBoardRow>(
  page:KeyedBoardPage<T>,
  previousCursor:BoardPageCursor|null,
  batchSize=BOARD_PAGE_BATCH_SIZE
):void {
  if(!page || !Array.isArray(page.items)
      || page.items.length>batchSize || !Number.isSafeInteger(batchSize)
      || batchSize<1 || batchSize>200) {
    throw new Error('O Quadro recebeu um lote de tamanho inválido.');
  }
  if(page.next_cursor && page.items.length===0) {
    throw new Error('A paginação do Quadro não avançou.');
  }
  let last=previousCursor;
  for(const item of page.items) {
    if(typeof item.id!=='string' || !item.id
        || !Number.isSafeInteger(item.created_at)
        || item.created_at<0) {
      throw new Error('Um cartão retornou um cursor inválido.');
    }
    const cursor={id:item.id,created_at:item.created_at};
    if(last && compareBoardCursor(last,cursor)>=0) {
      throw new Error('A paginação retornou cartões repetidos ou fora de ordem.');
    }
    last=cursor;
  }
  if(page.next_cursor && (!last ||
       compareBoardCursor(last,page.next_cursor)!==0)) {
    throw new Error('O próximo cursor não corresponde ao último cartão.');
  }
}

/** Preserve source order; no sort or mutation of the original rows. */
export function mergeBoardPage<T extends KeyedBoardRow>(
  accumulator: T[],
  page:KeyedBoardPage<T>,
  previousCursor:BoardPageCursor|null,
  batchSize=BOARD_PAGE_BATCH_SIZE
):BoardPageCursor|null {
  validateBoardPage(page,previousCursor,batchSize);
  accumulator.push(...page.items);
  return page.next_cursor;
}
