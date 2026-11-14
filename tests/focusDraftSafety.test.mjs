import test from 'node:test';
import assert from 'node:assert/strict';
import { shouldClearFocusDraft } from '../src/lib/focusDraftSafety.ts';

test('clear only the exact focus summary committed to SQLite', () => {
  assert.equal(shouldClearFocusDraft('antes', 'antes'), true);
  assert.equal(shouldClearFocusDraft('antes', 'texto digitado depois'), false);
  assert.equal(shouldClearFocusDraft('resumo', null), false);
  assert.equal(shouldClearFocusDraft(null, 'texto novo'), false);
  assert.equal(shouldClearFocusDraft(null, null), false);
  assert.equal(shouldClearFocusDraft('', ''), true);
});
