<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { Play, Pause, Square, Clock3, History, ChevronDown, ChevronUp } from 'lucide-svelte';
  import { focusApi } from '../lib/api';
  import type { FocusSession as Session } from '../lib/types';
  import { effectiveElapsedMs, formatElapsed } from '../lib/focusTime';

  let { taskId }: { taskId: string } = $props();
  let active = $state<Session | null>(null);
  let recent = $state<Session[]>([]);
  let summary = $state('');
  let busy = $state(false);
  let loading = $state(true);
  let message = $state('');
  let clock = $state(Date.now());
  let showHistory = $state(false);
  let touched = $state(false);
  let mounted = false;
  let generation = 0;
  const draftKey = (sessionId: string) => 'recall-focus-summary:' + sessionId;
  let localSessionId = '';

  let here = $derived(active?.task_id === taskId ? active : null);
  let elapsed = $derived(here
    ? effectiveElapsedMs(here.active_ms, here.last_tick_at, clock, true)
    : 0);
  let completedSessions = $derived(recent.filter(item => item.state !== 'active'));

  function preserveSummary(sessionId: string) {
    try { localStorage.setItem(draftKey(sessionId), summary); }
    catch { message = 'O rascunho não pôde ser guardado localmente.'; }
  }
  function loadSummary(sessionId: string) {
    if (localSessionId === sessionId) return;
    localSessionId = sessionId;
    touched = false;
    try {
      const cached = localStorage.getItem(draftKey(sessionId));
      summary = cached?.slice(0, 2000) ?? '';
    } catch { summary = ''; }
  }
  function discardSummary(sessionId: string) {
    try { localStorage.removeItem(draftKey(sessionId)); } catch { /* Unavailable storage. */ }
  }

  async function load() {
    const thisRequest = ++generation;
    try {
      const [running, entries] = await Promise.all([
        focusApi.active(), focusApi.history(taskId)
      ]);
      if (!mounted || thisRequest !== generation || busy) return;
      active = running;
      recent = entries;
      if (running?.task_id === taskId) loadSummary(running.id);
      else if (!running) { localSessionId = ''; summary = ''; touched = false; }
    } catch (e) {
      if (mounted && thisRequest === generation) message = 'Erro ao consultar sessões: ' + String(e);
    } finally {
      if (mounted && thisRequest === generation) loading = false;
    }
  }

  onMount(() => {
    mounted = true;
    void load();
    let dispose: (() => void) | undefined;
    void listen('focus:changed', () => { if (mounted && !busy) void load(); })
      .then(unlisten => { if (mounted) dispose = unlisten; else unlisten(); });
    // Persistence is driven by the Rust scheduler, independent of this panel.
    const timer = setInterval(() => { clock = Date.now(); }, 1000);
    return () => {
      mounted = false;
      generation++;
      clearInterval(timer);
      dispose?.();
      if (here && summary) preserveSummary(here.id);
    };
  });

  async function begin() {
    if (busy || active) return;
    busy = true;
    message = '';
    try {
      const session = await focusApi.start(taskId);
      active = session;
      loadSummary(session.id);
    } catch (e) {
      message = 'Não foi possível iniciar foco: ' + String(e);
    } finally {
      busy = false;
      void load();
    }
  }

  async function end(reason: 'paused' | 'finished') {
    const session = here;
    if (!session || busy) return;
    busy = true;
    message = '';
    try {
      await focusApi.stop(session.id, reason, summary.slice(0, 2000));
      discardSummary(session.id);
      active = null;
      localSessionId = '';
      summary = '';
      touched = false;
      message = reason === 'paused' ? 'Sessão pausada. Você pode retomar depois.' : 'Sessão finalizada e registrada.';
    } catch (e) {
      preserveSummary(session.id);
      message = 'Não foi possível finalizar. Seu texto continua aqui: ' + String(e);
    } finally {
      busy = false;
      await load();
    }
  }
</script>

<section class="re-focus-session" aria-label="Sessão de foco">
  <div class="re-focus-session-top">
    <div class="re-focus-session-heading"><Clock3 size={16}/><strong>Modo Foco 2.0</strong></div>
    {#if here}<span class="re-focus-session-live"><span class="re-focus-pulse"></span> Em andamento</span>{/if}
  </div>
  {#if loading}
    <p class="re-focus-hint">Consultando seu progresso...</p>
  {:else if active && !here}
    <p class="re-focus-hint">Existe uma sessão ativa em outra tarefa. Volte a ela e pause antes de iniciar uma nova.</p>
  {:else if here}
    <div class="re-focus-clock" aria-label={'Tempo em foco: ' + formatElapsed(elapsed)}>{formatElapsed(elapsed)}</div>
    <label class="re-focus-note-label" for={'re-focus-summary-' + taskId}>O que você conseguiu fazer?</label>
    <textarea id={'re-focus-summary-' + taskId} maxlength="2000" rows="2"
      value={summary} placeholder="Anote seu progresso antes de pausar..."
      oninput={(event) => {
        summary = event.currentTarget.value;
        touched = true;
        preserveSummary(here.id);
      }}></textarea>
    <div class="re-focus-session-actions">
      <button class="re-outline" disabled={busy} onclick={() => { void end('paused'); }}><Pause size={14}/> Pausar</button>
      <button class="re-outline" disabled={busy} onclick={() => { void end('finished'); }}><Square size={14}/> Finalizar sessão</button>
    </div>
  {:else}
    <p class="re-focus-hint">Trabalhe uma coisa de cada vez. O tempo e seu progresso ficam registrados neste computador.</p>
    <button class="re-outline" disabled={busy} onclick={() => { void begin(); }}>
      <Play size={14}/> Iniciar sessão de foco
    </button>
  {/if}
  {#if message}<p class="re-focus-message" role="status">{message}</p>{/if}
  {#if completedSessions.length}
    <button class="re-focus-history-toggle" aria-expanded={showHistory} onclick={() => showHistory = !showHistory}>
      <History size={13}/> Últimas sessões
      {#if showHistory}<ChevronUp size={13}/>{:else}<ChevronDown size={13}/>{/if}
    </button>
    {#if showHistory}
      <div class="re-focus-history">
        {#each completedSessions.slice(0, 5) as item (item.id)}
          <div class="re-focus-history-item">
            <span>{item.state === 'paused' ? 'Pausada' : item.state === 'finished' ? 'Finalizada' : 'Interrompida'}
              · {formatElapsed(item.active_ms)}</span>
            <small>{new Intl.DateTimeFormat('pt-BR', {day:'2-digit',month:'short',hour:'2-digit',minute:'2-digit'}).format(item.ended_at ?? item.started_at)}</small>
            {#if item.summary}<p>{item.summary}</p>{/if}
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</section>
