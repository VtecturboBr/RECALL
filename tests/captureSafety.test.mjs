import test from 'node:test';
import assert from 'node:assert/strict';
import { shouldClearCaptureAfterSave } from '../src/lib/captureSafety.ts';
import { taskTitleForSave } from '../src/lib/editorSafety.ts';

test('typing after IPC save starts is never cleared', () => {
  assert.equal(shouldClearCaptureAfterSave('Nova digitação','idea','Texto enviado','idea'),false);
  assert.equal(shouldClearCaptureAfterSave('Texto enviado','task','Texto enviado','idea'),false);
  assert.equal(shouldClearCaptureAfterSave('Texto enviado','idea','Texto enviado','idea'),true);
});
test('blank quick-capture task shares the same placeholder as normal editor', () => {
  assert.equal(taskTitleForSave(''),'Nova tarefa');
  assert.equal(taskTitleForSave('  '),'Nova tarefa');
});
