<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { Inbox, Plus, NotebookPen, CheckCircle2, Trash2, ArrowRight } from 'lucide-svelte';
  import { inboxApi, formatDateTime } from '../lib/api';
  import type { InboxItem } from '../lib/types';

  let { onNoteCreated, onTaskCreated }: { onNoteCreated: (id: string) => void; onTaskCreated: () => void } = $props();
  let items = $state<InboxItem[]>([]);
  let idea = $state('');
  let loading = $state(true);
  let busy = $state<string | null>(null);
  let error = $state('');

  async function refresh() {
    try { items = await inboxApi.list(); error = ''; }
    catch (e) { error = 'Não foi possível carregar as ideias: ' + String(e); }
    finally { loading = false; }
  }
  onMount(() => {
    let alive = true;
    let dispose: (() => void) | undefined;
    void refresh();
    void listen('inbox:changed', () => { if (alive) void refresh(); })
      .then(fn => { if (alive) dispose = fn; else fn(); });
    return () => { alive = false; dispose?.(); };
  });

  async function save() {
    if (!idea.trim() || busy) return;
    busy = 'new';
    try {
      await inboxApi.capture(idea);
      idea = '';
      await refresh();
    } catch (e) { error = 'Falha ao guardar a ideia: ' + String(e); }
    finally { busy = null; }
  }

  async function process(item: InboxItem, action: 'note' | 'task' | 'dismiss') {
    if (busy) return;
    if (action === 'dismiss' && !confirm('Descartar esta ideia?')) return;
    busy = item.id;
    try {
      if (action === 'note') {
        const note = await inboxApi.toNote(item.id);
        await refresh();
        onNoteCreated(note.id);
      } else if (action === 'task') {
        await inboxApi.toTask(item.id);
        await refresh();
        onTaskCreated();
      } else {
        await inboxApi.dismiss(item.id);
        await refresh();
      }
    } catch (e) { error = 'Não foi possível organizar esta ideia: ' + String(e); }
    finally { busy = null; }
  }
</script>

<section class="v05-page" aria-label="Caixa de entrada">
  <div class="v05-content">
    <header class="v05-intro">
      <p class="v05-eyebrow">ESCREVA AGORA · ORGANIZE DEPOIS</p>
      <h2>Caixa de entrada</h2>
      <p>Um lugar para guardar pensamentos sem precisar decidir o que fazer com eles.</p>
    </header>
    <form class="v05-idea-form" onsubmit={(event) => { event.preventDefault(); void save(); }}>
      <textarea rows="3" maxlength="100000" aria-label="Ideia rápida" placeholder="O que você não quer esquecer?" bind:value={idea}></textarea>
      <div class="v05-form-actions"><span>A captura rápida também salva aqui.</span><button type="submit" class="v05-primary" disabled={!idea.trim() || !!busy}><Plus size={16}/> Guardar ideia</button></div>
    </form>
    {#if error}<p class="v05-error" role="alert">{error}</p>{/if}
    {#if loading}<p class="v05-muted">Carregando...</p>
    {:else if !items.length}
      <div class="v05-empty"><Inbox size={24}/><h3>Tudo organizado por aqui.</h3><p>As ideias que você capturar aparecerão neste espaço.</p></div>
    {:else}
      <section class="v05-list-section" aria-label="Ideias não organizadas">
        <h3>Para organizar <span>{items.length}</span></h3>
        {#each items as item (item.id)}
          <article class="v05-inbox-item">
            <p class="v05-inbox-text">{item.text}</p>
            <small>{formatDateTime(item.created_at)}</small>
            <div class="v05-inbox-actions">
              <button disabled={!!busy} onclick={() => { void process(item, 'task'); }}><CheckCircle2 size={15}/> Virar tarefa <ArrowRight size={13}/></button>
              <button disabled={!!busy} onclick={() => { void process(item, 'note'); }}><NotebookPen size={15}/> Virar nota</button>
              <button class="v05-icon-btn v05-discard" disabled={!!busy} aria-label="Descartar ideia" onclick={() => { void process(item, 'dismiss'); }}><Trash2 size={16}/></button>
            </div>
          </article>
        {/each}
      </section>
    {/if}
  </div>
</section>
