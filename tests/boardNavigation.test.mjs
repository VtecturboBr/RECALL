import test from 'node:test';
import assert from 'node:assert/strict';
import {
  boardBounds, fitBoardView, focusBoardItem,
  saveBoardView, restoreBoardView,
  searchBoardCards, miniRect, visibleWorldRect,
  clampBoardZoom
} from '../src/lib/boardNavigation.ts';

const cards = [
  {id:'n1',kind:'note',content:'Reunião do projeto',x:0,y:0,width:280,height:210},
  {id:'t1',kind:'note',content:'',x:900,y:350,width:330,height:210},
  {id:'l1',kind:'link',content:'https://example.com',x:-600,y:-200,width:300,height:140},
];

test('bounds include negative coordinates and card extents',()=>{
  const b=boardBounds(cards,0);
  assert.deepEqual(b,{left:-600,top:-200,width:1830,height:760});
});
test('fit view centers every card and avoids non-finite geometry',()=>{
  const view=fitBoardView(cards,900,500);
  assert.ok(view.zoom>=0.1 && view.zoom<=2.2);
  const bounds=boardBounds(cards,0);
  const cx=(bounds.left+bounds.width/2)*view.zoom+view.panX;
  const cy=(bounds.top+bounds.height/2)*view.zoom+view.panY;
  assert.ok(Math.abs(cx-450)<0.0001);
  assert.ok(Math.abs(cy-250)<0.0001);
  assert.deepEqual(fitBoardView([],900,500),{panX:0,panY:0,zoom:1});
});
test('focus card moves its center to the viewport center',()=>{
  const view=focusBoardItem(cards[1],1000,600,0.2);
  assert.equal(view.zoom,0.9);
  assert.ok(Math.abs((cards[1].x+cards[1].width/2)*view.zoom+view.panX-500)<1e-6);
  assert.ok(Math.abs((cards[1].y+cards[1].height/2)*view.zoom+view.panY-300)<1e-6);
});
test('view persists a world-space center even if viewport size changes',()=>{
  const current={panX:-750,panY:200,zoom:1.2};
  const data=saveBoardView(current,1000,600);
  const restored=restoreBoardView(data,1600,900);
  assert.ok(restored);
  assert.equal(restored.zoom,current.zoom);
  const savedAgain=saveBoardView(restored,1600,900);
  assert.deepEqual(savedAgain,data);
});
test('malformed view data cannot inject invalid pan or zoom',()=>{
  assert.equal(restoreBoardView({centerX:Infinity,centerY:0,zoom:1},900,600),null);
  assert.equal(restoreBoardView({centerX:0,centerY:0,zoom:-2},900,600),null);
  assert.equal(restoreBoardView({centerX:1e40,centerY:0,zoom:1},900,600),null);
  assert.equal(restoreBoardView({centerX:0,centerY:0,zoom:'1'},900,600),null);
  assert.equal(clampBoardZoom(Number.NaN),1);
});
test('search normalizes accents, includes live references, hides deleted content',()=>{
  const linked={
    t1:{title:'Próxima ação',preview:'Corrigir o bug',available:true}
  };
  assert.deepEqual(searchBoardCards(cards,linked,'reuniao').map(x=>x.id),['n1']);
  assert.deepEqual(searchBoardCards(cards,linked,'PROXIMA').map(x=>x.id),['t1']);
  linked.t1.available=false;
  assert.deepEqual(searchBoardCards(cards,linked,'Próxima').map(x=>x.id),[]);
  assert.deepEqual(searchBoardCards(cards,linked,'https').map(x=>x.id),['l1']);
  assert.deepEqual(searchBoardCards(cards,linked,'').map(x=>x.id),[]);
});
test('minimap mapping and viewport rect are coherent',()=>{
  const b=boardBounds(cards,0);
  const mini=miniRect(cards[0],b,180,120);
  assert.ok(mini.x>=0 && mini.x<180 && mini.y>=0 && mini.y<120);
  assert.equal(mini.width,280*180/b.width);
  const world=visibleWorldRect({panX:-200,panY:100,zoom:2},800,400);
  assert.deepEqual(world,{x:100,y:-50,width:400,height:200});
});
