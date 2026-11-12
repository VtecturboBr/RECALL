import type { Task } from './types';

export interface SmartTaskDraft {
  title: string;
  details: string;
  bucket: Task['bucket'];
  plannedFor: number | null;
  dueAt: number | null;
  recognized: boolean;
  label: string;
}

function localDay(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

function validLocalDate(year: number, month: number, day: number): Date | null {
  const result = new Date(year, month - 1, day);
  return result.getFullYear() === year && result.getMonth() === month - 1 && result.getDate() === day
    ? result : null;
}

function nextDayMonth(day: number, month: number, now: Date): Date | null {
  for (let year = now.getFullYear(); year <= now.getFullYear() + 4; year++) {
    const candidate = validLocalDate(year, month, day);
    if (candidate && candidate.getTime() >= localDay(now).getTime()) return candidate;
  }
  return null;
}

function parseDateToken(token: string, now: Date): { bucket: Task['bucket']; date: Date | null } | null {
  const normalized = token.toLocaleLowerCase('pt-BR').normalize('NFD').replace(/[\u0300-\u036f]/g, '');
  if (normalized === 'algum dia') return { bucket: 'someday', date: null };
  if (normalized === 'semana que vem') return { bucket: 'later', date: null };
  if (normalized === 'hoje') return { bucket: 'today', date: localDay(now) };
  if (normalized === 'amanha') return { bucket: 'later', date: new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1) };
  if (normalized === 'depois de amanha') return { bucket: 'later', date: new Date(now.getFullYear(), now.getMonth(), now.getDate() + 2) };

  const match = /^(?:em\s+)?(\d{1,2})\/(\d{1,2})(?:\/(\d{4}))?$/.exec(normalized);
  if (!match) return null;
  const day = Number(match[1]);
  const month = Number(match[2]);
  const date = match[3]
    ? validLocalDate(Number(match[3]), month, day)
    : nextDayMonth(day, month, now);
  if (!date || date.getTime() < localDay(now).getTime()) return null;
  return { bucket: date.getTime() <= localDay(now).getTime() ? 'today' : 'later', date };
}

function previewText(bucket: Task['bucket'], date: Date | null, dueAt: number | null): string {
  if (bucket === 'someday') return 'Algum dia · sem data marcada';
  if (date) {
    const opts: Intl.DateTimeFormatOptions = {day: '2-digit', month: 'short'};
    const dateText = new Intl.DateTimeFormat('pt-BR', opts).format(date);
    const timeText = dueAt === null ? '' : ' às ' + new Intl.DateTimeFormat('pt-BR', {hour: '2-digit', minute: '2-digit'}).format(dueAt);
    return (bucket === 'today' ? 'Hoje' : 'Programado') + ' · ' + dateText + timeText;
  }
  return bucket === 'later' ? 'Depois · sem data marcada' : 'Hoje · sem horário';
}

/**
 * Conservative, deterministic PT-BR parser: only a final date/time expression is interpreted.
 * Invalid dates and uncertain expressions remain part of the original title (no silent changes).
 * This is scheduling metadata, not a notification or recurring reminder.
 */
export function parseTaskCapture(input: string, now: Date = new Date()): SmartTaskDraft {
  const cleaned = input.replace(/\r\n?/g, '\n').trim();
  const [firstLine = '', ...otherLines] = cleaned.split('\n');
  const original = firstLine.trim();
  const details = otherLines.join('\n').trim();
  const fallback: SmartTaskDraft = {
    title: original, details, bucket: 'today', plannedFor: null, dueAt: null,
    recognized: false, label: ''
  };
  if (!original) return fallback;

  const when = /(?:^|\s)(depois\s+de\s+amanh[ãa]|amanh[ãa]|hoje|algum\s+dia|semana\s+que\s+vem|(?:em\s+)?\d{1,2}\/\d{1,2}(?:\/\d{4})?)(?:\s+(?:às|as)\s+(\d{1,2})(?:[:h](\d{2}))?h?)?\s*$/iu.exec(original);
  const timeOnly = !when
    ? /(?:^|\s)(?:às|as)\s+(\d{1,2})(?:[:h](\d{2}))?h?\s*$/iu.exec(original)
    : null;
  if (!when && !timeOnly) return fallback;

  const rawTitle = original.slice(0, (when ?? timeOnly)!.index).trim();
  if (!rawTitle) return fallback;
  const hour = when ? when[2] : timeOnly?.[1];
  const minute = when ? when[3] : timeOnly?.[2];
  const hours = hour === undefined ? null : Number(hour);
  const minutes = minute === undefined ? 0 : Number(minute);
  if (hours !== null && (hours > 23 || minutes > 59)) return fallback;

  const parsed = when ? parseDateToken(when[1], now) : { bucket: 'today' as const, date: localDay(now) };
  if (!parsed || (parsed.bucket === 'someday' && hours !== null)) return fallback;
  const due = hours === null || parsed.date === null
    ? null
    : new Date(parsed.date.getFullYear(), parsed.date.getMonth(), parsed.date.getDate(), hours, minutes).getTime();

  return {
    title: rawTitle,
    details,
    bucket: parsed.bucket,
    plannedFor: parsed.bucket === 'later' && parsed.date !== null ? parsed.date.getTime() : null,
    dueAt: due,
    recognized: true,
    label: previewText(parsed.bucket, parsed.date, due)
  };
}
