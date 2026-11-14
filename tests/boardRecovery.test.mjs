import test from 'node:test';
import assert from 'node:assert/strict';
import {
  parseBoardDraft,boardDraftState,listBoardRecovery,
  shouldDiscardExactBoardDraft,sameBoardDraftFields
} from '../src/lib/boardRecovery.ts';

const card = {id:'card-a',board_id:'default',kind:'note',
  content:'Autorizado',x:10,y:20,width:280,height:210,revision:4,created_at:1,updated_at:2};
const snapshot = (overrides={})=>JSON.stringify({boardId:'default',kind:'note',content:'Rascunho',
  x:12,y:20,width:280,height:210,revision:4,...overrides});

test('old draft without board identifier can be restored only for a matching live item',()=>{
  const old=JSON.stringify({content:'texto antigo',revision:4,x:10,y:20,width:280,height:210});
  assert.equal(parseBoardDraft(old,card)?.boardId,'default');
  assert.equal(parseBoardDraft(old,undefined),null);
  assert.equal(boardDraftState(old,card),'restore');
});
test('strict parsing rejects injected board, oversized text and invalid geometry',()=>{
  assert.equal(parseBoardDraft(snapshot({boardId:'other'}),card),null);
  assert.equal(parseBoardDraft(snapshot({kind:'image'}),card),null);
  assert.equal(parseBoardDraft(snapshot({x:Number.MAX_VALUE}),card),null);
  assert.equal(parseBoardDraft(snapshot({width:5}),card),null);
  assert.equal(parseBoardDraft(snapshot({content:'a'.repeat(100_001)}),card),null);
  assert.equal(parseBoardDraft('not json',card),null);
});
test('revision mismatch never auto-restores over newer SQLite revision',()=>{
  assert.equal(boardDraftState(snapshot({revision:3}),card),'conflict');
  assert.equal(boardDraftState(snapshot({revision:4}),card),'restore');
  assert.equal(boardDraftState(snapshot({content:'Autorizado',x:10}),card),'clean');
  assert.equal(boardDraftState(null,card),'absent');
});
test('archive copies and deleted-card recoveries remain available without changing source',()=>{
  const orphan=snapshot({__updatedAt:20});
  const result=listBoardRecovery([
    {key:'recall-board-draft:card-a',raw:snapshot({revision:2,__updatedAt:10})},
    {key:'recall-draft-recovery:recall-board-draft:card-a:other',raw:snapshot({__updatedAt:30})},
    {key:'recall-board-draft:deleted-card',raw:orphan},
    {key:'recall-board-draft:external-card',raw:snapshot({boardId:'other'})},
    {key:'other-key',raw:orphan}
  ],'default',[card]);
  assert.equal(result.length,3);
  assert.deepEqual(result.map(v=>v.status),['competing','orphan','conflict']);
  assert.equal(result[0].itemId,'card-a');
  assert.equal(result[1].itemId,'deleted-card');
});
test('invalid local records remain listed for a live card and never overwrite it',()=>{
  const result=listBoardRecovery([{key:'recall-board-draft:card-a',raw:'{broken'}],
    'default',[card]);
  assert.equal(result.length,1);
  assert.equal(result[0].status,'unreadable');
  assert.equal(result[0].snapshot,null);
});
test('failed saves and competing owners appear in recovery list',()=>{
  const entries=[
    {key:'recall-board-draft:card-a',raw:snapshot({__owner:'other'})}
  ];
  const competing=listBoardRecovery(entries,'default',[card],100,new Set(),'this-editor');
  assert.equal(competing.length,1);
  assert.equal(competing[0].status,'competing');
  const pending=listBoardRecovery(
    [{key:'recall-board-draft:card-a',raw:snapshot()}],
    'default',[card],100,new Set(['card-a']),'this-editor'
  );
  assert.equal(pending[0].status,'pending');
});

test('only the exact original snapshot may be removed after a save or user decision',()=>{
  const first=snapshot();
  const second=snapshot({content:'Outra alteração'});
  assert.ok(shouldDiscardExactBoardDraft(first,first));
  assert.ok(!shouldDiscardExactBoardDraft(second,first));
  assert.ok(!shouldDiscardExactBoardDraft(null,first));
  assert.ok(sameBoardDraftFields(parseBoardDraft(
    snapshot({content:'Autorizado',x:10}),card),card));
});
