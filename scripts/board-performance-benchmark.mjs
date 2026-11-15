/**
 * Non-flaky synthetic RECALL board benchmark. Records metrics, not arbitrary
 * timing-based pass/fail gates; Windows CI hardware varies between runners.
 * These numbers are NOT production WebView/SQLite RAM or FPS measurements.
 */
import { performance } from 'node:perf_hooks';
import assert from 'node:assert/strict';
import { selectVisibleBoardItems,selectVisibleBoardEdges } from '../src/lib/boardPerformance.ts';
import { mergeBoardPage,BOARD_PAGE_BATCH_SIZE } from '../src/lib/boardPaging.ts';
import { assertConsistentBoardLoad } from '../src/lib/boardReadIntegrity.ts';

const CARD_COUNT=50_000;
const before=process.memoryUsage().heapUsed;
const constructionStart=performance.now();
const cards=Array.from({length:CARD_COUNT},(_,i)=>({
  id:'card-'+String(i).padStart(7,'0'),
  kind:i%20===0?'image':'note',
  x:(i%250)*400,y:Math.floor(i/250)*320,
  width:280,height:210,
  content:'ideia '+i,
  created_at:1730000000000,
  revision:0
}));
const creationMs=performance.now()-constructionStart;
const edges=Array.from({length:10_000},(_,i)=>({
  from_id:cards[i].id,to_id:cards[i+1].id
}));
const regionStart=performance.now();
const visible=selectVisibleBoardItems(cards,{panX:0,panY:0,zoom:1},1280,720);
const lines=selectVisibleBoardEdges(edges,cards,{panX:0,panY:0,zoom:1},1280,720);
const viewportMs=performance.now()-regionStart;
assert.ok(visible.length>0&&visible.length<250,'DOM should not include all cards');
assert.ok(lines.length<edges.length,'offscreen lines must be clipped');
const pagesStart=performance.now();
const merged=[];
let cursor=null;
for(let i=0;i<cards.length;i+=BOARD_PAGE_BATCH_SIZE) {
  const chunk=cards.slice(i,i+BOARD_PAGE_BATCH_SIZE);
  const hasNext=i+BOARD_PAGE_BATCH_SIZE<cards.length;
  cursor=mergeBoardPage(merged,{
    items:chunk,
    next_cursor:hasNext?{
      created_at:chunk.at(-1).created_at,id:chunk.at(-1).id
    }:null
  },cursor);
}
assert.equal(merged.length,CARD_COUNT);
const pagedMs=performance.now()-pagesStart;
const stamp={card_count:CARD_COUNT,revision_total:0,rowid_total:1250025000,content_bytes_total:500000};
assertConsistentBoardLoad(stamp,{...stamp},merged.length);
const used=process.memoryUsage().heapUsed-before;
console.log('RECALL board synthetic benchmark:',JSON.stringify({
  cards:CARD_COUNT,connections:edges.length,
  visibleCards:visible.length,visibleConnections:lines.length,
  creationMs:Number(creationMs.toFixed(2)),
  visibilityMs:Number(viewportMs.toFixed(2)),
  pageMergeMs:Number(pagedMs.toFixed(2)),
  heapDeltaMiB:Number((used/1048576).toFixed(2)),
  caveat:'Synthetic Node measurements, not WebView rendering or real user SQLite data'
}));
