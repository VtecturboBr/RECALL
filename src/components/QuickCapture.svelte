<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { NotebookPen, Check, X, Sparkles, CheckCircle2, CalendarDays, Command } from 'lucide-svelte';
  import { inboxApi, tasksApi } from '../lib/api';
  import { parseTaskCapture } from '../lib/smartCapture';

  let content = $state('');
  let kind = $state<'idea' | 'task'>('idea');
  let suggestion = $derived(parseTaskCapture(content));
  const KIND_KEY = 'recall-quick-capture-kind';
  let busy = $state(false);
  let failure = $state('');
  let textarea: HTMLTextAreaElement;
  const windowHandle = getCurrentWindow();
  const DRAFT_KEY = 'memoria-externa-quick-capture-draft-v1';
  function rememberDraft() { try { localStorage.setItem(DRAFT_KEY, content); } catch { /* Private browsing can disable storage. */ } }

  onMount(() => {
    try {
      content = localStorage.getItem(DRAFT_KEY) ?? '';
      kind = localStorage.getItem(KIND_KEY) === 'task' ? 'task' : 'idea';
    } catch { /* Local storage unavailable. */ }
    let unlisten: (() => void) | undefined;
    let alive = true;
    void listen('quick-capture:opened', () => { setTimeout(() => textarea?.focus(), 50); })
      .then(fn => { if (alive) unlisten = fn; else fn(); });
    const focus = () => textarea?.focus();
    window.addEventListener('focus', focus);
    setTimeout(focus, 80);
    return () => { alive = false; unlisten?.(); window.removeEventListener('focus', focus); };
  });

  function chooseKind(next: 'idea' | 'task') {
    kind = next;
    try { localStorage.setItem(KIND_KEY, next); } catch { /* Storage unavailable. */ }
  }

  async function closeCapture() {
    failure = '';
    await windowHandle.hide();
  }

  async function save() {
    if (busy || !content.trim()) return;
    busy = true; failure = '';
    try {
      if (kind === 'task') {
        const draft = parseTaskCapture(content, new Date());
        if (!draft.title.trim() || draft.title.length > 250) throw new Error('Use um título de até 250 caracteres.');
        await tasksApi.createPlanned(draft.title, draft.details, draft.bucket, draft.plannedFor, draft.dueAt);
      } else {
        await inboxApi.capture(content);
      }
      content = '';
      try { localStorage.removeItem(DRAFT_KEY); } catch { /* Storage unavailable. */ }
      await windowHandle.hide();
    } catch (error) {
      failure = `Não foi possível salvar. ${String(error)}`;
    } finally {
      busy = false;
    }
  }

  function keyboard(event: KeyboardEvent) {
    if (event.key === 'Escape') { event.preventDefault(); void closeCapture(); }
    if (event.key === 'Enter' && event.ctrlKey) { event.preventDefault(); void save(); }
  }
</script>

<svelte:head><title>Captura rápida — RECALL</title></svelte:head>
<section class="re-cap" aria-label="Captura rápida">
  <header class="re-cap-header" data-tauri-drag-region>
    <div class="re-cap-brand" data-tauri-drag-region>
      <span class="re-cap-mark" data-tauri-drag-region><NotebookPen size={17}/></span>
      <span data-tauri-drag-region>RECALL <small data-tauri-drag-region>/ CAPTURA RÁPIDA</small></span>
    </div>
    <button aria-label="Fechar janela de captura" title="Fechar (Esc)" onclick={closeCapture}><X size={18}/></button>
  </header>
  <div class="re-cap-body">
    <h1>Guarde o pensamento<span>.</span></h1>
    <p>Capture agora. Organize quando quiser.</p>
    <div class="re-cap-tabs" role="group" aria-label="Tipo de registro">
      <button class:chosen={kind === 'idea'} aria-pressed={kind === 'idea'} onclick={() => chooseKind('idea')}><Sparkles size={15}/> Ideia</button>
      <button class:chosen={kind === 'task'} aria-pressed={kind === 'task'} onclick={() => chooseKind('task')}><CheckCircle2 size={15}/> Tarefa</button>
    </div>
    <label class="re-cap-input-wrap">
      <span class="re-visually-hidden">Texto a capturar</span>
      <textarea bind:this={textarea} bind:value={content} oninput={rememberDraft} maxlength="100000" spellcheck="true" placeholder={kind === 'task' ? 'Qual é a próxima ação?' : 'O que você quer lembrar?'}></textarea>
      {#if kind === 'task' && suggestion.recognized}
        <span class="re-cap-hint"><CalendarDays size={13}/>{suggestion.label} · sem notificação automática</span>
      {/if}
    </label>
    {#if failure}<div class="re-cap-error" role="alert">{failure}</div>{/if}
  </div>
  <footer class="re-cap-footer">
    <span><Command size={13}/> <kbd>Ctrl</kbd> + <kbd>Enter</kbd></span>
    <button class="re-cap-save" disabled={busy || !content.trim()} onclick={save}>
      {#if busy}Salvando...{:else}<Check size={16}/>{kind === 'task' ? 'Criar tarefa' : 'Guardar ideia'}{/if}
    </button>
  </footer>
</section>
