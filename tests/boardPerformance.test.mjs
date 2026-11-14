import test from 'node:test';
import assert from 'node:assert/strict';
import {
 currentBoardWorldWindow,selectVisibleBoardItems,selectVisibleBoardEdges,
 boardSegmentCrossesWorld,chooseVisibleImageIds,minimapWorldPoint,
 BOARD_IMAGE_CACHE_LIMIT,MINIMAP_HEIGHT,MINIMAP_WIDTH
} from '../src/lib/boardPerformance.ts';

const viewport={panX:0,panY:0,zoom:1};
const note=(id,x,y,width=260,height=180)=>({id,kind:'note',x,y,width,height,content:id});

test('overscan includes near-edge cards and drops far-offscreen cards',()=>{
  const cards=[note('onscreen',10,10),note('edge',-450,-200),note('far',-5000,-5000)];
  assert.deepEqual(
    selectVisibleBoardItems(cards,viewport,800,600,null,220).map(c=>c.id),
    ['onscreen','edge']
  );
  assert.deepEqual(
    selectVisibleBoardItems(cards,viewport,800,600,'far',220).map(c=>c.id),
    ['onscreen','edge','far']
  );
});
test('zoom, panning and negative positions do not crop visible cards',()=>{
  const zoomed={zoom:0.5,panX:900,panY:700};
  const card=note('negative',-1700,-1200);
  assert.ok(selectVisibleBoardItems([card],zoomed,800,600).length===1);
  const r=currentBoardWorldWindow(zoomed,800,600,0);
  assert.ok(r.left<0 && r.right<0);
  assert.ok(r.top<0);
});
test('thousands of stored items do not produce thousands of DOM cards',()=>{
  const cards=Array.from({length:5000},(_,i)=>note('card'+i,(i%100)*450,Math.floor(i/100)*450));
  const selected=selectVisibleBoardItems(cards,viewport,1000,650,null,100);
  assert.ok(selected.length>0);
  assert.ok(selected.length<40, 'visibility should not render the whole database');
  assert.equal(cards.length,5000,'never remove cards from authoritative local collection');
});
test('edges are clipped, including crossings with both endpoints offscreen',()=>{
  const cards=[
    note('left',-1500,200),
    note('right',1500,200),
    note('veryfar',8000,5000),
    note('upper',200,-1500),
    note('lower',200,1600)
  ];
  const edges=[
    {id:'horizontal',from_id:'left',to_id:'right'},
    {id:'vertical',from_id:'upper',to_id:'lower'},
    {id:'outside',from_id:'right',to_id:'veryfar'},
    {id:'missing',from_id:'nonexistent',to_id:'left'}
  ];
  const result=selectVisibleBoardEdges(edges,cards,viewport,800,600,0);
  assert.deepEqual(result.map(x=>x.edge.id),['horizontal','vertical']);
  assert.equal(boardSegmentCrossesWorld(-1,10,1000,10,{left:0,top:0,right:800,bottom:600}),true);
  assert.equal(boardSegmentCrossesWorld(900,900,1000,1000,{left:0,top:0,right:800,bottom:600}),false);
});
test('image requests remain bounded, never re-request cache or concurrent ids',()=>{
  const cards=Array.from({length:90},(_,i)=>({...note('pic'+i,i*5,0),kind:'image'}));
  const first=chooseVisibleImageIds(cards,new Set(),new Set());
  assert.equal(first.length,3);
  const next=chooseVisibleImageIds(cards,new Set(['pic0','pic1']),new Set(['pic2']));
  assert.deepEqual(next,['pic3','pic4']);
  const full=new Set(cards.slice(0,BOARD_IMAGE_CACHE_LIMIT).map(i=>i.id));
  assert.deepEqual(chooseVisibleImageIds(cards,full,new Set()),[]);
});
test('minimap coordinate mapping is stable and clamps out-of-range clicks',()=>{
  const bounds={left:-2000,top:-1200,width:4000,height:2400};
  assert.deepEqual(minimapWorldPoint(bounds,90,56),{x:0,y:0});
  assert.deepEqual(minimapWorldPoint(bounds,-100,-20),{x:-2000,y:-1200});
  assert.deepEqual(minimapWorldPoint(bounds,1e6,1e6),{x:2000,y:1200});
  assert.equal(MINIMAP_WIDTH,180);
  assert.equal(MINIMAP_HEIGHT,112);
});
