<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { NotebookPen, Check, X } from 'lucide-svelte';
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

<svelte:window onkeydown={keyboard} />
<svelte:head><title>Capturar ideia · RECALL</title></svelte:head>

<section class="capture-shell" aria-label="Captura rápida">
  <header class="capture-head" data-tauri-drag-region>
    <div class="capture-label"><span class="capture-symbol"><NotebookPen size={18}/></span><span>CAPTURA RÁPIDA</span></div>
    <button class="capture-close" title="Fechar (Esc)" aria-label="Fechar captura" onclick={closeCapture}><X size={18}/></button>
  </header>
  <div class="capture-main">
    <h1>Não perca o pensamento.</h1>
    <p>Registre sem interromper o que estava fazendo.</p>
    <div class="capture-kind" role="group" aria-label="Tipo do registro">
      <button class:active={kind === 'idea'} aria-pressed={kind === 'idea'} onclick={() => chooseKind('idea')}>Ideia</button>
      <button class:active={kind === 'task'} aria-pressed={kind === 'task'} onclick={() => chooseKind('task')}>Tarefa</button>
    </div>
    <textarea bind:this={textarea} bind:value={content} oninput={rememberDraft} aria-label="Escreva sua ideia" placeholder="O que você quer lembrar?" maxlength="100000" spellcheck="true"></textarea>
    {#if kind === 'task' && suggestion.recognized}<p class="capture-suggestion">{suggestion.label} · sem notificação automática</p>{/if}
    {#if failure}<div class="capture-error" role="alert">{failure}</div>{/if}
  </div>
  <footer class="capture-footer">
    <span><kbd>Ctrl</kbd> + <kbd>Enter</kbd> para salvar</span>
    <button class="capture-save" disabled={!content.trim() || busy} onclick={save}>{#if busy}<span>Salvando...</span>{:else}<Check size={17}/> {kind === 'task' ? 'Criar tarefa' : 'Guardar ideia'}{/if}</button>
  </footer>
</section>
