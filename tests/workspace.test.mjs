import test from 'node:test';
import assert from 'node:assert/strict';
import { buildWorkspace, nextTask, searchWorkspace, toSlides } from '../src/lib/workspace.ts';

const note = (id, title, body = '', updated = 10) => ({
  id, title, body, kind: 'note', pinned: false, remind_at: null,
  notified_at: null, created_at: 1, updated_at: updated, deleted_at: null
});
const task = (id, title, dueAt = null, updated = 10) => ({
  id, title, details: '', due_at: dueAt, completed_at: null,
  created_at: updated, updated_at: updated, deleted_at: null
});
const idea = (id, text, updated = 10) => ({
  id, text, created_at: 1, updated_at: updated, processed_at: null
});

test('unifies notes, tasks, and unprocessed ideas without modifying originals', () => {
  const notes = [note('n', 'Lembrete', 'Apresentação', 30)];
  const tasks = [task('t', 'Revisar slides', null, 20)];
  const ideas = [idea('i', 'Estudar\npara amanhã', 40), {...idea('old', 'Processado', 50), processed_at: 60}];
  const result = buildWorkspace(notes, tasks, ideas);
  assert.deepEqual(result.map(e => e.key), ['idea:i', 'note:n', 'task:t']);
  assert.equal(result[0].text, 'Estudar\npara amanhã');
  assert.equal(notes[0].body, 'Apresentação');
});

test('search ignores accents, case and outer spaces', () => {
  const rows = buildWorkspace([note('n', 'REUNIÃO do sábado')], [task('t', 'Revisar')], [idea('i', 'Ideia rápida')]);
  assert.deepEqual(searchWorkspace(rows, ' reuniao ').map(e => e.key), ['note:n']);
  assert.deepEqual(searchWorkspace(rows, 'RAPIDA').map(e => e.key), ['idea:i']);
  assert.deepEqual(searchWorkspace(rows, '').length, 3);
});

test('filter by type does not duplicate entries', () => {
  const rows = buildWorkspace([note('n', 'Revisão')], [task('t', 'Revisão')], [idea('i', 'Revisão')]);
  assert.deepEqual(searchWorkspace(rows, 'revisao', 'task').map(e => e.key), ['task:t']);
  assert.deepEqual(searchWorkspace(rows, '', 'idea').map(e => e.key), ['idea:i']);
});

test('nextTask skips completed and deleted and prefers earlier due date', () => {
  const tasks = [
    {...task('completed', 'Concluída', 5), completed_at: 10},
    task('later', 'Depois', 200),
    task('urgent', 'Primeiro', 100),
    {...task('deleted', 'Apagada', 1), deleted_at: 12}
  ];
  assert.equal(nextTask(tasks)?.id, 'urgent');
  assert.equal(tasks[0].id, 'completed');
});

test('presentation cards split only on standalone separators', () => {
  assert.deepEqual(toSlides('Primeiro\n---\nSegundo\n---\nTerceiro'), ['Primeiro', 'Segundo', 'Terceiro']);
  assert.deepEqual(toSlides('A---B'), ['A---B']);
});
