<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { Grip, X, Check, StickyNote, Copy } from 'lucide-svelte';
  import { notesApi } from '../lib/api';
  import type { Note } from '../lib/types';

  let { id }: { id: string } = $props();
  let note = $state<Note | null>(null);
  let error = $state('');
  let conflict = $state(false);
  let saving = $state(false);
  let dirty = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let saveInFlight: Promise<boolean> | null = null;
  const windowHandle = getCurrentWindow();
  const draftKey = 'recall-sticky-draft:' + id;

  function preserveDraft() {
    if (!note) return;
    try {
      localStorage.setItem(draftKey,
        JSON.stringify({ title: note.title, body: note.body, revision: note.revision }));
    } catch { /* SQLite is authoritative; local recovery is best effort. */ }
  }

  function clearDraft() {
    try { localStorage.removeItem(draftKey); } catch { /* Storage unavailable. */ }
  }

  async function load() {
    try {
      const value = await notesApi.get(id);
      if (value === null) { error = 'Essa nota não existe mais.'; return; }
      // Never discard an active edit because an event arrived from another window.
      if (dirty || saving || saveInFlight) return;
      let restored = false;
      try {
        const raw = localStorage.getItem(draftKey);
        if (raw) {
          const cached = JSON.parse(raw) as { title?: string; body?: string; revision?: number };
          if (typeof cached.title === 'string' && typeof cached.body === 'string'
            && (cached.title !== value.title || cached.body !== value.body)) {
            note = {
              ...value,
              title: cached.title,
              body: cached.body,
              revision: typeof cached.revision === 'number' ? cached.revision : value.revision
            };
            dirty = true;
            restored = true;
            error = 'Rascunho recuperado. Confira antes de continuar.';
          } else { clearDraft(); }
        }
      } catch { /* Leave invalid stored drafts for manual recovery. */ }
      if (!restored) { note = value; conflict = false; }
    } catch (e) { error = 'Não foi possível carregar a nota: ' + String(e); }
  }

  onMount(() => {
    void load();
    let unlisten: (() => void) | undefined;
    let alive = true;
    void listen<string>('notes:changed', event => { if (alive && event.payload === id) void load(); })
      .then(fn => { if (alive) unlisten = fn; else fn(); });
    return () => {
      alive = false;
      unlisten?.();
      if (timer) clearTimeout(timer);
      if (dirty) { preserveDraft(); void persist(); }
    };
  });

  function edit(field: 'title' | 'body', value: string) {
    if (!note) return;
    note = { ...note, [field]: value };
    dirty = true;
    preserveDraft();
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => { void persist(); }, 650);
  }

  async function persist(): Promise<boolean> {
    if (timer) { clearTimeout(timer); timer = undefined; }
    if (saveInFlight && !(await saveInFlight)) return false;
    if (!note || !dirty) return true;
    if (note.title.length > 250 || note.body.length > 100_000) {
      error = 'Título ou corpo da nota excede o limite. Corrija o texto antes de fechar.';
      preserveDraft();
      return false;
    }
    const snapshot = { title: note.title, body: note.body, revision: note.revision };
    dirty = false;
    saving = true;
    const writing: Promise<boolean> = notesApi.save(id, snapshot.title, snapshot.body, snapshot.revision)
      .then(updated => {
        if (note) note = { ...note, revision: updated.revision, updated_at: updated.updated_at };
        if (!dirty) clearDraft();
        conflict = false;
        error = '';
        return true;
      })
      .catch(e => {
        dirty = true;
        preserveDraft();
        conflict = String(e).includes('CONFLITO_REVISAO');
        error = conflict
          ? 'Outra janela alterou esta nota. Seu texto foi preservado. Salve uma cópia para não sobrescrever nada.'
          : 'Não foi possível salvar. Seu cartão e rascunho permanecem abertos: ' + String(e);
        return false;
      });
    saveInFlight = writing;
    const ok = await writing;
    if (saveInFlight === writing) saveInFlight = null;
    saving = false;
    if (!ok) return false;
    return dirty ? persist() : true;
  }

  async function saveCopyAndClose() {
    if (!note || saving) return;
    if (saveInFlight) await saveInFlight;
    try {
      await notesApi.copy(note.title, note.body, note.kind);
      clearDraft();
      dirty = false;
      conflict = false;
      error = '';
      await windowHandle.hide();
    } catch (e) {
      error = 'Falha ao criar cópia. O texto continua aqui: ' + String(e);
      preserveDraft();
    }
  }

  async function close() {
    if (await persist()) await windowHandle.hide();
  }
</script>

<svelte:head><title>Nota flutuante — RECALL</title></svelte:head>
<div class="sticky-shell">
  <header class="sticky-header" data-tauri-drag-region>
    <div data-tauri-drag-region><Grip size={15}/><span data-tauri-drag-region>NOTA FLUTUANTE</span></div>
    <button title="Fechar cartão" aria-label="Fechar cartão" onclick={() => { void close(); }}><X size={17}/></button>
  </header>
  {#if note}
    <div class="sticky-content">
      <input aria-label="Título da nota flutuante" value={note.title} placeholder="Título da nota" maxlength="250"
        oninput={(e) => edit('title', e.currentTarget.value)}/>
      <textarea aria-label="Conteúdo da nota flutuante" value={note.body} placeholder="Sua anotação..." maxlength="100000"
        oninput={(e) => edit('body', e.currentTarget.value)}></textarea>
    </div>
    <footer class="sticky-footer">
      <div><StickyNote size={14}/> RECALL</div>
      <span aria-live="polite"><Check size={12}/>{conflict ? 'Conflito' : saving ? 'Salvando' : dirty ? 'Não salva' : 'Salva'}</span>
    </footer>
  {:else}
    <div class="sticky-content loading-sticky">Carregando cartão...</div>
  {/if}
  {#if error}
    <div class="sticky-error" role="alert">{error}
      {#if conflict}
        <button class="sticky-copy-button" onclick={() => { void saveCopyAndClose(); }}><Copy size={14}/> Salvar meu texto como nova nota e fechar</button>
      {/if}
    </div>
  {/if}
</div>
