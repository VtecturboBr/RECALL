import type { BoardRect, BoardBounds, BoardViewport } from './boardNavigation';
import { clampBoardZoom } from './boardNavigation';

/**
 * Small, testable visibility layer; SQLite keeps every board card. Only the
 * WebView's DOM and embedded assets are windowed. No persistence is changed.
 */
export interface BoardSpatialItem extends BoardRect {
  id: string;
  kind: string;
}
export interface BoardSpatialEdge {
  from_id: string;
  to_id: string;
}
export interface WorldViewportRect {
  left: number;
  top: number;
  right: number;
  bottom: number;
}
export const BOARD_OVERSCAN_PX=280;
export const BOARD_IMAGE_CACHE_LIMIT=20;
export const BOARD_IMAGE_PARALLEL_LIMIT=3;
export const MINIMAP_WIDTH=180;
export const MINIMAP_HEIGHT=112;

export function currentBoardWorldWindow(
  view: BoardViewport,
  screenWidth: number,
  screenHeight: number,
  overscanPx=BOARD_OVERSCAN_PX
): WorldViewportRect {
  const zoom=clampBoardZoom(view.zoom);
  const width=Math.max(1, Number.isFinite(screenWidth)?screenWidth:1);
  const height=Math.max(1, Number.isFinite(screenHeight)?screenHeight:1);
  const pad=Math.max(0,Number.isFinite(overscanPx)?overscanPx:0);
  return {
    left: (-view.panX-pad)/zoom,
    top: (-view.panY-pad)/zoom,
    right: (-view.panX+width+pad)/zoom,
    bottom: (-view.panY+height+pad)/zoom
  };
}

export function intersectsBoardWorldRect(item:BoardRect,view:WorldViewportRect):boolean {
  return item.x <= view.right && item.y <= view.bottom
    && item.x+item.width>=view.left && item.y+item.height>=view.top;
}

export function selectVisibleBoardItems<T extends BoardSpatialItem>(
  items:readonly T[],
  view:BoardViewport,
  viewportWidth:number,
  viewportHeight:number,
  pinnedId:string|null=null,
  overscanPx=BOARD_OVERSCAN_PX
):T[] {
  const region=currentBoardWorldWindow(view,viewportWidth,viewportHeight,overscanPx);
  return items.filter(item=>item.id===pinnedId || intersectsBoardWorldRect(item,region));
}

/** Liang–Barsky clipping: keep an edge if any part crosses the visible area. */
export function boardSegmentCrossesWorld(
  x1:number,y1:number,x2:number,y2:number,box:WorldViewportRect
):boolean {
  let start=0,end=1;
  const dx=x2-x1,dy=y2-y1;
  const p=[-dx,dx,-dy,dy];
  const q=[x1-box.left,box.right-x1,y1-box.top,box.bottom-y1];
  for(let i=0;i<4;i++) {
    if(p[i]===0) { if(q[i]<0)return false;continue; }
    const r=q[i]/p[i];
    if(p[i]<0)start=Math.max(start,r);
    else end=Math.min(end,r);
    if(start>end)return false;
  }
  return true;
}

export function selectVisibleBoardEdges<E extends BoardSpatialEdge>(
  edges:readonly E[],
  items:readonly BoardSpatialItem[],
  view:BoardViewport,
  viewportWidth:number,
  viewportHeight:number,
  overscanPx=BOARD_OVERSCAN_PX
):Array<{edge:E;from:BoardSpatialItem;to:BoardSpatialItem}> {
  const region=currentBoardWorldWindow(view,viewportWidth,viewportHeight,overscanPx);
  const cardMap=new Map(items.map(item=>[item.id,item]));
  const visible:Array<{edge:E;from:BoardSpatialItem;to:BoardSpatialItem}>=[];
  for(const edge of edges) {
    const from=cardMap.get(edge.from_id),to=cardMap.get(edge.to_id);
    if(!from||!to)continue;
    if(boardSegmentCrossesWorld(
      from.x+from.width/2,from.y+from.height/2,
      to.x+to.width/2,to.y+to.height/2,region
    ))visible.push({edge,from,to});
  }
  return visible;
}

export function chooseVisibleImageIds(
  cards:readonly BoardSpatialItem[],
  cached:ReadonlySet<string>,
  loading:ReadonlySet<string>,
  maxCache=BOARD_IMAGE_CACHE_LIMIT,
  maxParallel=BOARD_IMAGE_PARALLEL_LIMIT
):string[] {
  const slots=Math.max(0,Math.min(
    Math.floor(maxCache)-cached.size-loading.size,
    Math.floor(maxParallel)-loading.size
  ));
  if(slots===0)return [];
  const chosen:string[]=[];
  for(const card of cards) {
    if(card.kind==='image'&&!cached.has(card.id)&&!loading.has(card.id)) {
      chosen.push(card.id);
      if(chosen.length>=slots)break;
    }
  }
  return chosen;
}

export function minimapWorldPoint(
  bounds:BoardBounds,
  positionX:number,
  positionY:number,
  canvasWidth=MINIMAP_WIDTH,
  canvasHeight=MINIMAP_HEIGHT
):{x:number;y:number} {
  const normalizedX=Math.max(0,Math.min(canvasWidth,positionX));
  const normalizedY=Math.max(0,Math.min(canvasHeight,positionY));
  return {
    x:bounds.left+normalizedX/Math.max(1,canvasWidth)*bounds.width,
    y:bounds.top+normalizedY/Math.max(1,canvasHeight)*bounds.height
  };
}
