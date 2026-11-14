import test from 'node:test';
import assert from 'node:assert/strict';
import {
  BOARD_PAGE_BATCH_SIZE,compareBoardCursor,validateBoardPage,mergeBoardPage
} from '../src/lib/boardPaging.ts';
const entry=(i,timestamp=500)=>({id:String(i).padStart(7,'0'),created_at:timestamp,content:'x'.repeat(100)});
test('cursor ordering is stable when thousands of rows have the same timestamp',()=>{
  const all=Array.from({length:5_003},(_,i)=>entry(i));
  const merged=[];
  let cursor=null;
  for(let i=0;i<all.length;i+=BOARD_PAGE_BATCH_SIZE){
    const cards=all.slice(i,i+BOARD_PAGE_BATCH_SIZE);
    const hasMore=i+BOARD_PAGE_BATCH_SIZE<all.length;
    cursor=mergeBoardPage(merged,{items:cards,next_cursor:hasMore?
      {id:cards.at(-1).id,created_at:cards.at(-1).created_at}:null
    },cursor);
  }
  assert.equal(merged.length,5_003);
  assert.deepEqual(merged.map(x=>x.id),all.map(x=>x.id));
  assert.equal(cursor,null);
});
test('prevents corrupt or cyclic cursors before committing partial board',()=>{
  const a=entry(1),b=entry(2);
  const stored=[];
  const first=mergeBoardPage(stored,{
    items:[a],next_cursor:{id:a.id,created_at:a.created_at}
  },null);
  assert.throws(()=>mergeBoardPage(stored,{
    items:[a],next_cursor:{id:a.id,created_at:a.created_at}
  },first),/repetidos|ordem/);
  assert.deepEqual(stored,[a]);
  assert.throws(()=>validateBoardPage({items:[],next_cursor:first},first),/avançou/);
  assert.throws(()=>validateBoardPage({items:[b],next_cursor:first},first),/corresponde/);
});
test('invalid page sizes and timestamps are rejected',()=>{
  assert.throws(()=>validateBoardPage({items:[entry(1)],next_cursor:null},null,0),/tamanho/);
  assert.throws(()=>validateBoardPage({items:Array.from({length:201},(_,i)=>entry(i)),next_cursor:null},null,200),/tamanho/);
  assert.throws(()=>validateBoardPage({items:[{id:'a',created_at:NaN}],next_cursor:null},null),/inválido/);
  assert.equal(compareBoardCursor({created_at:9,id:'z'},{created_at:10,id:'a'}),-1);
  assert.equal(compareBoardCursor({created_at:10,id:'a'},{created_at:10,id:'b'}),-1);
});
test('empty terminal page is valid; pagination must finish rather than retry',()=>{
  validateBoardPage({items:[],next_cursor:null},{id:'x',created_at:100});
});
