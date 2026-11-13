import type { DailyReviewItem, ReviewKind } from './types';

/** A local calendar day, not UTC, because the review is chosen by the user. */
export function reviewDayKey(date: Date): string {
  const pad = (value: number) => String(value).padStart(2, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

export function reviewItemKey(item: DailyReviewItem): string {
  return item.target_type + ':' + item.target_id + ':' + item.kind;
}

export function visibleReviewItems(items: DailyReviewItem[], hidden: ReadonlySet<string>): DailyReviewItem[] {
  return items.filter(item => !hidden.has(reviewItemKey(item)));
}

export const reviewLabels: Record<ReviewKind, string> = {
  reminder: 'Lembrete',
  overdue_task: 'Prazo passado',
  interrupted_focus: 'Foco interrompido',
  missing_step: 'Sem próximo passo',
  stale_task: 'Sem atividade recente'
};
