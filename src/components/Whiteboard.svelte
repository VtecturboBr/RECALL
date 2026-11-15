<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { StickyNote, ImagePlus, Link2, Table2, ZoomIn, ZoomOut, RotateCcw, Undo2, Redo2, Trash2, Move, Plus, ExternalLink, Pencil, Unlink2, Database, FileText, ListTodo, Search, X, CheckCircle2 } from 'lucide-svelte';
  import { boardApi } from '../lib/api';
  import { BOARD_PAGE_BATCH_SIZE, mergeBoardPage } from '../lib/boardPaging';
  import { assertConsistentBoardLoad } from '../lib/boardReadIntegrity';
  import type { BoardPageCursor } from '../lib/types';
  import {
    selectVisibleBoardItems, selectVisibleBoardEdges,
    chooseVisibleImageIds, minimapWorldPoint,
    BOARD_IMAGE_CACHE_LIMIT, MINIMAP_WIDTH, MINIMAP_HEIGHT
  } from '../lib/boardPerformance';
  import {
    emptyBoardHistory, recordBoardHistory, createBoardHistoryAction,
    topBoardHistory, canApplyBoardHistory, historyDestination,
    completeBoardHistory, forgetBoardCardHistory, sameBoardFields,
    type BoardHistoryState, type BoardHistoryDirection
  } from '../lib/boardHistory';
  import { saveOwnedDraft, discardOwnedDraft, isOwnedOrLegacyDraft, currentDraftOwner } from '../lib/draftOwnership';
  import {
    boardDraftState, parseBoardDraft, listBoardRecovery, sameBoardDraftFields,
    shouldDiscardExactBoardDraft, type BoardRecoveryEntry
  } from '../lib/boardRecovery';
  import {
    boardBounds, fitBoardView, focusBoardItem, saveBoardView,
    restoreBoardView, searchBoardCards, miniRect, visibleWorldRect,
    clampBoardZoom
  } from '../lib/boardNavigation';
  import type { BoardItem, BoardKind, Whiteboard, BoardConnection, BoardReference, BoardReferenceTarget } from '../lib/types';

  let { onOpenReference }: { onOpenReference?: (kind: 'note' | 'task', id: string) => void | Promise<void> } = $props();

  // Every card edit replaces the array instead of mutating nested objects.
  // Avoid creating a deep reactive Proxy for thousands of SQLite rows.
  let items = $state.raw<BoardItem[]>([]);
  let boards = $state<Whiteboard[]>([]);
  let references = $state.raw<Record<string, BoardReference>>({});
  let pickerOpen = $state(false);
  let referenceQuery = $state('');
  let referenceTargets = $state<BoardReferenceTarget[]>([]);
  let referenceSearching = $state(false);
  let referenceSaving = $state(false);
  let referenceSearchRequest = 0;
  let pickerTimer: ReturnType<typeof setTimeout> | null = null;
  let activeBoardId = $state('default');
  let connections = $state.raw<BoardConnection[]>([]);
  let connectFrom = $state<string | null>(null);
  let switching = $state(false);
  let connecting = $state(false);
  let boardRequest = 0;
  let assets = $state.raw<Record<string, string>>({});
  let loading = $state(true);
  let loadingCount = $state(0);
  let boardLoadFailed = $state(false);
  let busy = $state(false);
  let error = $state('');
  let message = $state('');
  let recoveryOpen = $state(false);
  let recoveryBusy = $state(false);
  let recoveries = $state<BoardRecoveryEntry[]>([]);
  let history = $state<BoardHistoryState>(emptyBoardHistory());
  let historyBusy = $state(false);
  // Only revisions confirmed by SQLite can be used for undo and redo.
  const confirmed = new Map<string,BoardItem>();
  let selected = $state<string | null>(null);
  let zoom = $state(1);
  let panX = $state(0);
  let panY = $state(0);
  let stage: HTMLDivElement;
  let imageInput: HTMLInputElement;
  let viewportWidth = $state(850);
  let viewportHeight = $state(520);
  let searchOpen = $state(false);
  let searchQuery = $state('');
  let minimapOpen = $state(true);
  let minimapCanvas = $state<HTMLCanvasElement | undefined>(undefined);
  let viewTimer: ReturnType<typeof setTimeout> | null = null;
  const foundCards = $derived(searchBoardCards(items,references,searchQuery,40));
  const overviewBounds = $derived(boardBounds(items,160));
  const currentMiniWindow = $derived(miniRect(
    visibleWorldRect({panX,panY,zoom},viewportWidth,viewportHeight),
    overviewBounds,MINIMAP_WIDTH,MINIMAP_HEIGHT
  ));
  const visibleItems = $derived(selectVisibleBoardItems(
    items,{panX,panY,zoom},viewportWidth,viewportHeight,selected
  ));
  const visibleEdges = $derived(selectVisibleBoardEdges(
    connections,items,{panX,panY,zoom},viewportWidth,viewportHeight
  ));
  // One canvas replaces thousands of clickable DOM elements in the minimap.
  $effect(() => {
    const canvas = minimapCanvas;
    const cards = items;
    const bounds = overviewBounds;
    const windowRect = currentMiniWindow;
    const selectedId = selected;
    if (!minimapOpen || !canvas || !cards.length) return;
    const frame=requestAnimationFrame(() => {
      const ctx=canvas.getContext('2d');
      if (!ctx) return;
      ctx.setTransform(2,0,0,2,0,0);
      ctx.clearRect(0,0,MINIMAP_WIDTH,MINIMAP_HEIGHT);
      ctx.fillStyle='#9c526b';
      for (const item of cards) {
        const rect=miniRect(item,bounds,MINIMAP_WIDTH,MINIMAP_HEIGHT);
        ctx.fillStyle=item.id===selectedId?'#ff6c9d':'#94516b';
        ctx.fillRect(rect.x,rect.y,Math.max(2,rect.width),Math.max(2,rect.height));
      }
      ctx.strokeStyle='#fff3f7';
      ctx.lineWidth=1.5;
      ctx.strokeRect(
        windowRect.x,windowRect.y,
        Math.max(0,windowRect.width),Math.max(0,windowRect.height)
      );
    });
    return () => cancelAnimationFrame(frame);
  });

  const imageInFlight = new Set<string>();
  const failedImageIds = new Set<string>();
  function hydrateVisibleImages(candidates: BoardItem[]) {
    if (loading || switching) return;
    const wanted=new Set(candidates.filter(item=>item.kind==='image').map(item=>item.id));
    // Release large base64 data URLs as soon as their image leaves the
    // overscanned viewport. This does not delete image bytes in SQLite.
    const stored=Object.keys(assets);
    const retained:Record<string,string>={};
    for (const id of stored) if (wanted.has(id)) retained[id]=assets[id];
    if (Object.keys(retained).length!==stored.length) assets=retained;
    const remaining=candidates.filter(item=>!failedImageIds.has(item.id));
    const requests=chooseVisibleImageIds(
      remaining,new Set(Object.keys(assets)),imageInFlight
    );
    for (const id of requests) {
      imageInFlight.add(id);
      const request=boardRequest;
      void boardApi.asset(id).then(dataUrl => {
        if (request !== boardRequest || !dataUrl
            || !visibleItems.some(item=>item.id===id)) return;
        if (!assets[id] && Object.keys(assets).length < BOARD_IMAGE_CACHE_LIMIT) {
          assets={...assets,[id]:dataUrl};
        }
      }).catch(() => {
        if (request===boardRequest) {
          failedImageIds.add(id);
          message='Uma imagem visível não pôde ser carregada. Reabra o quadro para tentar novamente.';
        }
      }).finally(() => {
        imageInFlight.delete(id);
        if (request===boardRequest) hydrateVisibleImages(visibleItems);
      });
    }
  }
  $effect(() => {
    const candidates=visibleItems;
    // Cache mutation is a dependency; free slots trigger the next batch.
    const count=Object.keys(assets).length;
    if (!loading && !switching && count>=0) hydrateVisibleImages(candidates);
  });

  const dirty = new Set<string>();
  const timers = new Map<string, ReturnType<typeof setTimeout>>();
  const flights = new Map<string, Promise<void>>();
  type Gesture = { kind: 'pan' | 'move' | 'resize'; pointer: number; startX: number; startY: number; x: number; y: number; width: number; height: number; id?: string };
  let gesture: Gesture | null = null;
  function navigateMinimap(event: MouseEvent) {
    if (switching || loading || historyBusy) return;
    const button=event.currentTarget as HTMLButtonElement | null;
    if (!button) return;
    const bounds=button.getBoundingClientRect();
    if (event.detail===0) { fitAll(); return; } // keyboard activation
    const clickX=(event.clientX-bounds.left)*MINIMAP_WIDTH/Math.max(1,bounds.width);
    const clickY=(event.clientY-bounds.top)*MINIMAP_HEIGHT/Math.max(1,bounds.height);
    const point=minimapWorldPoint(overviewBounds,clickX,clickY);
    panX=viewportWidth/2-point.x*zoom;
    panY=viewportHeight/2-point.y*zoom;
    scheduleViewSave();
  }
  const draftKey = (id: string) => 'recall-board-draft:' + id;
  const viewKey = (id: string) => 'recall-board-viewport-v1:' + id;
  function persistView(id = activeBoardId) {
    if (!id || !stage) return;
    try {
      localStorage.setItem(viewKey(id), JSON.stringify(
        saveBoardView({panX,panY,zoom},viewportWidth,viewportHeight)
      ));
    } catch { /* View state is optional; never interrupt SQLite content saves. */ }
  }
  function scheduleViewSave() {
    if (viewTimer !== null) clearTimeout(viewTimer);
    viewTimer = setTimeout(() => {
      viewTimer = null;
      persistView();
    }, 320);
  }
  function restoreView(id: string) {
    try {
      const raw = localStorage.getItem(viewKey(id));
      const restored = raw ? restoreBoardView(JSON.parse(raw),viewportWidth,viewportHeight) : null;
      if (restored) {
        panX = restored.panX;
        panY = restored.panY;
        zoom = restored.zoom;
        return;
      }
    } catch { /* Ignore malformed or unavailable view state only. */ }
    panX = 0; panY = 0; zoom = 1;
  }
  function labelForCard(item: BoardItem): string {
    const linked = references[item.id];
    if (linked) return linked.available ? linked.title : 'Referência indisponível';
    if (item.kind==='image') return item.content || 'Imagem';
    if (item.kind==='table') {
      try {
        const parsed: unknown = JSON.parse(item.content);
        if (Array.isArray(parsed)) return parsed.flat().map(v => String(v)).join(' ').slice(0,120) || 'Tabela';
      } catch { /* older table content */ }
      return 'Tabela';
    }
    return item.content.trim().slice(0,120) || 'Cartão sem texto';
  }
  function focusCard(id: string) {
    if (switching || loading) return;
    const item = findItem(id);
    if (!item || !stage) return;
    const view = focusBoardItem(item,viewportWidth,viewportHeight,zoom);
    panX = view.panX; panY = view.panY; zoom = view.zoom;
    selected = id;
    scheduleViewSave();
  }
  function fitAll() {
    if (switching || loading) return;
    const view = fitBoardView(items,viewportWidth,viewportHeight);
    panX=view.panX; panY=view.panY; zoom=view.zoom;
    scheduleViewSave();
  }
  function closeBoardSearch() { searchOpen=false; searchQuery=''; }
  function boardShortcut(event: KeyboardEvent) {
    if (switching || loading || historyBusy) return;
    const target=event.target;
    if (pickerOpen || recoveryOpen || (target instanceof HTMLElement &&
      target.closest('input,textarea,select,[contenteditable="true"],button'))) return;
    // Preserve native browser text undo in editable fields.
    if ((event.ctrlKey || event.metaKey) && !event.altKey &&
        (event.key.toLowerCase()==='z' || event.key.toLowerCase()==='y')) {
      event.preventDefault();
      const direction: BoardHistoryDirection =
        event.key.toLowerCase()==='y' || event.shiftKey ? 'redo' : 'undo';
      void performHistory(direction);
      return;
    }
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase()==='f') {
      event.preventDefault();
      searchOpen=true;
      return;
    }
    if (event.key==='Escape') {
      connectFrom=null; selected=null;
      closeBoardSearch();
    } else if (event.key.toLowerCase()==='f' && !event.altKey && !event.ctrlKey && !event.metaKey) {
      event.preventDefault(); fitAll();
    }
  }

  const findItem = (id: string) => items.find(item => item.id === id);
  async function performHistory(direction: BoardHistoryDirection) {
    if (historyBusy || loading || switching || busy || recoveryBusy || referenceSaving || gesture) return;
    historyBusy=true;
    try {
      // Never undo against an unsaved draft; flush it first, preserving
      // recovery snapshots on failure.
      if (!(await finishPending())) {
        error='Há alterações não salvas. O histórico foi interrompido; o rascunho está preservado.';
        return;
      }
      const action=topBoardHistory(history,direction);
      if (!action) return;
      const local=findItem(action.itemId);
      const authoritative=confirmed.get(action.itemId);
      if (!authoritative || !local || !sameBoardFields(local,authoritative)
          || !canApplyBoardHistory(action,direction,activeBoardId,authoritative)) {
        history=emptyBoardHistory();
        error='O histórico não corresponde à versão atual do cartão. Operação cancelada; reabra o quadro para atualizar.';
        return;
      }
      const destination=historyDestination(action,direction);
      // boardApi.update sends expectedRevision=authoritative.revision;
      // another window's concurrent update cannot be overwritten.
      const updated=await boardApi.update({...authoritative,...destination});
      confirmed.set(updated.id,updated);
      items=items.map(item=>item.id===updated.id ? updated : item);
      history=completeBoardHistory(history,direction);
      selected=updated.id;
      message=(direction==='undo'?'Desfeito: ':'Refeito: ')+action.label+'.';
      error='';
      scanBoardRecoveries();
    } catch(e) {
      if (String(e).includes('CONFLITO_QUADRO')) {
        history=emptyBoardHistory();
        error='Outra janela alterou o cartão. Desfazer/Refazer foi bloqueado; reabra o quadro para atualizar.';
      } else {
        error='Não foi possível aplicar o histórico. Os dados foram preservados: '+String(e);
      }
    } finally {
      historyBusy=false;
    }
  }
  function sameFields(a: BoardItem, b: BoardItem): boolean {
    return a.x === b.x && a.y === b.y && a.width === b.width
      && a.height === b.height && a.content === b.content;
  }
  function scanBoardRecoveries(boardId = activeBoardId) {
    try {
      const entries: {key: string; raw: string}[] = [];
      for (let index=0; index<localStorage.length; index++) {
        const key = localStorage.key(index);
        if (!key || !(key.startsWith('recall-board-draft:') ||
          key.startsWith('recall-draft-recovery:recall-board-draft:'))) continue;
        const raw = localStorage.getItem(key);
        if (raw!==null) entries.push({key,raw});
      }
      recoveries = listBoardRecovery(entries,boardId,items,100,dirty,currentDraftOwner());
    } catch {
      // Storage can be disabled. SQLite is authoritative; do not claim data recovered.
      message = 'O armazenamento local de recuperação não está disponível nesta janela.';
    }
  }
  function keepDraft(item: BoardItem) {
    try {
      saveOwnedDraft(draftKey(item.id), {
        boardId:item.board_id, kind:item.kind,
        content:item.content, x:item.x,y:item.y,
        width:item.width,height:item.height,revision:item.revision
      });
    } catch { message = 'O rascunho local não pôde ser gravado. Evite fechar antes de salvar.'; }
  }
  function clearDraftIfSame(item: BoardItem, expectedRaw: string | null) {
    try {
      const key = draftKey(item.id);
      const raw = localStorage.getItem(key);
      if (!shouldDiscardExactBoardDraft(raw, expectedRaw) || !raw) return;
      const snapshot = parseBoardDraft(raw,item);
      if (snapshot && sameBoardDraftFields(snapshot,item)) {
        discardOwnedDraft(key);
      }
    } catch { /* Never delete a damaged recovery snapshot. */ }
  }
  function recoveryDescription(entry: BoardRecoveryEntry): string {
    if (entry.status==='conflict') return 'O banco possui uma revisão mais recente. Nenhum texto foi sobrescrito.';
    if (entry.status==='competing') return 'Rascunho criado em outra janela ou cópia preservada de um conflito.';
    if (entry.status==='orphan') return 'O cartão original não aparece mais neste quadro.';
    if (entry.status==='pending') return 'Alteração ainda não confirmada pelo banco SQLite.';
    return 'O registro não pode ser lido automaticamente; o texto original está preservado.';
  }
  async function copyRecovery(entry: BoardRecoveryEntry) {
    const snap=entry.snapshot;
    if (!snap || snap.kind==='image' || references[entry.itemId]
        || loading || switching || recoveryBusy) return;
    recoveryBusy = true;
    try {
      const fresh = await boardApi.create(
        activeBoardId,snap.kind,
        Math.max(-9_999_900,Math.min(9_999_900,snap.x+48)),
        Math.max(-9_999_900,Math.min(9_999_900,snap.y+48)),
        snap.content
      );
      items = [...items,fresh];
      confirmed.set(fresh.id,fresh);
      history={...history,redo:[]};
      selected=fresh.id;
      message='Cópia recuperada em um novo cartão. O cartão e o rascunho originais foram preservados.';
      recoveryOpen=false;
      scanBoardRecoveries();
      focusCard(fresh.id);
    } catch(e) { error='Não foi possível criar a cópia de recuperação: '+String(e); }
    finally { recoveryBusy=false; }
  }
  function discardRecovery(entry: BoardRecoveryEntry) {
    if (dirty.has(entry.itemId) || flights.has(entry.itemId)) {
      error='O cartão ainda possui mudanças pendentes. Salve ou crie uma cópia antes de descartar.';
      return;
    }
    if (!window.confirm('Descartar apenas esta cópia local? Esta ação não altera os dados do SQLite.')) return;
    try {
      if (shouldDiscardExactBoardDraft(localStorage.getItem(entry.key),entry.raw)) {
        localStorage.removeItem(entry.key);
        scanBoardRecoveries();
        message='Cópia de recuperação local descartada. Os dados SQLite não foram alterados.';
      } else {
        error='A cópia foi modificada em outra janela e não foi descartada.';
        scanBoardRecoveries();
      }
    } catch(e) { error='Não foi possível descartar a cópia: '+String(e); }
  }
  function updateLocal(id: string, changes: Partial<BoardItem>, commit = true) {
    if (switching || loading || historyBusy) return;
    items = items.map(item => item.id === id ? { ...item, ...changes } : item);
    const current = findItem(id);
    if (!current) return;
    if (commit) {
      dirty.add(id);
      keepDraft(current);
      scheduleSave(id);
    }
  }
  function scheduleSave(id: string, delay = 550) {
    const previous = timers.get(id);
    if (previous !== undefined) clearTimeout(previous);
    timers.set(id, setTimeout(() => {
      timers.delete(id);
      void persist(id);
    }, delay));
  }
  async function persist(id: string): Promise<void> {
    if (flights.has(id)) return flights.get(id);
    const value = findItem(id);
    if (!value || !dirty.has(id)) return;
    const previous = timers.get(id);
    if (previous !== undefined) clearTimeout(previous);
    timers.delete(id);
    const snapshot = { ...value };
    let originalDraftRaw: string | null = null;
    try { originalDraftRaw = localStorage.getItem(draftKey(id)); } catch { /* optional cache */ }
    dirty.delete(id);
    const flight = (async () => {
      try {
        const before=confirmed.get(id);
        const saved=await boardApi.update(snapshot);
        confirmed.set(id,saved);
        if (before) {
          const action=createBoardHistoryAction(before,saved);
          if (action) history=recordBoardHistory(history,action);
        }
        const latest = findItem(id);
        if (!latest) return;
        const unchanged = sameFields(latest, snapshot);
        items = items.map(item => item.id === id ? { ...item, revision: saved.revision, updated_at: saved.updated_at } : item);
        if (unchanged && !dirty.has(id)) clearDraftIfSame(snapshot,originalDraftRaw);
        else {
          dirty.add(id);
          const current = findItem(id);
          if (current) keepDraft(current);
          scheduleSave(id, 250);
        }
        error = '';
        if (recoveryOpen) scanBoardRecoveries();
      } catch (e) {
        dirty.add(id);
        error = String(e).includes('CONFLITO_QUADRO')
          ? 'Esse cartão foi alterado em outra janela. Seu rascunho local foi preservado; não o apague.'
          : 'Falha ao salvar cartão. Rascunho local preservado: ' + String(e);
        scanBoardRecoveries();
      } finally {
        flights.delete(id);
      }
    })();
    flights.set(id, flight);
    return flight;
  }

  const BOARD_KEY = 'recall-board-selected-v1';
  async function loadBoard(id: string) {
    const request = ++boardRequest;
    loading = true;
    boardLoadFailed = false;
    loadingCount = 0;
    try {
      // Stream small IPC responses. Keep the previous board intact until
      // *every* page succeeds; a partial load must never look like deletion.
      const [initialStamp,edges,linked] = await Promise.all([
        boardApi.readStamp(id),
        boardApi.connections(id),
        boardApi.references(id)
      ]);
      if (request !== boardRequest) return;
      const first=await boardApi.listPage(id,null,BOARD_PAGE_BATCH_SIZE);
      if (request !== boardRequest) return;
      const loaded:BoardItem[] = [];
      let page=first;
      let previousCursor:BoardPageCursor | null=null;
      let batchNumber=0;
      while (true) {
        if (request !== boardRequest) return;
        const next:BoardPageCursor | null=mergeBoardPage(
          loaded,page,previousCursor,BOARD_PAGE_BATCH_SIZE
        );
        loadingCount=loaded.length;
        if (!next) break;
        previousCursor=next;
        batchNumber++;
        if (batchNumber%4===0) {
          // Yield to the WebView to paint progress and process input;
          // unlike rAF this also works while Windows is minimised.
          await new Promise<void>(resolve=>setTimeout(resolve,0));
        }
        page=await boardApi.listPage(id,next,BOARD_PAGE_BATCH_SIZE);
      }
      if (request !== boardRequest) return;
      // Fail closed when another WebView changed the active board during
      // pagination. Never publish a mixture of revisions as a complete list.
      const finalStamp=await boardApi.readStamp(id);
      if (request !== boardRequest) return;
      assertConsistentBoardLoad(initialStamp,finalStamp,loaded.length);
      for (const handle of timers.values()) clearTimeout(handle);
      timers.clear();
      dirty.clear();
      confirmed.clear();
      for (const item of loaded) confirmed.set(item.id,item);
      history=emptyBoardHistory();
      items = loaded.map(item => {
        try {
          const raw = localStorage.getItem(draftKey(item.id));
          if (!raw) return item;
          const state = boardDraftState(raw,item);
          if (state==='conflict' || state==='invalid') {
            message = 'Há um rascunho recuperável. A versão do SQLite não foi sobrescrita.';
            return item;
          }
          if (state!=='restore') return item;
          if (!isOwnedOrLegacyDraft(raw)) {
            message='Um rascunho de outra janela foi preservado para recuperação manual.';
            return item;
          }
          const draft = parseBoardDraft(raw,item);
          if (!draft) return item;
          const restored = { ...item, content:draft.content,x:draft.x,y:draft.y,
            width:draft.width,height:draft.height };
          dirty.add(item.id);
          scheduleSave(item.id);
          return restored;
        } catch { return item; }
      });
      connections = edges;
      references = Object.fromEntries(linked.map(ref => [ref.item_id, ref]));
      scanBoardRecoveries(id);
      pickerOpen = false;
      selected = null;
      connectFrom = null;
      assets = {};
      failedImageIds.clear();
      restoreView(id);
      searchQuery = '';
      // Visible image data is fetched later, once the viewport is ready.
      // Hidden image blobs are never loaded just to open the board.
    } catch (e) {
      if (request === boardRequest) {
        boardLoadFailed = true;
        error = 'Não foi possível abrir o Quadro. Nenhuma alteração parcial foi aplicada: ' + String(e);
      }
      throw e;
    } finally {
      if (request === boardRequest) {
        // Keep content controls blocked if any page failed.
        loading = boardLoadFailed;
        loadingCount = 0;
      }
    }
  }
  async function load() {
    try {
      const available = await boardApi.boards();
      if (!available.length) throw new Error('Nenhum quadro foi encontrado no banco SQLite.');
      boards = available;
      let wanted: string | null = null;
      try { wanted = localStorage.getItem(BOARD_KEY); } catch { /* storage optional */ }
      activeBoardId = available.find(board=>board.id===wanted)?.id ?? available[0].id;
      await loadBoard(activeBoardId);
    } catch (e) {
      boardLoadFailed = true;
      error = 'Não foi possível carregar os quadros: ' + String(e);
      loading = true;
    }
  }
  async function finishPending(): Promise<boolean> {
    for (let round=0;round<4;round++) {
      const pending = [...new Set([...dirty,...flights.keys()])];
      if (pending.length===0) return true;
      await Promise.all(pending.map(id=>persist(id)));
    }
    return dirty.size===0 && flights.size===0;
  }
  async function switchBoard(id: string): Promise<void> {
    if (id===activeBoardId || switching || busy || loading || historyBusy) return;
    switching = true;
    try {
      if (!(await finishPending())) {
        error = 'Existem cartões não salvos. Resolva o erro antes de trocar de quadro.';
        return;
      }
      persistView(activeBoardId);
      await loadBoard(id);
      activeBoardId = id;
      try { localStorage.setItem(BOARD_KEY,id); } catch { /* optional */ }
      message = '';
    } catch { /* loadBoard already shows the failure */ }
    finally { switching = false; }
  }
  async function newBoard(): Promise<void> {
    if (switching || busy || historyBusy || loading) return;
    const name = window.prompt('Nome do novo quadro:', 'Novo quadro');
    if (name===null) return;
    try {
      const created = await boardApi.createBoard(name);
      boards = [...boards,created];
      await switchBoard(created.id);
      if (activeBoardId===created.id) message = 'Novo quadro criado.';
    } catch (e) { error = 'Falha ao criar quadro: ' + String(e); }
  }
  async function renameCurrentBoard(): Promise<void> {
    const current=boards.find(board=>board.id===activeBoardId);
    if (!current || switching || historyBusy) return;
    const name=window.prompt('Renomear quadro:',current.name);
    if (name===null || name===current.name) return;
    try {
      const renamed=await boardApi.renameBoard(current.id,name);
      boards = boards.map(board=>board.id===renamed.id?renamed:board);
      message = 'Nome atualizado.';
    } catch(e) { error='Não foi possível renomear: '+String(e); }
  }
  async function chooseConnection(id: string): Promise<void> {
    if (connecting || switching || loading || historyBusy) return;
    if (!connectFrom) {
      connectFrom=id;
      message='Selecione o botão Conectar em outro cartão para traçar uma seta.';
      return;
    }
    if (connectFrom===id) { connectFrom=null; message='Conexão cancelada.'; return; }
    connecting=true;
    const source=connectFrom;
    try {
      const edge=await boardApi.connect(activeBoardId,source,id);
      if (!connections.some(existing=>existing.id===edge.id)) connections=[...connections,edge];
      history={...history,redo:[]};
      connectFrom=null;
      message='Conexão salva.';
    } catch(e) { error='Falha ao conectar os cartões: '+String(e); }
    finally { connecting=false; }
  }
  async function removeConnection(id: string): Promise<void> {
    if (historyBusy || switching || loading) return;
    try {
      await boardApi.disconnect(activeBoardId,id);
      connections=connections.filter(edge=>edge.id!==id);
      history={...history,redo:[]};
      message='Conexão removida.';
    } catch(e) { error='Falha ao remover conexão: '+String(e); }
  }
  async function refreshReferences() {
    const id = activeBoardId;
    try {
      const current = await boardApi.references(id);
      if (activeBoardId === id && !loading) {
        references = Object.fromEntries(current.map(ref => [ref.item_id, ref]));
      }
    } catch { /* Keep the last visible state while a refresh fails. */ }
  }
  async function searchReferences() {
    const request = ++referenceSearchRequest;
    referenceSearching = true;
    try {
      const matches = await boardApi.searchTargets(referenceQuery);
      if (request === referenceSearchRequest) referenceTargets = matches;
    } catch (e) {
      if (request === referenceSearchRequest) error = 'Erro na busca do Arquivo: ' + String(e);
    } finally {
      if (request === referenceSearchRequest) referenceSearching = false;
    }
  }
  function openPicker() {
    if (busy || switching || loading || referenceSaving || historyBusy) return;
    pickerOpen = true;
    referenceQuery = '';
    referenceTargets = [];
    void searchReferences();
  }
  function onReferenceSearch(text: string) {
    referenceQuery = text;
    if (pickerTimer !== null) clearTimeout(pickerTimer);
    pickerTimer = setTimeout(() => {
      pickerTimer = null;
      void searchReferences();
    }, 220);
  }
  async function addReference(target: BoardReferenceTarget) {
    if (busy || switching || loading || referenceSaving || historyBusy) return;
    referenceSaving = true;
    const boardId = activeBoardId;
    try {
      const point = center();
      const card = await boardApi.createReference(boardId,target.target_type,target.target_id,point.x,point.y);
      if (boardId !== activeBoardId) return;
      items = [...items, card];
      confirmed.set(card.id,card);
      history={...history,redo:[]};
      const refs = await boardApi.references(boardId);
      if (boardId === activeBoardId) {
        references = Object.fromEntries(refs.map(ref => [ref.item_id, ref]));
        selected = card.id;
        pickerOpen = false;
        message = 'Referência adicionada. O texto continua salvo apenas no Arquivo.';
      }
    } catch (e) { error = 'Não foi possível vincular o item: ' + String(e); }
    finally { referenceSaving = false; }
  }
  onMount(() => {
    let alive = true;
    const unlisten: Array<() => void> = [];
    viewportWidth = Math.max(1,stage.clientWidth);
    viewportHeight = Math.max(1,stage.clientHeight);
    const observer = new ResizeObserver(() => {
      viewportWidth = Math.max(1,stage.clientWidth);
      viewportHeight = Math.max(1,stage.clientHeight);
    });
    observer.observe(stage);
    for (const eventName of ['notes:changed','tasks:changed']) {
      void listen(eventName, () => { if (alive) void refreshReferences(); })
        .then(dispose => { if (alive) unlisten.push(dispose); else dispose(); });
    }
    void load();
    // A non-passive wheel listener is required to prevent browser-level zoom.
    stage.addEventListener('wheel', wheel, { passive: false });
    // Clipboard paste should work even immediately after using the toolbar.
    window.addEventListener('paste', paste);
    return () => {
      alive = false;
      observer.disconnect();
      if (viewTimer !== null) clearTimeout(viewTimer);
      persistView();
      unlisten.forEach(dispose => dispose());
      if (pickerTimer !== null) clearTimeout(pickerTimer);
      stage.removeEventListener('wheel', wheel);
      window.removeEventListener('paste', paste);
      for (const timeout of timers.values()) clearTimeout(timeout);
      timers.clear();
      for (const id of [...dirty]) void persist(id);
    };
  });

  function center(): { x: number; y: number } {
    const width = stage?.clientWidth ?? 850, height = stage?.clientHeight ?? 520;
    return { x: Math.round((width / 2 - panX) / zoom - 150),
             y: Math.round((height / 2 - panY) / zoom - 100) };
  }
  async function create(kind: BoardKind, content?: string, dataUrl?: string) {
    if (busy || switching || loading || historyBusy) return;
    busy = true;
    error = '';
    try {
      const position = center();
      const initial = content ?? (kind === 'note' ? 'Nova anotação...' :
        kind === 'link' ? 'https://' :
        kind === 'table' ? JSON.stringify([['Título', 'Valor'], ['Item 1', ''], ['Item 2', '']]) : 'Imagem');
      const item = await boardApi.create(activeBoardId, kind, position.x + items.length % 6 * 24,
        position.y + items.length % 6 * 24, initial, dataUrl);
      items = [...items, item];
      confirmed.set(item.id,item);
      history={...history,redo:[]};
      if (dataUrl && Object.keys(assets).length < BOARD_IMAGE_CACHE_LIMIT) {
        assets = { ...assets, [item.id]: dataUrl };
        // The next visibility pass evicts it if the new card is off-screen.
      }
      selected = item.id;
      message = 'Cartão criado e salvo localmente.';
    } catch (e) { error = 'Falha ao criar cartão: ' + String(e); }
    finally { busy = false; }
  }
  async function addImage(file: File) {
    if (!['image/jpeg','image/png','image/webp','image/gif'].includes(file.type)) {
      error = 'Use imagens PNG, JPG, WebP ou GIF.';
      return;
    }
    if (file.size > 4_500_000) {
      error = 'Nesta versão, imagens precisam ter até 4,5 MB.';
      return;
    }
    const boardAtStart = activeBoardId;
    const reader = new FileReader();
    reader.onload = () => {
      if (boardAtStart !== activeBoardId || switching || loading) {
        message = 'Imagem não adicionada porque o quadro foi alterado. Selecione-a novamente.';
        return;
      }
      if (typeof reader.result === 'string') void create('image', file.name.slice(0, 250), reader.result);
    };
    reader.onerror = () => { error = 'Não foi possível ler a imagem.'; };
    reader.readAsDataURL(file);
  }
  function chooseImage() { imageInput?.click(); }
  function inputImages(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (file) void addImage(file);
    input.value = '';
  }
  function paste(event: ClipboardEvent) {
    if (switching || loading || busy || historyBusy) return;
    const target = event.target;
    if (target instanceof HTMLElement && target.closest('input,textarea,[contenteditable="true"]')) return;
    const files = Array.from(event.clipboardData?.files ?? []);
    const image = files.find(file => file.type.startsWith('image/'));
    if (image) { event.preventDefault(); void addImage(image); return; }
    const text = event.clipboardData?.getData('text/plain')?.trim();
    if (!text) return;
    event.preventDefault();
    if (text.includes('\t')) {
      const rows = text.split(/\r?\n/).filter(Boolean).slice(0, 15)
        .map(row => row.split('\t').slice(0, 8));
      void create('table', JSON.stringify(rows));
    } else if (/^https?:\/\//i.test(text)) void create('link', text.slice(0, 100_000));
    else void create('note', text.slice(0, 100_000));
  }
  function onDrop(event: DragEvent) {
    event.preventDefault();
    if (switching || loading || busy || historyBusy) return;
    const file = Array.from(event.dataTransfer?.files ?? []).find(item => item.type.startsWith('image/'));
    if (file) void addImage(file);
  }
  function zoomAt(clientX: number, clientY: number, to: number) {
    const next = clampBoardZoom(to);
    const bounds = stage?.getBoundingClientRect();
    if (!bounds) return;
    const x = clientX - bounds.left, y = clientY - bounds.top;
    panX = x - (x - panX) * next / zoom;
    panY = y - (y - panY) * next / zoom;
    zoom = next;
    scheduleViewSave();
  }
  function wheel(event: WheelEvent) {
    event.preventDefault();
    if (switching || loading) return;
    zoomAt(event.clientX, event.clientY, zoom * (event.deltaY > 0 ? .90 : 1.10));
  }
  function zoomButton(factor: number) {
    const bounds = stage?.getBoundingClientRect();
    if (bounds) zoomAt(bounds.left + bounds.width / 2, bounds.top + bounds.height / 2, zoom * factor);
  }
  function startPan(event: PointerEvent) {
    if (switching || loading || historyBusy || (event.button !== 0 && event.button !== 1)) return;
    const target = event.target;
    if (target instanceof HTMLElement && target.closest('.re-board-card,.re-board-empty,.re-board-zoom,.re-board-minimap,.re-board-search-panel,button,input,textarea,a')) return;
    stage.focus({preventScroll:true});
    gesture = { kind: 'pan', pointer: event.pointerId, startX: event.clientX, startY: event.clientY,
      x: panX, y: panY, width: 0, height: 0 };
    stage.setPointerCapture(event.pointerId);
  }
  function itemPointer(event: PointerEvent, item: BoardItem, mode: 'move' | 'resize') {
    if (event.button !== 0 || switching || loading || historyBusy) return;
    event.preventDefault();
    event.stopPropagation();
    selected = item.id;
    gesture = { kind: mode, pointer: event.pointerId, startX: event.clientX, startY: event.clientY,
      x: item.x, y: item.y, width: item.width, height: item.height, id: item.id };
    stage.setPointerCapture(event.pointerId);
  }
  function pointerMove(event: PointerEvent) {
    if (!gesture || gesture.pointer !== event.pointerId) return;
    const dx = event.clientX - gesture.startX, dy = event.clientY - gesture.startY;
    if (gesture.kind === 'pan') { panX = gesture.x + dx; panY = gesture.y + dy; }
    else if (gesture.id) {
      if (gesture.kind === 'move') updateLocal(gesture.id, { x: Math.round(gesture.x + dx / zoom),
        y: Math.round(gesture.y + dy / zoom) }, false);
      else updateLocal(gesture.id, { width: Math.round(Math.max(160,Math.min(1600,gesture.width+dx/zoom))),
        height: Math.round(Math.max(110,Math.min(1600,gesture.height+dy/zoom))) }, false);
    }
  }
  function pointerEnd(event: PointerEvent) {
    if (!gesture || gesture.pointer !== event.pointerId) return;
    if (gesture.id) {
      const item = findItem(gesture.id);
      if (item) { dirty.add(item.id); keepDraft(item); scheduleSave(item.id, 180); }
    }
    if (gesture.kind === 'pan') scheduleViewSave();
    gesture = null;
    if (stage.hasPointerCapture(event.pointerId)) stage.releasePointerCapture(event.pointerId);
  }
  function rows(content: string): string[][] {
    try {
      const parsed: unknown = JSON.parse(content);
      if (Array.isArray(parsed) && parsed.length > 0 && parsed.every(row => Array.isArray(row))) {
        return parsed.slice(0, 15).map(row => row.slice(0, 8).map(cell => String(cell).slice(0, 250)));
      }
    } catch { /* Older or incomplete table content. */ }
    return [['Título','Valor'],['','']];
  }
  function tableCell(id: string, row: number, column: number, text: string) {
    const item = findItem(id);
    if (!item) return;
    const cells = rows(item.content);
    if (cells[row]) cells[row][column] = text;
    updateLocal(id, { content: JSON.stringify(cells) });
  }
  function addTableRow(id: string) {
    const item = findItem(id); if (!item) return;
    const cells = rows(item.content); if (cells.length >= 15) return;
    cells.push(Array.from({length: cells[0]?.length ?? 2},()=>''));
    updateLocal(id,{content: JSON.stringify(cells)});
  }
  function addTableColumn(id: string) {
    const item = findItem(id); if (!item) return;
    const cells = rows(item.content); if ((cells[0]?.length ?? 0) >= 8) return;
    updateLocal(id,{content:JSON.stringify(cells.map(row=>[...row,'']))});
  }
  function safeLink(value: string): string | null {
    try { const url = new URL(value.trim()); return (url.protocol==='https:'||url.protocol==='http:') ? url.href : null; }
    catch { return null; }
  }
  async function removeCard(item: BoardItem) {
    if (switching || loading || historyBusy) return;
    if (!window.confirm('Mover este cartão para fora do Quadro? Ele continuará preservado no banco de dados.')) return;
    try {
      if (flights.has(item.id)) await flights.get(item.id);
      if (dirty.has(item.id)) await persist(item.id);
      if (dirty.has(item.id)) { error='Não foi possível salvar as alterações. A exclusão foi cancelada.'; return; }
      const current = findItem(item.id);
      if (!current) return;
      await boardApi.remove(item.id,current.revision);
      items = items.filter(card=>card.id!==item.id);
      confirmed.delete(item.id);
      history={...forgetBoardCardHistory(history,item.id),redo:[]};
      references = Object.fromEntries(Object.entries(references).filter(([key])=>key!==item.id));
      connections = connections.filter(edge=>edge.from_id!==item.id && edge.to_id!==item.id);
      if (connectFrom===item.id) connectFrom=null;
      if (selected===item.id) selected=null;
      // Do not remove localStorage drafts here: another WebView may own
      // the latest copy, or an orphan may be the user's only recovery path.
      // The recovery centre can safely display/discard such copies.
      scanBoardRecoveries();
      message = 'Cartão removido do quadro. Possíveis rascunhos de recuperação foram preservados.';
    } catch (e) { error='Não foi possível remover: '+String(e); }
  }
</script>

<section class="re-board-screen" aria-label="Quadro infinito">
  <div class="re-board-toolbar">
    <div class="re-board-intro"><span class="re-board-kicker">SEU ESPAÇO VISUAL</span><strong>Quadros<span class="re-board-accent">.</span></strong><small>Organize suas ideias e conecte cartões.</small>
      <div class="re-board-picker"><label for="recall-board-select">Quadro atual</label>
        <select id="recall-board-select" value={activeBoardId} disabled={switching || loading}
          onchange={(event)=>{const element=event.currentTarget;const value=element.value;void switchBoard(value).finally(()=>{element.value=activeBoardId;});}}>
          {#each boards as board (board.id)}<option value={board.id}>{board.name}</option>{/each}
        </select>
        <button title="Criar novo quadro" aria-label="Criar novo quadro" disabled={switching} onclick={()=>{void newBoard();}}><Plus size={15}/></button>
        <button title="Renomear quadro" aria-label="Renomear quadro" disabled={switching} onclick={()=>{void renameCurrentBoard();}}><Pencil size={14}/></button>
      </div>
    </div>
    <div class="re-board-tools" role="toolbar" aria-label="Adicionar ao quadro">
      <button onclick={() => { void create('note'); }} disabled={busy || switching || loading}><StickyNote size={16}/> Nota</button>
      <button onclick={chooseImage} disabled={busy || switching || loading}><ImagePlus size={16}/> Imagem</button>
      <button onclick={() => { void create('link'); }} disabled={busy || switching || loading}><Link2 size={16}/> Link</button>
      <button onclick={() => { void create('table'); }} disabled={busy || switching || loading}><Table2 size={16}/> Tabela</button>
      <button onclick={openPicker} disabled={busy || switching || loading || referenceSaving} title="Referenciar nota ou tarefa já existente"><Database size={16}/> Do Arquivo</button>
      <button onclick={()=>{searchOpen=!searchOpen;if(!searchOpen)searchQuery='';}} disabled={loading} title="Localizar cartões neste quadro"><Search size={16}/> Encontrar</button>
      <button onclick={fitAll} disabled={loading || items.length===0} title="Enquadrar todos os cartões"><ZoomOut size={16}/> Enquadrar</button>
      <button onclick={()=>{minimapOpen=!minimapOpen;}} disabled={loading} title="Mostrar ou ocultar minimapa"><RotateCcw size={16}/> Minimapa</button>
      <span class="re-board-history-controls" aria-label="Histórico de ações do quadro">
        <button title="Desfazer última alteração salva (Ctrl+Z com o quadro em foco)"
          aria-label="Desfazer última alteração do cartão" disabled={historyBusy || switching || loading || history.undo.length===0}
          onclick={()=>{void performHistory('undo');}}><Undo2 size={16}/> Desfazer</button>
        <button title="Refazer última alteração desfeita (Ctrl+Y ou Ctrl+Shift+Z com o quadro em foco)"
          aria-label="Refazer última alteração do cartão" disabled={historyBusy || switching || loading || history.redo.length===0}
          onclick={()=>{void performHistory('redo');}}><Redo2 size={16}/> Refazer</button>
      </span>
      <input bind:this={imageInput} type="file" accept="image/png,image/jpeg,image/webp,image/gif" class="re-board-hidden" onchange={inputImages} aria-label="Escolher imagem para o quadro"/>
    </div>
  </div>
  {#if error}<div role="alert" class="re-board-alert"><span>{error}</span><button onclick={()=>error=''} aria-label="Fechar erro">×</button></div>{/if}
  {#if message}<div role="status" class="re-board-message"><span>{message}</span><button onclick={()=>message=''} aria-label="Fechar aviso">×</button></div>{/if}
  {#if recoveries.length>0}
    <div class="re-board-recovery-banner" role="status">
      <span><Database size={15}/> {recoveries.length} {recoveries.length===1?'rascunho recuperável':'rascunhos recuperáveis'} neste quadro.</span>
      <button onclick={()=>{scanBoardRecoveries();recoveryOpen=true;}}>Examinar cópias</button>
    </div>
  {/if}
  <div class="re-board-stage" bind:this={stage} tabindex="0" role="application" aria-label="Quadro visual: arraste o fundo para navegar, use a roda do mouse para aproximar"
    onpointerdown={startPan} onpointermove={pointerMove} onpointerup={pointerEnd} onpointercancel={pointerEnd}
    ondragover={event=>event.preventDefault()} ondrop={onDrop} onkeydown={boardShortcut}>
    <div class="re-board-grid" style={'background-size: '+(28*zoom)+'px '+(28*zoom)+'px; background-position: '+panX+'px '+panY+'px'}></div>
    <div class="re-board-world" style={'transform:translate('+panX+'px,'+panY+'px) scale('+zoom+')'}>
      <svg class="re-board-edges" xmlns="http://www.w3.org/2000/svg" width="1" height="1" aria-hidden="true">
        <defs><marker id="re-board-arrow" markerWidth="8" markerHeight="8" refX="7" refY="3" orient="auto">
          <path d="M0,0 L7,3 L0,6 Z" fill="#da5578"/></marker></defs>
        {#each visibleEdges as link (link.edge.from_id+':'+link.edge.to_id)}
          <line x1={link.from.x + link.from.width/2} y1={link.from.y + link.from.height/2}
            x2={link.to.x + link.to.width/2} y2={link.to.y + link.to.height/2}
            stroke="#da5578" stroke-width="2.5" marker-end="url(#re-board-arrow)"/>
        {/each}
      </svg>
      {#each visibleItems as item (item.id)}
        <article class="re-board-card" class:re-board-selected={selected===item.id}
          class:re-board-note={item.kind==='note'} class:re-board-image={item.kind==='image'}
          class:re-board-link={item.kind==='link'} class:re-board-table={item.kind==='table'} class:re-board-connecting={connectFrom===item.id}
          style={'left:'+item.x+'px;top:'+item.y+'px;width:'+item.width+'px;height:'+item.height+'px'}>
          <div class="re-board-card-top">
            <button class="re-board-handle" title="Arrastar cartão" aria-label="Arrastar cartão"
              onpointerdown={event=>itemPointer(event,item,'move')} onclick={()=>selected=item.id}>
              <Move size={14}/> <span>{references[item.id]?(references[item.id].target_type==='note'?'NOTA VINCULADA':'TAREFA VINCULADA'):item.kind==='note'?'NOTA':item.kind==='link'?'LINK':item.kind==='image'?'IMAGEM':'TABELA'}</span>
            </button>
            <button class="re-board-connect" title={connectFrom===item.id?'Cancelar conexão':'Criar conexão'} aria-label={connectFrom===item.id?'Cancelar conexão':'Conectar cartão'} disabled={connecting || switching || loading || historyBusy} onclick={()=>{void chooseConnection(item.id);}}><Link2 size={14}/></button>
            <button class="re-board-remove" title="Remover cartão" aria-label="Remover cartão" disabled={switching || loading || historyBusy} onclick={()=>{void removeCard(item);}}><Trash2 size={14}/></button>
          </div>
          {#if references[item.id]}
            <div class="re-board-reference">
              <div class="re-board-reference-kind">
                {#if references[item.id].target_type==='note'}<FileText size={16}/>{:else}<ListTodo size={16}/>{/if}
                {references[item.id].target_type==='note'?'Nota do Arquivo':'Tarefa do Arquivo'}
                {#if references[item.id].completed}<span class="re-board-reference-done"><CheckCircle2 size={12}/> Concluída</span>{/if}
              </div>
              <strong title={references[item.id].title}>{references[item.id].title}</strong>
              <p>{references[item.id].available ? (references[item.id].preview || 'Sem descrição.') : 'O item original foi excluído. A referência permanece para recuperação.'}</p>
              <button disabled={!references[item.id].available || switching || loading} onclick={()=>{void onOpenReference?.(references[item.id].target_type,references[item.id].target_id);}}>
                Abrir no Arquivo <ExternalLink size={13}/>
              </button>
            </div>
          {:else if item.kind==='note'}
            <textarea aria-label="Conteúdo da nota no quadro" placeholder="Escreva sua ideia..." value={item.content} disabled={switching || loading || historyBusy}
              oninput={event=>updateLocal(item.id,{content:event.currentTarget.value})}></textarea>
          {:else if item.kind==='link'}
            <div class="re-board-link-body"><label for={'board-link-'+item.id}>Endereço da página</label>
              <textarea id={'board-link-'+item.id} spellcheck="false" placeholder="https://..." value={item.content} disabled={switching || loading || historyBusy}
                oninput={event=>updateLocal(item.id,{content:event.currentTarget.value})}></textarea>
              {#if safeLink(item.content)}<a href={safeLink(item.content) ?? undefined} target="_blank" rel="noopener noreferrer">Abrir link <ExternalLink size={13}/></a>
              {:else}<small>Digite um link http:// ou https://</small>{/if}
            </div>
          {:else if item.kind==='image'}
            {#if assets[item.id]}<img src={assets[item.id]} alt={item.content || 'Imagem adicionada ao quadro'} draggable="false"/>
            {:else}<span class="re-board-image-loading">Imagem sob demanda · aproxime para visualizar</span>{/if}
            <span class="re-board-image-name" title={item.content}>{item.content}</span>
          {:else}
            <div class="re-board-table-scroller">
              <table aria-label="Tabela editável do quadro"><tbody>
                {#each rows(item.content) as row, ri}
                  <tr>{#each row as cell, ci}
                    <td><input aria-label={'Linha '+(ri+1)+', coluna '+(ci+1)} value={cell} disabled={switching || loading || historyBusy}
                      oninput={event=>tableCell(item.id,ri,ci,event.currentTarget.value)}/></td>
                  {/each}</tr>
                {/each}
              </tbody></table>
            </div>
            <div class="re-board-table-actions">
              <button onclick={()=>addTableRow(item.id)}><Plus size={12}/> Linha</button>
              <button onclick={()=>addTableColumn(item.id)}><Plus size={12}/> Coluna</button>
            </div>
          {/if}
          <button class="re-board-resizer" title="Redimensionar cartão" aria-label="Redimensionar cartão"
            disabled={switching || loading || historyBusy} onpointerdown={event=>itemPointer(event,item,'resize')}></button>
        </article>
      {/each}
    </div>
    {#if loading}
      <div class="re-board-empty" role="status" aria-live="polite">
        <StickyNote size={25}/>
        {#if boardLoadFailed}
          <strong>Não foi possível carregar o Quadro</strong>
          <span>Nenhuma lista incompleta foi aplicada. Os cartões salvos continuam no SQLite.</span>
          <button onclick={()=>{void load();}}><RotateCcw size={15}/> Tentar novamente</button>
        {:else}
          <strong>Carregando quadro...</strong>
          <span>{loadingCount > 0 ? loadingCount + ' cartões recebidos em lotes seguros' : 'Preparando dados locais...'}</span>
          <span>As edições serão liberadas quando o carregamento estiver completo.</span>
        {/if}
      </div>
    {:else if items.length===0}
      <div class="re-board-empty"><StickyNote size={28}/><strong>Um espaço aberto para suas ideias.</strong>
        <span>Crie uma nota, cole uma imagem ou arraste um arquivo para começar.</span>
        <button onclick={()=>{void create('note');}}><Plus size={15}/> Criar primeira nota</button>
      </div>
    {/if}
    {#if searchOpen}
      <div class="re-board-search-panel" aria-label="Busca de cartões neste quadro">
        <div class="re-board-search-head"><strong><Search size={15}/> Encontrar no quadro</strong>
          <button title="Fechar busca" aria-label="Fechar busca" onclick={closeBoardSearch}><X size={16}/></button>
        </div>
        <input type="search" aria-label="Buscar cartões" placeholder="Pesquise notas, tabelas ou vínculos..."
          maxlength="140" value={searchQuery} oninput={event=>searchQuery=event.currentTarget.value}
          onkeydown={event=>{if(event.key==='Escape')closeBoardSearch();}}/>
        {#if searchQuery.trim()}
          <div class="re-board-search-results">
            {#if foundCards.length}
              <small>{foundCards.length}{foundCards.length===40?' ou mais':''} cartões encontrados</small>
              {#each foundCards as found (found.id)}
                <button onclick={()=>{focusCard(found.id);closeBoardSearch();}} title={labelForCard(found)}>
                  <span class="re-board-search-kind">{references[found.id]?'VÍNCULO':found.kind.toUpperCase()}</span>
                  <span>{labelForCard(found)}</span>
                </button>
              {/each}
            {:else}<p>Nenhum cartão corresponde à busca.</p>{/if}
          </div>
        {:else}<p class="re-board-search-tip">Busque dentro do quadro atual. Selecione um resultado para ir diretamente ao cartão.</p>{/if}
      </div>
    {/if}
    {#if minimapOpen && items.length>0}
      <div class="re-board-minimap" aria-label="Minimapa do quadro">
        <div class="re-board-minimap-header"><strong>MINIMAPA</strong><span>{items.length} cartões</span></div>
        <button class="re-board-mini-content" aria-label="Navegar pelo minimapa; Enter enquadra o quadro"
          title="Clique para centralizar a área; Enter para enquadrar"
          onclick={navigateMinimap}>
          <canvas bind:this={minimapCanvas} width={MINIMAP_WIDTH*2} height={MINIMAP_HEIGHT*2}
            aria-hidden="true"></canvas>
        </button>
      </div>
    {/if}
    <div class="re-board-zoom" aria-label="Controles de zoom">
      <button title="Diminuir zoom" aria-label="Diminuir zoom" onclick={()=>zoomButton(1/1.15)}><ZoomOut size={17}/></button>
      <span>{Math.round(zoom*100)}%</span>
      <button title="Aumentar zoom" aria-label="Aumentar zoom" onclick={()=>zoomButton(1.15)}><ZoomIn size={17}/></button>
      <button title="Recentralizar quadro" aria-label="Recentralizar quadro" onclick={()=>{panX=0;panY=0;zoom=1;scheduleViewSave();}}><RotateCcw size={16}/></button>
    </div>
    {#if selected && connections.some(edge=>edge.from_id===selected || edge.to_id===selected)}
      <div class="re-board-connections" aria-label="Conexões do cartão selecionado">
        <strong>Conexões</strong>
        {#each connections.filter(edge=>edge.from_id===selected || edge.to_id===selected) as edge (edge.id)}
          <button title="Remover esta conexão" onclick={()=>{void removeConnection(edge.id);}}>
            <Unlink2 size={13}/> {edge.from_id===selected?'Para':'De'} {references[edge.from_id===selected?edge.to_id:edge.from_id]?.title.slice(0,22) || items.find(item=>item.id===(edge.from_id===selected?edge.to_id:edge.from_id))?.content.slice(0,22) || 'cartão'}
          </button>
        {/each}
      </div>
    {/if}
    <div class="re-board-help">Arraste para navegar · F: enquadrar · Ctrl+F: busca · Ctrl+Z/Y: histórico (quadro em foco)</div>
  </div>
  {#if recoveryOpen}
    <div class="re-board-modal-backdrop">
      <div class="re-board-modal re-board-recovery-modal" role="dialog" aria-modal="true" aria-labelledby="re-board-recovery-title">
        <div class="re-board-modal-head">
          <div><span class="re-board-kicker">PROTEÇÃO DO SEU TRABALHO</span>
            <h3 id="re-board-recovery-title">Recuperar rascunhos</h3>
            <p>Examine cópias locais antes de descartá-las. O SQLite continua sendo a versão oficial.</p>
          </div>
          <button aria-label="Fechar recuperação" disabled={recoveryBusy} onclick={()=>recoveryOpen=false}><X size={19}/></button>
        </div>
        <div class="re-board-recovery-list">
          {#if recoveries.length===0}
            <p>Nenhuma cópia pendente neste quadro.</p>
          {:else}
            {#each recoveries as entry (entry.key)}
              <div class="re-board-recovery-item">
                <strong>{entry.status==='conflict'?'Revisão conflitante':entry.status==='orphan'?'Cartão removido':entry.status==='competing'?'Outra janela ou versão':'Rascunho não salvo'}</strong>
                <small>{recoveryDescription(entry)}</small>
                <textarea readonly aria-label="Conteúdo original do rascunho de recuperação"
                  value={entry.snapshot?.content ?? entry.raw} rows="4"></textarea>
                <div class="re-board-recovery-actions">
                  {#if entry.snapshot && entry.snapshot.kind!=='image' && !references[entry.itemId]}
                    <button disabled={recoveryBusy || switching || loading} onclick={()=>{void copyRecovery(entry);}}>
                      <Plus size={14}/> Criar cartão com esta cópia
                    </button>
                  {:else}
                    <span>Esta cópia só pode ser examinada; imagens e vínculos não são reconstruídos automaticamente.</span>
                  {/if}
                  <button class="re-board-recovery-discard" disabled={recoveryBusy || dirty.has(entry.itemId) || flights.has(entry.itemId)}
                    onclick={()=>discardRecovery(entry)}><Trash2 size={13}/> Descartar cópia</button>
                </div>
              </div>
            {/each}
          {/if}
        </div>
      </div>
    </div>
  {/if}
  {#if pickerOpen}
    <div class="re-board-modal-backdrop">
      <div class="re-board-modal" role="dialog" aria-modal="true" aria-labelledby="re-board-picker-title">
        <div class="re-board-modal-head">
          <div><span class="re-board-kicker">CONEXÃO COM SUA MEMÓRIA</span><h3 id="re-board-picker-title">Adicionar do Arquivo</h3><p>O cartão mostra o conteúdo original sem duplicá-lo.</p></div>
          <button aria-label="Fechar seleção" disabled={referenceSaving} onclick={()=>{pickerOpen=false;}}><X size={19}/></button>
        </div>
        <label class="re-board-modal-search" for="re-board-reference-query"><Search size={17}/>
          <input id="re-board-reference-query" aria-label="Buscar notas ou tarefas" placeholder="Pesquise pelo nome ou conteúdo..."
            maxlength="100" value={referenceQuery} oninput={event=>onReferenceSearch(event.currentTarget.value)}
            onkeydown={event=>{if(event.key==='Escape'&&!referenceSaving)pickerOpen=false;}}/>
        </label>
        <div class="re-board-target-list">
          {#if referenceSearching}<p class="re-board-target-message">Buscando no Arquivo...</p>
          {:else if referenceTargets.length===0}<p class="re-board-target-message">Nenhuma nota ou tarefa encontrada.</p>
          {:else}
            {#each referenceTargets as target (target.target_type+':'+target.target_id)}
              <button class="re-board-target" disabled={referenceSaving} onclick={()=>{void addReference(target);}}>
                <span class="re-board-target-icon">{#if target.target_type==='note'}<FileText size={17}/>{:else}<ListTodo size={17}/>{/if}</span>
                <span class="re-board-target-copy"><strong>{target.title}</strong><small>{target.preview || (target.target_type==='note'?'Nota sem descrição':'Tarefa sem descrição')}</small></span>
                <Plus size={16}/>
              </button>
            {/each}
          {/if}
        </div>
        <div class="re-board-modal-bottom">Até 40 resultados mais recentes. As edições continuam sendo feitas no Arquivo.</div>
      </div>
    </div>
  {/if}
</section>

<style>
  .re-board-screen{display:flex;flex-direction:column;flex:1;min-height:0;background:#0c0c0f;overflow:hidden;position:relative}
  .re-board-toolbar{display:flex;justify-content:space-between;align-items:center;gap:18px;padding:17px 28px;border-bottom:1px solid #2b2930;background:#111114;flex-wrap:wrap}
  .re-board-intro{display:flex;flex-direction:column;gap:3px}
  .re-board-kicker{font-size:10px;letter-spacing:1.6px;color:#bc8591;font-weight:750}
  .re-board-intro strong{font-size:22px;line-height:1.1;letter-spacing:-.4px}
  .re-board-intro small{font-size:11px;color:#a9a3ae}
  .re-board-accent{color:#e90530}
  .re-board-picker{display:flex;align-items:center;gap:5px;margin-top:8px}
  .re-board-picker label{font-size:10px;color:#bbaeb7;white-space:nowrap;margin-right:3px}
  .re-board-picker select{background:#201a20;color:#f4edf2;border:1px solid #58414d;border-radius:7px;max-width:185px;padding:5px 8px;font-size:12px}
  .re-board-picker button{display:grid;place-items:center;height:28px;width:28px;background:#271d24;border:1px solid #49323e;border-radius:6px;color:#f1d9e1}
  .re-board-edges{position:absolute;top:0;left:0;overflow:visible;pointer-events:none}
  .re-board-connect{display:grid;place-items:center;width:28px;height:27px;border:0;background:transparent;color:#e4a6ba}
  .re-board-connect:hover{color:#ff5c89}
  .re-board-connecting{border-color:#fc5e8a!important;box-shadow:0 0 0 2px #d73d6a,0 0 25px #d73d6a40}
  .re-board-connections{position:absolute;top:15px;right:15px;display:flex;flex-direction:column;gap:5px;max-height:170px;overflow-y:auto;min-width:155px;max-width:250px;background:#1a181ded;border:1px solid #52414d;border-radius:9px;padding:9px;z-index:3}
  .re-board-connections strong{font-size:11px}
  .re-board-connections button{display:flex;align-items:center;gap:6px;background:#2b222b;border:0;border-radius:5px;padding:7px;color:#f8d0df;font-size:11px;text-align:left;max-width:230px;word-break:break-word}
  .re-board-tools{display:flex;gap:7px;flex-wrap:wrap}
  .re-board-history-controls{display:flex;align-items:center;gap:6px;padding-left:8px;border-left:1px solid #4c3a45}
  .re-board-tools button:disabled,.re-board-history-controls button:disabled{opacity:.42;cursor:not-allowed}
  .re-board-tools button,.re-board-empty button{display:inline-flex;gap:7px;align-items:center;justify-content:center;background:#221d23;border:1px solid #4b353d;border-radius:9px;padding:9px 12px;color:#f4ecf0;font-size:12px;font-weight:650}
  .re-board-tools button:hover,.re-board-empty button:hover{background:#432330;border-color:#ae3854}
  .re-board-hidden{display:none}
  .re-board-recovery-banner{display:flex;gap:12px;align-items:center;justify-content:space-between;padding:8px 22px;background:#2e2224;color:#f8d5df;border-bottom:1px solid #7b4654;font-size:12px}
  .re-board-recovery-banner span{display:flex;align-items:center;gap:8px}
  .re-board-recovery-banner button{border:1px solid #ad6077;color:#ffe9f0;background:#442631;border-radius:7px;padding:7px 10px;font-size:11px;font-weight:700}
  .re-board-recovery-modal{max-height:min(760px,90%)}
  .re-board-recovery-list{display:flex;flex-direction:column;gap:10px;overflow-y:auto;padding:4px 18px 19px;min-height:80px}
  .re-board-recovery-list>p{font-size:13px;color:#c7b8c4}
  .re-board-recovery-item{display:flex;flex-direction:column;gap:7px;background:#211b23;border:1px solid #5b4654;border-radius:9px;padding:13px}
  .re-board-recovery-item strong{font-size:13px}
  .re-board-recovery-item small{font-size:11px;color:#b8a7b4}
  .re-board-recovery-item textarea{resize:vertical;min-height:70px;width:100%;background:#111014;color:#f1e3ec;border:1px solid #50404d;border-radius:6px;padding:10px;line-height:1.4;font-size:12px}
  .re-board-recovery-actions{display:flex;align-items:center;gap:8px;flex-wrap:wrap}
  .re-board-recovery-actions button{display:flex;align-items:center;gap:6px;background:#432532;border:1px solid #86536a;border-radius:6px;color:#ffdfeb;padding:8px;font-size:11px}
  .re-board-recovery-actions .re-board-recovery-discard{margin-left:auto;background:#231c23;border-color:#534450;color:#cbbbc7}
  .re-board-recovery-actions button:disabled{opacity:.45}
  .re-board-recovery-actions span{font-size:11px;color:#b6a6b5}
  .re-board-search-panel{position:absolute;top:17px;left:17px;width:min(355px,calc(100% - 34px));max-height:min(430px,70%);display:flex;flex-direction:column;gap:9px;padding:13px;background:#18151dea;border:1px solid #5a4555;border-radius:12px;box-shadow:0 14px 35px #0009;z-index:4}
  .re-board-search-head{display:flex;align-items:center;justify-content:space-between;color:#f8e9f2}
  .re-board-search-head strong{display:flex;gap:7px;align-items:center;font-size:13px}
  .re-board-search-head button{background:transparent;border:0;color:#c5b4be;padding:3px;display:grid;place-items:center}
  .re-board-search-panel input{width:100%;background:#100e14;border:1px solid #554451;border-radius:7px;padding:10px;color:#f7eef4;outline:none;font-size:12px}
  .re-board-search-panel input:focus{border-color:#b95879}
  .re-board-search-results{overflow-y:auto;display:flex;flex-direction:column;gap:4px;min-height:0}
  .re-board-search-results small{color:#b6acb9;font-size:10px;padding:3px}
  .re-board-search-results button{display:flex;align-items:center;gap:9px;border:1px solid transparent;border-radius:7px;padding:9px;background:#292029;color:#e9dce5;text-align:left;font-size:12px}
  .re-board-search-results button:hover{border-color:#ad637e;background:#39232d}
  .re-board-search-results button span:last-child{overflow:hidden;text-overflow:ellipsis;white-space:nowrap;flex:1}
  .re-board-search-kind{font-size:9px;font-weight:750;color:#e99bb6;width:58px;flex:0 0 58px}
  .re-board-search-tip,.re-board-search-results p{font-size:11px;line-height:1.4;color:#bcb0bf;margin:3px 0}
  .re-board-minimap{position:absolute;left:17px;bottom:55px;background:#1c1820e8;border:1px solid #4b3e49;border-radius:9px;padding:8px;z-index:3}
  .re-board-minimap-header{display:flex;align-items:center;justify-content:space-between;gap:8px;color:#b6a4b0;margin-bottom:7px}
  .re-board-minimap-header strong{font-size:9px;letter-spacing:1px}
  .re-board-minimap-header span{font-size:9px}
  .re-board-mini-content{display:block;position:relative;width:180px;height:112px;overflow:hidden;background-color:#100f13;background-image:radial-gradient(#4b3e4c .8px,transparent 1px);background-size:15px 15px;border:1px solid #3d323c;border-radius:5px;padding:0;cursor:crosshair}
  .re-board-mini-content canvas{display:block;width:180px;height:112px;pointer-events:none}
  .re-board-mini-content:focus-visible{outline:2px solid #ff799f;outline-offset:3px}
  .re-board-modal-backdrop{position:absolute;inset:0;display:flex;align-items:center;justify-content:center;background:#070608c9;z-index:20;padding:20px}
  .re-board-modal{width:min(600px,100%);max-height:min(650px,100%);display:flex;flex-direction:column;background:#18151c;border:1px solid #50404e;border-radius:14px;box-shadow:0 35px 80px #000a;overflow:hidden}
  .re-board-modal-head{display:flex;justify-content:space-between;align-items:flex-start;gap:15px;padding:21px 22px 14px}
  .re-board-modal-head h3{margin:5px 0;font-size:22px;color:#f7eff3}
  .re-board-modal-head p{margin:0;font-size:12px;color:#aaa3ad}
  .re-board-modal-head button{display:grid;place-items:center;border:0;background:transparent;color:#d6c6d1;padding:5px}
  .re-board-modal-search{display:flex;align-items:center;gap:8px;border:1px solid #554550;border-radius:9px;background:#0f0e13;margin:0 22px 14px;padding:0 10px;color:#ba9caa}
  .re-board-modal-search input{border:0;outline:0;background:transparent;color:#f5eaf2;padding:12px 0;font-size:13px;flex:1;min-width:0}
  .re-board-target-list{display:flex;flex-direction:column;gap:4px;overflow-y:auto;padding:0 13px 14px;min-height:120px}
  .re-board-target-message{padding:22px;color:#b9acb6;text-align:center;font-size:13px}
  .re-board-target{display:flex;align-items:center;gap:11px;border:1px solid transparent;border-radius:9px;padding:12px;background:#221d24;color:#f1e8ef;text-align:left}
  .re-board-target:hover:not(:disabled){border-color:#96516b;background:#31222e}
  .re-board-target-icon{display:grid;place-items:center;color:#fa8ead;width:30px;height:30px;flex-shrink:0;background:#432732;border-radius:7px}
  .re-board-target-copy{flex:1;min-width:0;display:flex;flex-direction:column;gap:3px}
  .re-board-target-copy strong{font-size:13px;text-overflow:ellipsis;white-space:nowrap;overflow:hidden}
  .re-board-target-copy small{font-size:11px;color:#b8abb6;text-overflow:ellipsis;white-space:nowrap;overflow:hidden}
  .re-board-modal-bottom{padding:12px 22px;background:#151318;border-top:1px solid #332b33;color:#a99aa7;font-size:11px}
  .re-board-alert,.re-board-message{display:flex;justify-content:space-between;gap:12px;padding:9px 24px;background:#3d1b24;color:#ffe6eb;font-size:12px;z-index:5}
  .re-board-message{background:#24212b;color:#d7d1dc}
  .re-board-alert button,.re-board-message button{border:0;background:none;color:inherit;font-size:20px}
  .re-board-stage{flex:1;position:relative;min-height:0;overflow:hidden;cursor:grab;touch-action:none;outline:0}
  .re-board-stage:active{cursor:grabbing}
  .re-board-grid{position:absolute;inset:0;background-image:radial-gradient(circle,#3b353c 1px,transparent 1.1px);pointer-events:none}
  .re-board-world{position:absolute;inset:0;transform-origin:0 0;pointer-events:none}
  .re-board-card{position:absolute;display:flex;flex-direction:column;overflow:hidden;background:#19191e;border:1px solid #50424a;border-radius:12px;box-shadow:0 14px 28px #0008;pointer-events:auto;color:#f2edf0;min-width:160px;min-height:110px}
  .re-board-selected{border-color:#e25373;box-shadow:0 0 0 1px #e25373,0 14px 28px #000a}
  .re-board-note{background:#241d21;border-color:#665058}
  .re-board-reference{display:flex;flex-direction:column;gap:8px;flex:1;min-height:0;padding:13px 15px;background:#191b20}
  .re-board-reference-kind{display:flex;gap:6px;align-items:center;color:#f17d9b;font-size:10px;font-weight:700;letter-spacing:.3px}
  .re-board-reference-done{display:flex;align-items:center;gap:3px;margin-left:auto;color:#88d9a4;font-size:9px;letter-spacing:0}
  .re-board-reference strong{font-size:16px;color:#f6eef2;white-space:normal;overflow:hidden;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical}
  .re-board-reference p{font-size:12px;color:#aea9b5;line-height:1.4;overflow:hidden;display:-webkit-box;-webkit-line-clamp:4;-webkit-box-orient:vertical;white-space:pre-wrap;overflow-wrap:anywhere;margin:0}
  .re-board-reference button{margin-top:auto;background:#30202c;border:1px solid #613b51;border-radius:7px;color:#f9d5e0;display:flex;align-items:center;justify-content:center;gap:5px;padding:7px;font-size:11px;font-weight:650}
  .re-board-reference button:disabled{opacity:.45}
  .re-board-card-top{height:34px;flex:0 0 34px;display:flex;align-items:center;justify-content:space-between;padding:0 6px;background:#13131870;border-bottom:1px solid #ffffff1c}
  .re-board-handle{display:flex;gap:7px;align-items:center;flex:1;background:none;border:0;text-align:left;color:#d4bbc5;font-size:10px;font-weight:750;letter-spacing:1px;cursor:grab;padding:7px}
  .re-board-handle:active{cursor:grabbing}
  .re-board-remove{display:grid;place-items:center;width:28px;height:27px;border:0;background:none;color:#9e8792}
  .re-board-remove:hover{color:#ff7898}
  .re-board-card textarea{flex:1;min-height:0;resize:none;width:100%;padding:14px;background:transparent;border:0;outline:none;color:#f5f0f2;font-size:14px;line-height:1.5}
  .re-board-note textarea{font-size:15px}
  .re-board-link-body{display:flex;flex-direction:column;gap:6px;padding:11px;flex:1;min-height:0}
  .re-board-link-body label{color:#afa4ad;font-size:10px;font-weight:700}
  .re-board-link-body textarea{font-size:12px;padding:7px;background:#101014;border:1px solid #36313a;border-radius:5px;max-height:62px;min-height:37px;word-break:break-all}
  .re-board-link-body a{display:flex;gap:7px;align-items:center;color:#f57a96;font-weight:650;font-size:12px;text-decoration:none}
  .re-board-link-body small{color:#a9a3ae;font-size:10px}
  .re-board-image img{width:100%;height:calc(100% - 55px);object-fit:contain;background:#0d0d12}
  .re-board-image-name{font-size:10px;padding:5px 10px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:#bcb4be}
  .re-board-image-loading{margin:auto;color:#afa5ae}
  .re-board-table-scroller{overflow:auto;flex:1;min-height:0;padding:8px}
  .re-board-table-scroller table{border-collapse:collapse;min-width:100%;table-layout:fixed}
  .re-board-table-scroller td{border:1px solid #504650;min-width:110px}
  .re-board-table-scroller tr:first-child{background:#38232c}
  .re-board-table-scroller input{width:100%;min-width:100px;border:0;background:transparent;color:#f3eff2;padding:7px;font-size:12px;outline:none}
  .re-board-table-actions{display:flex;gap:8px;padding:4px 10px 10px}
  .re-board-table-actions button{display:flex;align-items:center;gap:3px;background:none;color:#bbb0bb;border:0;font-size:11px}
  .re-board-resizer{position:absolute;right:0;bottom:0;width:19px;height:19px;background:linear-gradient(135deg,transparent 52%,#c5516c 53%,#c5516c 59%,transparent 60%,transparent 70%,#c5516c 71%);border:0;cursor:nwse-resize}
  .re-board-empty{position:absolute;top:50%;left:50%;transform:translate(-50%,-50%);display:flex;align-items:center;flex-direction:column;gap:12px;min-width:min(440px,90%);padding:32px;background:#18171bc9;border:1px solid #373138;border-radius:16px;text-align:center;color:#bcb5be;pointer-events:auto}
  .re-board-empty strong{font-size:19px;color:#faf4f7}
  .re-board-empty span{font-size:12px}
  .re-board-zoom{position:absolute;bottom:22px;right:22px;display:flex;gap:4px;align-items:center;background:#1d1a20e8;border:1px solid #49404b;padding:5px;border-radius:10px;z-index:2}
  .re-board-zoom button{width:32px;height:32px;display:grid;place-items:center;background:none;border:0;color:#eee}
  .re-board-zoom span{font-size:12px;min-width:47px;text-align:center;color:#cac0c9}
  .re-board-help{position:absolute;bottom:24px;left:22px;font-size:11px;color:#a9a0ae;background:#17151abc;padding:9px 11px;border-radius:7px;pointer-events:none}
  @media(max-width:800px){.re-board-toolbar{padding:12px;gap:10px}.re-board-intro small{display:none}.re-board-tools button{padding:8px;font-size:11px}.re-board-help{display:none}.re-board-minimap{bottom:16px;left:10px}}
</style>
