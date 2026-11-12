<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import {
    NotebookPen, Search, Plus, Pin, Bell, Presentation, Target, ArrowUpRight,
    Trash2, ChevronLeft, ChevronRight, X, Check, Menu, MoreHorizontal,
    StickyNote, FileText, CircleHelp, CalendarClock, PencilLine, Zap, Sun, Inbox as InboxIcon
  } from 'lucide-svelte';
  import { notesApi, formatDateTime, toLocalInput } from '../lib/api';
  import { openSticky, openQuickCapture } from '../lib/windows';
  import type { Note } from '../lib/types';
  import Today from './Today.svelte';
  import Inbox from './Inbox.svelte';

  type Tab = 'today' | 'inbox' | 'all' | 'pinned' | 'reminders' | 'presentation';
  let tab = $state<Tab>('today');
  let notes = $state<Note[]>([]);
  let selectedId = $state<string | null>(null);
  let search = $state('');
  let loading = $state(true);
  let saving = $state(false);
  let error = $state('');
  let focusMode = $state(false);
  let presentationMode = $state(false);
  let slideIndex = $state(0);
  let sidebarOpen = $state(true);
  let menuOpen = $state(false);
  let reminderOpen = $state(false);
  let helpOpen = $state(false);

  const dirty = new Map<string, { title: string; body: string }>();
  const saveTimers = new Map<string, ReturnType<typeof setTimeout>>();
  const writing = new Map<string, Promise<void>>();
  const tabNames: Record<Tab, string> = { today: 'Hoje', inbox: 'Caixa de entrada', all: 'Todas as notas', pinned: 'Fixadas', reminders: 'Lembretes', presentation: 'Apresentações' };
  let active = $derived(notes.find((n) => n.id === selectedId) ?? null);
  let visible = $derived(notes
    .filter((n) => tab === 'all' || (tab === 'pinned' && n.pinned) || (tab === 'reminders' && n.remind_at !== null) || (tab === 'presentation' && n.kind === 'presentation'))
    .filter((n) => `${n.title} ${n.body}`.toLowerCase().includes(search.toLowerCase()))
    .sort((a, b) => Number(b.pinned) - Number(a.pinned) || b.updated_at - a.updated_at));
  let slides = $derived(active ? active.body.split(/^\s*---\s*$/m).map(s => s.trim()).filter(Boolean) : []);

  function label(n: Note) { return n.title.trim() || (n.kind === 'presentation' ? 'Nova apresentação' : 'Nota sem título'); }
  function tabMatch(n: Note, view: Tab) {
    return view === 'all' || (view === 'pinned' && n.pinned) || (view === 'reminders' && n.remind_at !== null) || (view === 'presentation' && n.kind === 'presentation');
  }
  async function refresh() {
    try {
      const fromDb = await notesApi.list();
      notes = fromDb.map(n => dirty.has(n.id) ? { ...n, ...dirty.get(n.id)! } : n);
      if (selectedId && !notes.some(n => n.id === selectedId)) selectedId = null;
      if (!selectedId && notes.length) selectedId = notes.find(n => tabMatch(n, tab))?.id ?? null;
      error = '';
    } catch (e) { error = `Não foi possível carregar as notas: ${String(e)}`; }
    finally { loading = false; }
  }
  onMount(() => {
    void refresh();
    let dispose: (() => void) | undefined;
    void listen('notes:changed', () => { void refresh(); }).then(fn => dispose = fn);
    const onKey = (event: KeyboardEvent) => {
      if (event.ctrlKey && event.key.toLowerCase() === 'n') { event.preventDefault(); void create(tab === 'presentation' ? 'presentation' : 'note'); }
      if (event.key === 'Escape') { focusMode = false; presentationMode = false; menuOpen = false; reminderOpen = false; helpOpen = false; }
      if (presentationMode && event.key === 'ArrowRight' && slideIndex < slides.length - 1) slideIndex++;
      if (presentationMode && event.key === 'ArrowLeft' && slideIndex > 0) slideIndex--;
    };
    window.addEventListener('keydown', onKey);
    return () => {
      dispose?.(); window.removeEventListener('keydown', onKey);
      saveTimers.forEach(timer => clearTimeout(timer));
      for (const id of dirty.keys()) void flush(id);
    };
  });
  async function create(kind: 'note' | 'presentation') {
    try {
      await flushAll();
      const note = await notesApi.create(kind);
      notes = [note, ...notes];
      tab = kind === 'presentation' ? 'presentation' : 'all';
      selectedId = note.id;
      focusMode = false; presentationMode = false; reminderOpen = false; menuOpen = false; search = '';
    } catch (e) { error = `Falha ao criar: ${String(e)}`; }
  }
  function edit(field: 'title' | 'body', value: string) {
    if (!active) return;
    const id = active.id;
    const next = { title: active.title, body: active.body, [field]: value };
    notes = notes.map(n => n.id === id ? { ...n, ...next } : n);
    dirty.set(id, next);
    if (saveTimers.has(id)) clearTimeout(saveTimers.get(id));
    saveTimers.set(id, setTimeout(() => { void flush(id); }, 650));
  }
  async function flush(id: string) {
    const previous = writing.get(id);
    if (previous) await previous;
    const payload = dirty.get(id);
    if (!payload) return;
    dirty.delete(id);
    clearTimeout(saveTimers.get(id)); saveTimers.delete(id);
    const task = (async () => {
      saving = true;
      try { await notesApi.save(id, payload.title, payload.body); }
      catch (e) {
        if (!dirty.has(id)) dirty.set(id, payload);
        error = `Não foi possível salvar: ${String(e)}`;
      } finally { saving = false; }
    })();
    writing.set(id, task); await task;
    if (writing.get(id) === task) writing.delete(id);
    if (dirty.has(id) && !saveTimers.has(id)) saveTimers.set(id, setTimeout(() => { void flush(id); }, 650));
  }
  async function flushAll() { for (const id of [...dirty.keys()]) await flush(id); }
  async function select(id: string) {
    if (selectedId) await flush(selectedId);
    selectedId = id; presentationMode = false; focusMode = false; slideIndex = 0; reminderOpen = false; menuOpen = false;
  }
  async function changeTab(next: Tab) {
    if (selectedId) await flush(selectedId);
    tab = next; search = ''; selectedId = notes.find(n => tabMatch(n, next))?.id ?? null;
    focusMode = false; presentationMode = false; menuOpen = false; reminderOpen = false;
  }
  async function pin() {
    if (!active) return;
    try { const n = await notesApi.pin(active.id, !active.pinned); notes = notes.map(x => x.id === n.id ? n : x); }
    catch (e) { error = String(e); }
  }
  async function remind(value: string) {
    if (!active) return;
    const timestamp = value ? new Date(value).getTime() : null;
    if (timestamp !== null && !Number.isFinite(timestamp)) return;
    try { const n = await notesApi.remind(active.id, timestamp); notes = notes.map(x => x.id === n.id ? n : x); }
    catch (e) { error = String(e); }
  }
  async function remove() {
    if (!active || !confirm(`Excluir “${label(active)}”?`)) return;
    const id = active.id;
    try {
      await notesApi.delete(id);
      dirty.delete(id);
      if (saveTimers.has(id)) clearTimeout(saveTimers.get(id));
      saveTimers.delete(id);
      notes = notes.filter(n => n.id !== id);
      selectedId = notes.find(n => tabMatch(n, tab))?.id ?? null;
      menuOpen = false; focusMode = false; presentationMode = false;
    } catch (e) { error = String(e); }
  }
  async function floatNote() {
    if (!active) return;
    try { await flush(active.id); await openSticky(active.id); menuOpen = false; }
    catch (e) { error = `Não foi possível abrir o cartão: ${String(e)}`; }
  }
  async function launchQuickCapture() {
    try { await openQuickCapture(); }
    catch (e) { error = `Não foi possível abrir a captura rápida: ${String(e)}`; }
  }
  function enterPresentation() { slideIndex = 0; presentationMode = true; }
</script>

<svelte:head><title>Memória Externa</title></svelte:head>

<div class="app-shell" class:sidebar-collapsed={!sidebarOpen}>
  <aside class="sidebar" aria-label="Navegação principal">
    <div class="brand"><div class="brand-icon"><NotebookPen size={20} strokeWidth={1.8}/></div><div class="brand-copy"><strong>memória<span>.</span></strong><small>EXTERNA</small></div></div>
    <nav class="navigation">
      <button class:current={tab === 'today'} onclick={() => changeTab('today')}><Sun size={18}/> Hoje</button>
      <button class:current={tab === 'inbox'} onclick={() => changeTab('inbox')}><InboxIcon size={18}/> Entrada</button>
      <button class:current={tab === 'all'} onclick={() => changeTab('all')}><StickyNote size={18}/> Notas</button>
      <button class:current={tab === 'pinned'} onclick={() => changeTab('pinned')}><Pin size={18}/> Fixadas</button>
      <button class:current={tab === 'reminders'} onclick={() => changeTab('reminders')}><Bell size={18}/> Lembretes</button>
      <button class:current={tab === 'presentation'} onclick={() => changeTab('presentation')}><Presentation size={18}/> Apresentações</button>
    </nav>
    <div class="sidebar-bottom"><span class="local-indicator"></span> Dados locais <span class="version-tag">v0.5 α</span></div>
  </aside>

  <main class="main-frame">
    <header class="app-toolbar">
      <button class="icon-btn menu-toggle" aria-label="Mostrar ou esconder menu" title="Mostrar ou esconder menu" onclick={() => sidebarOpen = !sidebarOpen}><Menu size={19}/></button>
      <h1>{focusMode ? 'Modo foco' : presentationMode ? 'Apresentação' : tabNames[tab]}</h1>
      {#if !focusMode && !presentationMode && tab !== 'today' && tab !== 'inbox'}<span class="total-count">{visible.length}</span>{/if}
      <div class="toolbar-spacer"></div>
      {#if focusMode || presentationMode}
        <button class="quiet-btn" onclick={() => { focusMode = false; presentationMode = false; }}><X size={16}/> Voltar</button>
      {:else}
        <button class="capture-toolbar" onclick={launchQuickCapture} title="Capturar ideia (Ctrl+Shift+Espaço)"><Zap size={16}/> Capturar <span>Ctrl+Shift+Espaço</span></button>
        <button class="icon-btn help-trigger" aria-label="Ajuda" title="Como usar" onclick={() => helpOpen = !helpOpen}><CircleHelp size={18}/></button>
        {#if tab !== 'today' && tab !== 'inbox'}<button class="create-btn" onclick={() => create(tab === 'presentation' ? 'presentation' : 'note')}><Plus size={18}/> {tab === 'presentation' ? 'Nova apresentação' : 'Nova nota'}</button>{/if}
      {/if}
    </header>
    {#if error}<div class="notice error-notice" role="alert">{error}<button aria-label="Fechar aviso" onclick={() => error = ''}><X size={15}/></button></div>{/if}
    {#if helpOpen}<div class="notice">Crie uma nota e ela será salva sozinha. A captura rápida guarda ideias na Entrada para organizar depois. Clique no ícone de janela para deixá-la flutuando no Windows. Use Ctrl+N para uma nova nota ou Ctrl+Shift+Espaço para uma captura rápida, mesmo com a janela principal minimizada.<button aria-label="Fechar ajuda" onclick={() => helpOpen = false}><X size={15}/></button></div>{/if}

    {#if focusMode && active}
      <div class="focus-view">
        <div class="focus-mini"><Target size={17}/> Uma coisa de cada vez</div>
        <input class="focus-title" aria-label="Título" placeholder="No que você está focando?" value={active.title} oninput={(e) => edit('title', e.currentTarget.value)}/>
        <textarea class="focus-body" aria-label="Anotação" placeholder="Escreva seu próximo passo..." value={active.body} oninput={(e) => edit('body', e.currentTarget.value)}></textarea>
        <div class="focus-save"><Check size={14}/> {saving ? 'Salvando...' : 'Salvo automaticamente'}</div>
      </div>
    {:else if presentationMode && active}
      <div class="presentation-view">
        <div class="presentation-counter">CARTÃO {Math.min(slideIndex + 1, Math.max(slides.length, 1))} DE {Math.max(slides.length, 1)}</div>
        <div class="presentation-body">{slides[slideIndex] || 'Escreva uma etapa por cartão, separando as partes com uma linha contendo ---'}</div>
        <div class="presentation-nav"><button disabled={slideIndex <= 0} onclick={() => slideIndex--}><ChevronLeft size={18}/> Anterior</button><div class="presentation-track">{#each slides as _, i}<span class:active={i === slideIndex}></span>{/each}</div><button disabled={slideIndex >= slides.length - 1} onclick={() => slideIndex++}>Próximo <ChevronRight size={18}/></button></div>
      </div>
    {:else if tab === 'today'}
      <Today />
    {:else if tab === 'inbox'}
      <Inbox onNoteCreated={(id) => { void refresh().then(() => { tab = 'all'; selectedId = id; }); }} onTaskCreated={() => { tab = 'today'; }}/>
    {:else}
      <div class="content-layout">
        <section class="list-pane" aria-label="Lista de anotações">
          <label class="search-field"><Search size={17}/><input aria-label="Buscar anotações" placeholder="Buscar notas..." bind:value={search}/></label>
          <div class="note-list">
            {#if loading}<p class="list-placeholder">Carregando...</p>
            {:else if visible.length === 0}
              <div class="list-empty"><StickyNote size={25}/><strong>{search ? 'Nenhum resultado' : 'Nada por aqui ainda'}</strong><p>{search ? 'Tente outra palavra.' : 'Comece com uma anotação.'}</p></div>
            {:else}
              {#each visible as note (note.id)}
                <button class="note-item" class:selected={selectedId === note.id} onclick={() => select(note.id)}>
                  <div class="note-item-top"><strong>{label(note)}</strong>{#if note.pinned}<Pin size={13} class="item-pin"/>{/if}</div>
                  <div class="note-item-body">{note.body.trim() || 'Clique para começar a escrever...'}</div>
                  <div class="note-item-bottom"><span>{formatDateTime(note.updated_at)}</span>{#if note.remind_at !== null}<Bell size={13} aria-label="Com lembrete"/>{/if}{#if note.kind === 'presentation'}<Presentation size={13} aria-label="Apresentação"/>{/if}</div>
                </button>
              {/each}
            {/if}
          </div>
          <div class="list-footer">{visible.length} {visible.length === 1 ? 'item' : 'itens'}</div>
        </section>

        <section class="editor-pane" aria-label="Editor de anotação">
          {#if active}
            <div class="editor-bar"><span><span class="saved-dot"></span>{saving ? 'Salvando...' : 'Salvo automaticamente'}</span>
              <div class="editor-buttons">
                <button class="icon-btn" class:icon-active={active.pinned} onclick={pin} aria-label={active.pinned ? 'Desafixar nota' : 'Fixar nota'} title={active.pinned ? 'Desafixar' : 'Fixar'}><Pin size={18}/></button>
                <button class="icon-btn" onclick={floatNote} aria-label="Abrir nota em janela flutuante" title="Abrir em janela flutuante"><ArrowUpRight size={19}/></button>
                <div class="more-wrap"><button class="icon-btn" aria-label="Mais opções" aria-expanded={menuOpen} onclick={() => menuOpen = !menuOpen}><MoreHorizontal size={19}/></button>{#if menuOpen}<div class="more-popover"><button class="danger-option" onclick={remove}><Trash2 size={16}/> Excluir nota</button></div>{/if}</div>
              </div>
            </div>
            <div class="editor-writing">
              <input class="editor-title" aria-label="Título da nota" placeholder="Sem título" value={active.title} oninput={(e) => edit('title', e.currentTarget.value)}/>
              <textarea class="editor-body" aria-label="Conteúdo da nota" placeholder={active.kind === 'presentation' ? 'Uma ideia por vez.\n\nPrimeiro cartão...\n\n---\n\nSegundo cartão...' : 'Escreva o que precisa lembrar...'} value={active.body} oninput={(e) => edit('body', e.currentTarget.value)}></textarea>
              {#if active.kind === 'presentation'}<div class="presentation-hint">Para criar outro cartão, escreva <code>---</code> em uma linha separada.</div>{/if}
            </div>
            <div class="editor-bottom">
              <div class="reminder-wrap"><button class="text-action" onclick={() => reminderOpen = !reminderOpen} aria-expanded={reminderOpen}><Bell size={17}/>{active.remind_at ? `Lembrete: ${formatDateTime(active.remind_at)}` : 'Adicionar lembrete'}</button>
                {#if reminderOpen}<div class="reminder-popover"><label for="reminder-date">Data e horário</label><div class="reminder-row"><input id="reminder-date" type="datetime-local" value={toLocalInput(active.remind_at)} onchange={(e) => remind(e.currentTarget.value)}/>{#if active.remind_at}<button class="clear-reminder" title="Remover lembrete" onclick={() => {void remind(''); reminderOpen = false;}}><X size={17}/></button>{/if}</div></div>{/if}
              </div>
              <div class="bottom-actions">{#if active.kind === 'presentation'}<button class="text-action" onclick={enterPresentation}><Presentation size={17}/> Apresentar</button>{/if}<button class="text-action" onclick={() => focusMode = true}><Target size={17}/> Focar</button></div>
            </div>
          {:else}
            <div class="editor-empty"><div class="empty-icon"><PencilLine size={25}/></div><h2>Guarde uma ideia.</h2><p>Crie uma anotação. Você pode organizar depois.</p><button class="create-btn" onclick={() => create(tab === 'presentation' ? 'presentation' : 'note')}><Plus size={18}/> Criar {tab === 'presentation' ? 'apresentação' : 'nota'}</button></div>
          {/if}
        </section>
      </div>
    {/if}
  </main>
</div>
