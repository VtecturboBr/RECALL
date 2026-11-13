import test from 'node:test';
import assert from 'node:assert/strict';
import { availableLinkTargets, linkedEntryKey } from '../src/lib/related.ts';

test('related candidates exclude themselves and existing associations', () => {
  const entries = [
    { type:'note', key:'note:n1', title:'Zeta' },
    { type:'task', key:'task:t1', title:'Beta' },
    { type:'idea', key:'idea:i1', title:'Árvore' },
    { type:'note', key:'note:n2', title:'Delta' }
  ];
  const links = [{ id:'l1', item_type:'task', item_id:'t1', title:'Beta' }];
  const available = availableLinkTargets(entries, 'note:n1', links);
  assert.deepEqual(available.map(e => e.key), ['idea:i1','note:n2']);
  assert.equal(linkedEntryKey(links[0]), 'task:t1');
  assert.equal(availableLinkTargets(entries,'note:n1',[]).length,3);
});
