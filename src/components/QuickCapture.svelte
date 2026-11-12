<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { NotebookPen, Check, X } from 'lucide-svelte';
  import { notesApi } from '../lib/api';

  let content = $state('');
  let busy = $state(false);
  let failure = $state('');
  let textarea: HTMLTextAreaElement;
  const windowHandle = getCurrentWindow();
  const DRAFT_KEY = 'memoria-externa-quick-capture-draft-v1';
  function rememberDraft() { try { localStorage.setItem(DRAFT_KEY, content); } catch { /* Private browsing can disable storage. */ } }

  onMount(() => {
    try { content = localStorage.getItem(DRAFT_KEY) ?? ''; } catch { /* Local storage unavailable. */ }
    let unlisten: (() => void) | undefined;
    let alive = true;
    void listen('quick-capture:opened', () => { setTimeout(() => textarea?.focus(), 50); })
      .then(fn => { if (alive) unlisten = fn; else fn(); });
    const focus = () => textarea?.focus();
    window.addEventListener('focus', focus);
    setTimeout(focus, 80);
    return () => { alive = false; unlisten?.(); window.removeEventListener('focus', focus); };
  });

  async function closeCapture() {
    failure = '';
    await windowHandle.hide();
  }

  async function save() {
    if (busy || !content.trim()) return;
    busy = true; failure = '';
    try {
      await notesApi.capture(content);
      content = '';
      try { localStorage.removeItem(DRAFT_KEY); } catch { /* Storage unavailable. */ }
      await windowHandle.hide();
    } catch (error) {
      failure = `Sua nota não foi salva. ${String(error)}`;
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
<svelte:head><title>Capturar ideia · Memória Externa</title></svelte:head>

<section class="capture-shell" aria-label="Captura rápida">
  <header class="capture-head" data-tauri-drag-region>
    <div class="capture-label"><span class="capture-symbol"><NotebookPen size={18}/></span><span>CAPTURA RÁPIDA</span></div>
    <button class="capture-close" title="Fechar (Esc)" aria-label="Fechar captura" onclick={closeCapture}><X size={18}/></button>
  </header>
  <div class="capture-main">
    <h1>Não perca a ideia.</h1>
    <p>Escreva agora. Organize depois.</p>
    <textarea bind:this={textarea} bind:value={content} oninput={rememberDraft} aria-label="Escreva sua ideia" placeholder="O que você quer lembrar?" maxlength="100000" spellcheck="true"></textarea>
    {#if failure}<div class="capture-error" role="alert">{failure}</div>{/if}
  </div>
  <footer class="capture-footer">
    <span><kbd>Ctrl</kbd> + <kbd>Enter</kbd> para salvar</span>
    <button class="capture-save" disabled={!content.trim() || busy} onclick={save}>{#if busy}<span>Salvando...</span>{:else}<Check size={17}/> Salvar nota{/if}</button>
  </footer>
</section>
