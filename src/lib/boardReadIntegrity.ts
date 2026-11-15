import type { BoardReadStamp } from './types';

/**
 * Fail-closed best-effort consistency check around keyset pagination.
 * The stamp is derived from the active SQLite rows of one board. Regular
 * application writes increment revisions, even for geometry-only edits.
 */
export function sameBoardReadStamp(
  start:BoardReadStamp,
  end:BoardReadStamp
):boolean {
  if(!isValidBoardReadStamp(start)||!isValidBoardReadStamp(end)) return false;
  return start.card_count===end.card_count
    && start.revision_total===end.revision_total
    && start.rowid_total===end.rowid_total
    && start.content_bytes_total===end.content_bytes_total;
}
export function isValidBoardReadStamp(value:unknown):value is BoardReadStamp {
  if (!value || typeof value !== 'object') return false;
  const s=value as Record<string,unknown>;
  return ['card_count','revision_total','rowid_total','content_bytes_total']
    .every(k=>typeof s[k]==='number' && Number.isSafeInteger(s[k]) && s[k]>=0);
}
export function assertConsistentBoardLoad(
  start:BoardReadStamp,
  end:BoardReadStamp,
  received:number
):void {
  if (!sameBoardReadStamp(start,end) || received!==end.card_count) {
    throw new Error(
      'QUADRO_ALTERADO_DURANTE_LEITURA: outra janela modificou o quadro. '+
      'Os lotes recebidos foram descartados; tente novamente.'
    );
  }
}
