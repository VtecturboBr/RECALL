import test from 'node:test';
import assert from 'node:assert/strict';
import {
  BOARD_HISTORY_LIMIT,emptyBoardHistory,createBoardHistoryAction,
  recordBoardHistory,topBoardHistory,canApplyBoardHistory,
  historyDestination,completeBoardHistory,forgetBoardCardHistory,
  editableBoardFields
} from '../src/lib/boardHistory.ts';

const base={id:'card1',board_id:'default',kind:'note',x:10,y:20,width:280,
  height:210,content:'Primeira ideia',revision:0,created_at:1,updated_at:1};
const changed={...base,content:'Primeira ideia revisada',revision:1};
const moved={...changed,x:70,y:120,revision:2};
const resized={...moved,width:500,height:300,revision:3};
function append(state,a,b) {
  const entry=createBoardHistoryAction(a,b);
  assert.ok(entry);
  return recordBoardHistory(state,entry);
}
test('history stores only committed changes and ignores identical snapshots',()=>{
  assert.equal(createBoardHistoryAction(base,base),null);
  assert.equal(createBoardHistoryAction(base,{...base,revision:1}),null);
  assert.equal(createBoardHistoryAction(base,{...base,id:'other'}),null);
  const result=append(emptyBoardHistory(),base,changed);
  assert.equal(result.undo.length,1);
  assert.equal(result.redo.length,0);
  assert.equal(result.undo[0].label,'edição de conteúdo');
});
test('undo/redo of content, move and resize always matches current card fields',()=>{
  let state=emptyBoardHistory();
  state=append(state,base,changed);
  state=append(state,changed,moved);
  state=append(state,moved,resized);
  const resize=topBoardHistory(state,'undo');
  assert.ok(resize);
  assert.equal(resize.label,'redimensionamento');
  assert.equal(canApplyBoardHistory(resize,'undo','default',resized),true);
  assert.deepEqual(historyDestination(resize,'undo'),editableBoardFields(moved));
  state=completeBoardHistory(state,'undo');
  assert.equal(state.undo.length,2);
  assert.equal(state.redo.length,1);
  assert.equal(canApplyBoardHistory(topBoardHistory(state,'undo'),'undo','default',moved),true);
  state=completeBoardHistory(state,'undo');
  assert.equal(canApplyBoardHistory(topBoardHistory(state,'redo'),'redo','default',changed),true);
  state=completeBoardHistory(state,'redo');
  assert.equal(state.undo.length,2);
  assert.equal(state.redo.length,1);
});
test('untracked changes from another window never match history snapshots',()=>{
  const state=append(emptyBoardHistory(),base,changed);
  const action=topBoardHistory(state,'undo');
  assert.ok(!canApplyBoardHistory(action,'undo','default',{...changed,content:'Alteração remota',revision:2}));
  assert.ok(!canApplyBoardHistory(action,'undo','other',changed));
  assert.ok(!canApplyBoardHistory(action,'undo','default',undefined));
  assert.equal(state.undo.length,1); // failed CAS / local mismatch never pops history
});
test('redo clears only after an actual new confirmed edit',()=>{
  let state=append(emptyBoardHistory(),base,changed);
  state=completeBoardHistory(state,'undo');
  assert.equal(state.redo.length,1);
  const next={...base,x:44,revision:2};
  state=append(state,base,next);
  assert.equal(state.redo.length,0);
  assert.equal(state.undo.length,1);
});
test('history is bounded and unrelated cards are preserved on delete',()=>{
  let state=emptyBoardHistory();
  const other={...base,id:'other'};
  state=append(state,other,{...other,x:21});
  for(let i=0;i<BOARD_HISTORY_LIMIT+20;i++){
    state=append(state,{...base,x:i},{...base,x:i+1});
  }
  assert.equal(state.undo.length,BOARD_HISTORY_LIMIT);
  assert.equal(state.undo[0].before.x,20);
  state=forgetBoardCardHistory(state,'card1');
  assert.equal(state.undo.length,0);
  state=append(state,other,{...other,width:300});
  assert.equal(state.undo.length,1);
});
