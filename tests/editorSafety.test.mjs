import test from 'node:test';
import assert from 'node:assert/strict';
import { UNNAMED_TASK, taskTitleForSave, isEmptyIdeaEdit, restoreIdeaDraft, captureCanClearAfterSave } from '../src/lib/editorSafety.ts';

test('blank new tasks get a readable persistent title', () => {
  assert.equal(taskTitleForSave(''), UNNAMED_TASK);
  assert.equal(taskTitleForSave('   \n  '), UNNAMED_TASK);
  assert.equal(taskTitleForSave('Tarefa importante'), 'Tarefa importante');
  assert.equal(taskTitleForSave('  Revisar texto  '), 'Revisar texto');
});

test('blank idea edits cannot shadow existing database content on reopen', () => {
  assert.equal(isEmptyIdeaEdit(''), true);
  assert.equal(isEmptyIdeaEdit('\t \n'), true);
  assert.equal(isEmptyIdeaEdit('Ideia'), false);
  assert.equal(restoreIdeaDraft('', 'Ideia original'), false);
  assert.equal(restoreIdeaDraft(' \n ', 'Ideia original'), false);
  assert.equal(restoreIdeaDraft('Texto válido em rascunho', 'Ideia original'), true);
  assert.equal(restoreIdeaDraft('Ideia original', 'Ideia original'), false);
});

test('capture never clears typing or a changed capture kind during an IPC write', () => {
  assert.equal(captureCanClearAfterSave('A','idea','A','idea'),true);
  assert.equal(captureCanClearAfterSave('A','idea','AB','idea'),false);
  assert.equal(captureCanClearAfterSave('A','idea','A','task'),false);
  assert.equal(captureCanClearAfterSave('','task','','task'),true);
});
