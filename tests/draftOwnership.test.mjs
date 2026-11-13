import test from 'node:test';
import assert from 'node:assert/strict';
import { saveOwnedDraft, discardOwnedDraft } from '../src/lib/draftOwnership.ts';

function fakeStorage() {
  const values = new Map();
  return {
    getItem: key => values.has(key) ? values.get(key) : null,
    setItem: (key, value) => values.set(key, value),
    removeItem: key => values.delete(key),
    keys: () => [...values.keys()]
  };
}

test('competing WebViews cannot erase a foreign recovery draft', () => {
  globalThis.localStorage = fakeStorage();
  const key = 'recall-task-draft:item-1';
  const foreign = JSON.stringify({ title: 'Outro texto', revision: 4, __owner: 'other-window' });
  localStorage.setItem(key, foreign);
  discardOwnedDraft(key);
  assert.equal(localStorage.getItem(key), foreign);
  saveOwnedDraft(key, { title: 'Minha versão', revision: 4 });
  assert.equal(localStorage.getItem('recall-draft-recovery:' + key + ':other-window'), foreign);
  assert.equal(JSON.parse(localStorage.getItem(key)).title, 'Minha versão');
  discardOwnedDraft(key);
  assert.equal(localStorage.getItem(key), null);
  assert.equal(localStorage.getItem('recall-draft-recovery:' + key + ':other-window'), foreign);
  delete globalThis.localStorage;
});
