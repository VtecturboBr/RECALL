import test from 'node:test';
import assert from 'node:assert/strict';
import { parseTaskCapture } from '../src/lib/smartCapture.ts';
import { taskPeriod, tasksByPeriod } from '../src/lib/workspace.ts';

const now = new Date(2026, 10, 12, 14, 30);
const day = (offset) => new Date(2026, 10, 12 + offset).getTime();

test('tomorrow at a time is extracted only at the end', () => {
  const result = parseTaskCapture('Revisar slides amanhã às 15h30', now);
  assert.equal(result.title, 'Revisar slides');
  assert.equal(result.bucket, 'later');
  assert.equal(result.plannedFor, day(1));
  assert.equal(result.dueAt, new Date(2026, 10, 13, 15, 30).getTime());
  assert.equal(result.recognized, true);
});

test('relative dates preserve multiline details', () => {
  const result = parseTaskCapture('Enviar relatório depois de amanhã\nConfirmar o anexo', now);
  assert.equal(result.bucket, 'later');
  assert.equal(result.plannedFor, day(2));
  assert.equal(result.title, 'Enviar relatório');
  assert.equal(result.details, 'Confirmar o anexo');
  assert.equal(result.dueAt, null);
});

test('natural planning words without a date', () => {
  assert.equal(parseTaskCapture('Organizar fotos algum dia', now).bucket, 'someday');
  const later = parseTaskCapture('Organizar o quarto semana que vem', now);
  assert.equal(later.bucket, 'later');
  assert.equal(later.plannedFor, null);
  assert.equal(parseTaskCapture('Trabalhar hoje', now).bucket, 'today');
});

test('calendar dates use local year and reject invalid dates', () => {
  const next = parseTaskCapture('Comprar remédio em 15/11 às 09:00', now);
  assert.equal(next.plannedFor, new Date(2026, 10, 15).getTime());
  assert.equal(next.dueAt, new Date(2026, 10, 15, 9).getTime());
  const yearRoll = parseTaskCapture('Planejar viagem 04/01', now);
  assert.equal(yearRoll.plannedFor, new Date(2027, 0, 4).getTime());
  const bad = parseTaskCapture('Comprar algo 31/02', now);
  assert.equal(bad.recognized, false);
  assert.equal(bad.title, 'Comprar algo 31/02');
  assert.equal(parseTaskCapture('Reunião 01/01/2020', now).recognized, false);
});

test('invalid and ambiguous phrases remain untouched', () => {
  assert.equal(parseTaskCapture('Ir à escola às 29h', now).recognized, false);
  assert.equal(parseTaskCapture('Ler livro hoje e amanhã', now).recognized, true); // last word only
  assert.equal(parseTaskCapture('O livro Amanhã foi ótimo ontem', now).recognized, false);
  const noData = parseTaskCapture('Arrumar a mesa', now);
  assert.equal(noData.title, 'Arrumar a mesa');
  assert.equal(noData.recognized, false);
  assert.equal(noData.plannedFor, null);
});

test('due dates do not generate notifications; only stage changes automatically', () => {
  const t = (id, bucket, planned_for) => ({id, title: id, details: '', due_at: null,
    bucket, planned_for, completed_at: null, created_at: 1, updated_at: 1, deleted_at: null});
  const items = [t('old', 'today', null),t('tomorrow', 'later', day(1)),t('future', 'later', day(5)),t('someday', 'someday', null)];
  assert.deepEqual(tasksByPeriod(items, 'today', now).map(x => x.id), ['old']);
  assert.deepEqual(tasksByPeriod(items, 'today', new Date(2026, 10, 13, 9)).map(x => x.id), ['old','tomorrow']);
  assert.deepEqual(tasksByPeriod(items, 'later', now).map(x => x.id), ['tomorrow','future']);
  assert.deepEqual(tasksByPeriod(items, 'someday', now).map(x => x.id), ['someday']);
  assert.equal(taskPeriod(t('legacy', undefined, null), now), 'today');
});
