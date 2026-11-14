<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { Clock3, Pause, Square, ArrowUpRight } from 'lucide-svelte';
  import { focusApi } from '../lib/api';
  import type { FocusSession } from '../lib/types';

  let { onOpen }: { onOpen: (taskId: string) => void } = $props();
  let active = $state<FocusSession | null>(null);
  let busy = $state(false);
  let error = $state('');
  let mounted = false;
  let generation = 0;

  async function refresh() {
    const req = ++generation;
    try {
      const current = await focusApi.active();
      if (mounted && req === generation && !busy) active = current;
    } catch (e) {
      if (mounted && req === generation) error = 'Não foi possível consultar o foco: ' + String(e);
    }
  }

  async function finish(reason: 'paused' | 'finished') {
    const session = active;
    if (!session || busy) return;
    busy = true;
    error = '';
    const key = 'recall-focus-summary:' + session.id;
    let summary = '';
    try { summary = (localStorage.getItem(key) ?? '').slice(0,2000); }
    catch { /* The session remains resumable even if cache is disabled. */ }
    try {
      await focusApi.stop(session.id, reason, summary);
      try { localStorage.removeItem(key); } catch { /* Best effort */ }
      active = null;
    } catch (e) {
      error = 'Não foi possível pausar/finalizar o foco. Seu resumo permanece guardado: ' + String(e);
    } finally {
      busy = false;
      void refresh();
    }
  }

  onMount(() => {
    mounted = true;
    void refresh();
    const disposers: Array<() => void> = [];
    void listen('focus:changed', () => { if (mounted && !busy) void refresh(); })
      .then(dispose => { if (mounted) disposers.push(dispose); else dispose(); });
    const poll = setInterval(() => { if (mounted && !busy) void refresh(); }, 25_000);
    return () => {
      mounted = false;
      generation++;
      clearInterval(poll);
      for (const dispose of disposers) dispose();
    };
  });
</script>

{#if active}
  <div class="re-global-focus" role="region" aria-label="Sessão de foco ativa">
    <div class="re-global-focus-label"><Clock3 size={15}/><strong>Foco em andamento</strong><small>A sessão continua mesmo fora da tela Agora.</small></div>
    <div class="re-global-focus-actions">
      <button class="re-outline" disabled={busy} onclick={() => onOpen(active!.task_id)}>Abrir tarefa <ArrowUpRight size={13}/></button>
      <button class="re-outline" disabled={busy} onclick={() => { void finish('paused'); }}><Pause size={13}/> Pausar</button>
      <button class="re-outline" disabled={busy} onclick={() => { void finish('finished'); }}><Square size={13}/> Finalizar</button>
    </div>
    {#if error}<span class="re-global-focus-error" role="status">{error}</span>{/if}
  </div>
{/if}
