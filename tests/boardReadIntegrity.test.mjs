import test from 'node:test';
import assert from 'node:assert/strict';
import {
  sameBoardReadStamp,isValidBoardReadStamp,assertConsistentBoardLoad
} from '../src/lib/boardReadIntegrity.ts';

const start={card_count:4,revision_total:0,rowid_total:31,content_bytes_total:30};
test('accepts complete and stable pagination',()=>{
  assertConsistentBoardLoad(start,{...start},4);
  assert.equal(sameBoardReadStamp(start,{...start}),true);
});
test('rejects concurrent update without changed card count',()=>{
  assert.throws(()=>assertConsistentBoardLoad(start,
    {...start,revision_total:1},4),/QUADRO_ALTERADO_DURANTE_LEITURA/);
  assert.throws(()=>assertConsistentBoardLoad(start,
    {...start,content_bytes_total:45},4),/QUADRO_ALTERADO_DURANTE_LEITURA/);
});
test('detects additions, deletions, row replacement and incomplete pages',()=>{
  for(const patch of [
    {card_count:5},{card_count:3},{rowid_total:32}
  ])assert.throws(()=>assertConsistentBoardLoad(start,{...start,...patch},4));
  assert.throws(()=>assertConsistentBoardLoad(start,{...start},3));
  assert.throws(()=>assertConsistentBoardLoad(start,{...start},5));
});
test('invalid, unsafe or corrupt counters fail closed',()=>{
  for(const value of [
    null,undefined,{...start,card_count:NaN},{...start,rowid_total:-1},
    {...start,revision_total:1.5},{...start,content_bytes_total:'30'},
    {...start,rowid_total:Number.MAX_SAFE_INTEGER+1}
  ]) {
    assert.equal(isValidBoardReadStamp(value),false);
    assert.equal(sameBoardReadStamp(start,value),false);
  }
});
test('empty board has a stable, validated stamp',()=>{
  assertConsistentBoardLoad({
    card_count:0,revision_total:0,rowid_total:0,content_bytes_total:0
  },{
    card_count:0,revision_total:0,rowid_total:0,content_bytes_total:0
  },0);
});
