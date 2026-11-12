<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { Grip, X, Check, StickyNote } from 'lucide-svelte';
  import { notesApi } from '../lib/api';
  import type { Note } from '../lib/types';
  let { id }: { id: string } = $props();
  let note = $state<Note | null>(null);
  let error = $state('');
  let saving = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let dirty = false;
  async function load() {
    try {
      const value = await notesApi.get(id);
      if (value === null) { error = 'Essa nota não existe mais.'; return; }
      if (!dirty) note = value;
    } catch (e) { error = String(e); }
  }
  onMount(() => {
    void load();
    let unlisten: (() => void) | undefined;
    void listen<string>('notes:changed', event => { if (event.payload === id) void load(); }).then(fn => unlisten = fn);
    return () => { unlisten?.(); if (timer) { clearTimeout(timer); void persist(); } };
  });
  function edit(field: 'title' | 'body', value: string) {
    if (!note) return;
    note = { ...note, [field]: value };
    dirty = true;
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => { void persist(); }, 600);
  }
  async function persist() {
    if (!note || !dirty) return;
    const snapshot = { title: note.title, body: note.body };
    dirty = false;
    saving = true;
    try { await notesApi.save(id, snapshot.title, snapshot.body); error = ''; }
    catch (e) { error = String(e); dirty = true; }
    finally { saving = false; }
  }
  async function close() { await persist(); await getCurrentWindow().close(); }
</script>

<svelte:head><title>Nota flutuante — Memória Externa</title></svelte:head>
<div class="sticky-shell">
  <header class="sticky-header" data-tauri-drag-region><div data-tauri-drag-region><Grip size={15}/><span data-tauri-drag-region>NOTA FLUTUANTE</span></div><button title="Fechar cartão" aria-label="Fechar cartão" onclick={close}><X size={17}/></button></header>
  {#if note}
    <div class="sticky-content"><input aria-label="Título da nota flutuante" value={note.title} placeholder="Título da nota" oninput={(e) => edit('title', e.currentTarget.value)}/><textarea aria-label="Conteúdo da nota flutuante" value={note.body} placeholder="Sua anotação..." oninput={(e) => edit('body', e.currentTarget.value)}></textarea></div>
    <footer class="sticky-footer"><div><StickyNote size={14}/> Memória Externa</div><span><Check size={12}/> {saving ? 'Salvando' : dirty ? 'Editando' : 'Salva'}</span></footer>
  {:else}<div class="sticky-content loading-sticky">Carregando cartão...</div>{/if}
  {#if error}<div class="sticky-error" role="alert">{error}</div>{/if}
</div>
