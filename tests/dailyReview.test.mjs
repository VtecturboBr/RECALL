import test from 'node:test';
import assert from 'node:assert/strict';
import { reviewDayKey, reviewItemKey, visibleReviewItems, reviewLabels } from '../src/lib/dailyReview.ts';

test('review day keys follow the local calendar and zero-pad month/day', () => {
  assert.equal(reviewDayKey(new Date(2026, 0, 2, 23, 59)), '2026-01-02');
  assert.equal(reviewDayKey(new Date(2026, 10, 13, 0, 1)), '2026-11-13');
  assert.notEqual(reviewDayKey(new Date(2026, 10, 13)), reviewDayKey(new Date(2026, 10, 14)));
});

test('hide only selected review reasons, without changing source items', () => {
  const items = [
    { kind:'reminder', target_type:'task', target_id:'one', title:'Lembrete' },
    { kind:'overdue_task', target_type:'task', target_id:'one', title:'Prazo' },
    { kind:'reminder', target_type:'note', target_id:'one', title:'Nota' }
  ];
  const hidden = new Set([reviewItemKey(items[0])]);
  assert.deepEqual(visibleReviewItems(items, hidden).map(item => item.title), ['Prazo', 'Nota']);
  assert.equal(items.length, 3, 'review cannot mutate persistent tasks or reminders');
  assert.equal(reviewItemKey(items[0]), 'task:one:reminder');
  assert.equal(reviewItemKey(items[2]), 'note:one:reminder');
  assert.equal(reviewLabels.interrupted_focus, 'Foco interrompido');
});
