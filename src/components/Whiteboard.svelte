<script lang="ts">
  import { onMount } from 'svelte';
  import { StickyNote, ImagePlus, Link2, Table2, ZoomIn, ZoomOut, RotateCcw, Trash2, Move, Plus, ExternalLink, Pencil, Unlink2 } from 'lucide-svelte';
  import { boardApi } from '../lib/api';
  import type { BoardItem, BoardKind, Whiteboard, BoardConnection } from '../lib/types';

  let items = $state<BoardItem[]>([]);
  let boards = $state<Whiteboard[]>([]);
  let activeBoardId = $state('default');
  let connections = $state<BoardConnection[]>([]);
  let connectFrom = $state<string | null>(null);
  let switching = $state(false);
  let connecting = $state(false);
  let boardRequest = 0;
  let assets = $state<Record<string, string>>({});
  let loading = $state(true);
  let busy = $state(false);
  let error = $state('');
  let message = $state('');
  let selected = $state<string | null>(null);
  let zoom = $state(1);
  let panX = $state(0);
  let panY = $state(0);
  let stage: HTMLDivElement;
  let imageInput: HTMLInputElement;

  const dirty = new Set<string>();
  const timers = new Map<string, ReturnType<typeof setTimeout>>();
  const flights = new Map<string, Promise<void>>();
  type Gesture = { kind: 'pan' | 'move' | 'resize'; pointer: number; startX: number; startY: number; x: number; y: number; width: number; height: number; id?: string };
  let gesture: Gesture | null = null;
  const draftKey = (id: string) => 'recall-board-draft:' + id;

  const findItem = (id: string) => items.find(item => item.id === id);
  function sameFields(a: BoardItem, b: BoardItem): boolean {
    return a.x === b.x && a.y === b.y && a.width === b.width
      && a.height === b.height && a.content === b.content;
  }
  function keepDraft(item: BoardItem) {
    try {
      localStorage.setItem(draftKey(item.id), JSON.stringify({
        content: item.content, x: item.x, y: item.y,
        width: item.width, height: item.height, revision: item.revision
      }));
    } catch { message = 'O rascunho local não pôde ser gravado. Evite fechar antes de salvar.'; }
  }
  function clearDraftIfSame(item: BoardItem) {
    try {
      const raw = localStorage.getItem(draftKey(item.id));
      if (!raw) return;
      const draft = JSON.parse(raw) as Partial<BoardItem>;
      if (draft.content === item.content && draft.x === item.x && draft.y === item.y
          && draft.width === item.width && draft.height === item.height) {
        localStorage.removeItem(draftKey(item.id));
      }
    } catch { /* Do not destroy a damaged recovery snapshot. */ }
  }
  function updateLocal(id: string, changes: Partial<BoardItem>, commit = true) {
    if (switching || loading) return;
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
    dirty.delete(id);
    const flight = (async () => {
      try {
        const saved = await boardApi.update(snapshot);
        const latest = findItem(id);
        if (!latest) return;
        const unchanged = sameFields(latest, snapshot);
        items = items.map(item => item.id === id ? { ...item, revision: saved.revision, updated_at: saved.updated_at } : item);
        if (unchanged && !dirty.has(id)) clearDraftIfSame(snapshot);
        else {
          dirty.add(id);
          const current = findItem(id);
          if (current) keepDraft(current);
          scheduleSave(id, 250);
        }
        error = '';
      } catch (e) {
        dirty.add(id);
        error = String(e).includes('CONFLITO_QUADRO')
          ? 'Esse cartão foi alterado em outra janela. Seu rascunho local foi preservado; não o apague.'
          : 'Falha ao salvar cartão. Rascunho local preservado: ' + String(e);
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
    try {
      const [loaded, edges] = await Promise.all([boardApi.list(id),boardApi.connections(id)]);
      if (request !== boardRequest) return;
      for (const handle of timers.values()) clearTimeout(handle);
      timers.clear();
      dirty.clear();
      items = loaded.map(item => {
        try {
          const raw = localStorage.getItem(draftKey(item.id));
          if (!raw) return item;
          const draft = JSON.parse(raw) as Partial<BoardItem>;
          if (typeof draft.content !== 'string' || typeof draft.revision !== 'number') return item;
          if (draft.revision !== item.revision) {
            message = 'Rascunho conflitante preservado: os dados originais permanecem no SQLite.';
            return item;
          }
          const restored = { ...item, content: draft.content, x: Number(draft.x), y: Number(draft.y),
            width: Number(draft.width), height: Number(draft.height) };
          if (![restored.x,restored.y,restored.width,restored.height].every(Number.isFinite)) return item;
          if (!sameFields(item, restored)) { dirty.add(item.id); scheduleSave(item.id); }
          return restored;
        } catch { return item; }
      });
      connections = edges;
      selected = null;
      connectFrom = null;
      assets = {};
      panX = 0; panY = 0; zoom = 1;
      for (const item of loaded) {
        if (item.kind !== 'image') continue;
        void boardApi.asset(item.id).then(dataUrl => {
          if (request === boardRequest && dataUrl) assets = { ...assets, [item.id]: dataUrl };
        }).catch(() => {
          if (request === boardRequest) message = 'Uma imagem não pôde ser carregada.';
        });
      }
    } catch (e) {
      if (request === boardRequest) error = 'Não foi possível abrir o Quadro: ' + String(e);
      throw e;
    } finally {
      if (request === boardRequest) loading = false;
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
      error = 'Não foi possível carregar os quadros: ' + String(e);
      loading = false;
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
    if (id===activeBoardId || switching || busy || loading) return;
    switching = true;
    try {
      if (!(await finishPending())) {
        error = 'Existem cartões não salvos. Resolva o erro antes de trocar de quadro.';
        return;
      }
      await loadBoard(id);
      activeBoardId = id;
      try { localStorage.setItem(BOARD_KEY,id); } catch { /* optional */ }
      message = '';
    } catch { /* loadBoard already shows the failure */ }
    finally { switching = false; }
  }
  async function newBoard(): Promise<void> {
    if (switching || busy) return;
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
    if (!current || switching) return;
    const name=window.prompt('Renomear quadro:',current.name);
    if (name===null || name===current.name) return;
    try {
      const renamed=await boardApi.renameBoard(current.id,name);
      boards = boards.map(board=>board.id===renamed.id?renamed:board);
      message = 'Nome atualizado.';
    } catch(e) { error='Não foi possível renomear: '+String(e); }
  }
  async function chooseConnection(id: string): Promise<void> {
    if (connecting || switching || loading) return;
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
      connectFrom=null;
      message='Conexão salva.';
    } catch(e) { error='Falha ao conectar os cartões: '+String(e); }
    finally { connecting=false; }
  }
  async function removeConnection(id: string): Promise<void> {
    try {
      await boardApi.disconnect(activeBoardId,id);
      connections=connections.filter(edge=>edge.id!==id);
      message='Conexão removida.';
    } catch(e) { error='Falha ao remover conexão: '+String(e); }
  }
  onMount(() => {
    void load();
    // A non-passive wheel listener is required to prevent browser-level zoom.
    stage.addEventListener('wheel', wheel, { passive: false });
    // Clipboard paste should work even immediately after using the toolbar.
    window.addEventListener('paste', paste);
    return () => {
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
    if (busy || switching || loading) return;
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
      if (dataUrl) assets = { ...assets, [item.id]: dataUrl };
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
    if (switching || loading || busy) return;
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
    if (switching || loading || busy) return;
    const file = Array.from(event.dataTransfer?.files ?? []).find(item => item.type.startsWith('image/'));
    if (file) void addImage(file);
  }
  function zoomAt(clientX: number, clientY: number, to: number) {
    const next = Math.max(.2, Math.min(2.2, to));
    const bounds = stage?.getBoundingClientRect();
    if (!bounds) return;
    const x = clientX - bounds.left, y = clientY - bounds.top;
    panX = x - (x - panX) * next / zoom;
    panY = y - (y - panY) * next / zoom;
    zoom = next;
  }
  function wheel(event: WheelEvent) {
    event.preventDefault();
    zoomAt(event.clientX, event.clientY, zoom * (event.deltaY > 0 ? .90 : 1.10));
  }
  function zoomButton(factor: number) {
    const bounds = stage?.getBoundingClientRect();
    if (bounds) zoomAt(bounds.left + bounds.width / 2, bounds.top + bounds.height / 2, zoom * factor);
  }
  function startPan(event: PointerEvent) {
    if (event.button !== 0 && event.button !== 1) return;
    const target = event.target;
    if (target instanceof HTMLElement && target.closest('.re-board-card,.re-board-empty,.re-board-zoom,button,input,textarea,a')) return;
    gesture = { kind: 'pan', pointer: event.pointerId, startX: event.clientX, startY: event.clientY,
      x: panX, y: panY, width: 0, height: 0 };
    stage.setPointerCapture(event.pointerId);
  }
  function itemPointer(event: PointerEvent, item: BoardItem, mode: 'move' | 'resize') {
    if (event.button !== 0 || switching || loading) return;
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
    if (switching || loading) return;
    if (!window.confirm('Mover este cartão para fora do Quadro? Ele continuará preservado no banco de dados.')) return;
    try {
      if (flights.has(item.id)) await flights.get(item.id);
      if (dirty.has(item.id)) await persist(item.id);
      if (dirty.has(item.id)) { error='Não foi possível salvar as alterações. A exclusão foi cancelada.'; return; }
      const current = findItem(item.id);
      if (!current) return;
      await boardApi.remove(item.id,current.revision);
      items = items.filter(card=>card.id!==item.id);
      connections = connections.filter(edge=>edge.from_id!==item.id && edge.to_id!==item.id);
      if (connectFrom===item.id) connectFrom=null;
      if (selected===item.id) selected=null;
      try { localStorage.removeItem(draftKey(item.id)); } catch { /* keep going */ }
      message = 'Cartão removido do quadro.';
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
      <input bind:this={imageInput} type="file" accept="image/png,image/jpeg,image/webp,image/gif" class="re-board-hidden" onchange={inputImages} aria-label="Escolher imagem para o quadro"/>
    </div>
  </div>
  {#if error}<div role="alert" class="re-board-alert"><span>{error}</span><button onclick={()=>error=''} aria-label="Fechar erro">×</button></div>{/if}
  {#if message}<div role="status" class="re-board-message"><span>{message}</span><button onclick={()=>message=''} aria-label="Fechar aviso">×</button></div>{/if}
  <div class="re-board-stage" bind:this={stage} tabindex="0" role="application" aria-label="Quadro visual: arraste o fundo para navegar, use a roda do mouse para aproximar"
    onpointerdown={startPan} onpointermove={pointerMove} onpointerup={pointerEnd} onpointercancel={pointerEnd}
    ondragover={event=>event.preventDefault()} ondrop={onDrop}>
    <div class="re-board-grid" style={'background-size: '+(28*zoom)+'px '+(28*zoom)+'px; background-position: '+panX+'px '+panY+'px'}></div>
    <div class="re-board-world" style={'transform:translate('+panX+'px,'+panY+'px) scale('+zoom+')'}>
      <svg class="re-board-edges" xmlns="http://www.w3.org/2000/svg" width="1" height="1" aria-hidden="true">
        <defs><marker id="re-board-arrow" markerWidth="8" markerHeight="8" refX="7" refY="3" orient="auto">
          <path d="M0,0 L7,3 L0,6 Z" fill="#da5578"/></marker></defs>
        {#each connections as edge (edge.id)}
          {@const source = items.find(item=>item.id===edge.from_id)}
          {@const target = items.find(item=>item.id===edge.to_id)}
          {#if source && target}
            <line x1={source.x + source.width/2} y1={source.y + source.height/2}
              x2={target.x + target.width/2} y2={target.y + target.height/2}
              stroke="#da5578" stroke-width="2.5" marker-end="url(#re-board-arrow)"/>
          {/if}
        {/each}
      </svg>
      {#each items as item (item.id)}
        <article class="re-board-card" class:re-board-selected={selected===item.id}
          class:re-board-note={item.kind==='note'} class:re-board-image={item.kind==='image'}
          class:re-board-link={item.kind==='link'} class:re-board-table={item.kind==='table'} class:re-board-connecting={connectFrom===item.id}
          style={'left:'+item.x+'px;top:'+item.y+'px;width:'+item.width+'px;height:'+item.height+'px'}>
          <div class="re-board-card-top">
            <button class="re-board-handle" title="Arrastar cartão" aria-label="Arrastar cartão"
              onpointerdown={event=>itemPointer(event,item,'move')} onclick={()=>selected=item.id}>
              <Move size={14}/> <span>{item.kind==='note'?'NOTA':item.kind==='link'?'LINK':item.kind==='image'?'IMAGEM':'TABELA'}</span>
            </button>
            <button class="re-board-connect" title={connectFrom===item.id?'Cancelar conexão':'Criar conexão'} aria-label={connectFrom===item.id?'Cancelar conexão':'Conectar cartão'} disabled={connecting || switching || loading} onclick={()=>{void chooseConnection(item.id);}}><Link2 size={14}/></button>
            <button class="re-board-remove" title="Remover cartão" aria-label="Remover cartão" disabled={switching || loading} onclick={()=>{void removeCard(item);}}><Trash2 size={14}/></button>
          </div>
          {#if item.kind==='note'}
            <textarea aria-label="Conteúdo da nota no quadro" placeholder="Escreva sua ideia..." value={item.content} disabled={switching || loading}
              oninput={event=>updateLocal(item.id,{content:event.currentTarget.value})}></textarea>
          {:else if item.kind==='link'}
            <div class="re-board-link-body"><label for={'board-link-'+item.id}>Endereço da página</label>
              <textarea id={'board-link-'+item.id} spellcheck="false" placeholder="https://..." value={item.content} disabled={switching || loading}
                oninput={event=>updateLocal(item.id,{content:event.currentTarget.value})}></textarea>
              {#if safeLink(item.content)}<a href={safeLink(item.content) ?? undefined} target="_blank" rel="noopener noreferrer">Abrir link <ExternalLink size={13}/></a>
              {:else}<small>Digite um link http:// ou https://</small>{/if}
            </div>
          {:else if item.kind==='image'}
            {#if assets[item.id]}<img src={assets[item.id]} alt={item.content || 'Imagem adicionada ao quadro'} draggable="false"/>
            {:else}<span class="re-board-image-loading">Carregando imagem...</span>{/if}
            <span class="re-board-image-name" title={item.content}>{item.content}</span>
          {:else}
            <div class="re-board-table-scroller">
              <table aria-label="Tabela editável do quadro"><tbody>
                {#each rows(item.content) as row, ri}
                  <tr>{#each row as cell, ci}
                    <td><input aria-label={'Linha '+(ri+1)+', coluna '+(ci+1)} value={cell} disabled={switching || loading}
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
            disabled={switching || loading} onpointerdown={event=>itemPointer(event,item,'resize')}></button>
        </article>
      {/each}
    </div>
    {#if loading}<div class="re-board-empty">Carregando quadro...</div>
    {:else if items.length===0}
      <div class="re-board-empty"><StickyNote size={28}/><strong>Um espaço aberto para suas ideias.</strong>
        <span>Crie uma nota, cole uma imagem ou arraste um arquivo para começar.</span>
        <button onclick={()=>{void create('note');}}><Plus size={15}/> Criar primeira nota</button>
      </div>
    {/if}
    <div class="re-board-zoom" aria-label="Controles de zoom">
      <button title="Diminuir zoom" aria-label="Diminuir zoom" onclick={()=>zoomButton(1/1.15)}><ZoomOut size={17}/></button>
      <span>{Math.round(zoom*100)}%</span>
      <button title="Aumentar zoom" aria-label="Aumentar zoom" onclick={()=>zoomButton(1.15)}><ZoomIn size={17}/></button>
      <button title="Recentralizar quadro" aria-label="Recentralizar quadro" onclick={()=>{panX=0;panY=0;zoom=1;}}><RotateCcw size={16}/></button>
    </div>
    {#if selected && connections.some(edge=>edge.from_id===selected || edge.to_id===selected)}
      <div class="re-board-connections" aria-label="Conexões do cartão selecionado">
        <strong>Conexões</strong>
        {#each connections.filter(edge=>edge.from_id===selected || edge.to_id===selected) as edge (edge.id)}
          <button title="Remover esta conexão" onclick={()=>{void removeConnection(edge.id);}}>
            <Unlink2 size={13}/> {edge.from_id===selected?'Para':'De'} {items.find(item=>item.id===(edge.from_id===selected?edge.to_id:edge.from_id))?.content.slice(0,22) || 'cartão'}
          </button>
        {/each}
      </div>
    {/if}
    <div class="re-board-help">Arraste o fundo · Roda: zoom · Use Conectar em dois cartões para criar uma seta</div>
  </div>
</section>

<style>
  .re-board-screen{display:flex;flex-direction:column;flex:1;min-height:0;background:#0c0c0f;overflow:hidden}
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
  .re-board-tools button,.re-board-empty button{display:inline-flex;gap:7px;align-items:center;justify-content:center;background:#221d23;border:1px solid #4b353d;border-radius:9px;padding:9px 12px;color:#f4ecf0;font-size:12px;font-weight:650}
  .re-board-tools button:hover,.re-board-empty button:hover{background:#432330;border-color:#ae3854}
  .re-board-hidden{display:none}
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
  @media(max-width:800px){.re-board-toolbar{padding:12px;gap:10px}.re-board-intro small{display:none}.re-board-tools button{padding:8px;font-size:11px}.re-board-help{display:none}}
</style>
