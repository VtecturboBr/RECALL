<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { ClipboardList, ChevronDown, ChevronUp, ArrowUpRight, EyeOff, RotateCcw, CalendarCheck2 } from 'lucide-svelte';
  import { reviewApi } from '../lib/api';
  import { reviewDayKey, reviewItemKey, visibleReviewItems, reviewLabels } from '../lib/dailyReview';
  import type { DailyReviewItem } from '../lib/types';

  let { onOpen }: { onOpen: (key: string) => void } = $props();
  let expanded = $state(false);
  let loading = $state(false);
  let rows = $state<DailyReviewItem[]>([]);
  let hidden = $state<Set<string>>(new Set());
  let day = $state(reviewDayKey(new Date()));
  let error = $state('');
  let loaded = $state(false);
  let alive = false;
  let generation = 0;

  let visible = $derived(visibleReviewItems(rows, hidden));
  const storageKey = (calendarDay: string) => 'recall-review-hidden-v1:' + calendarDay;

  function readHidden(calendarDay: string): Set<string> {
    try {
      const stored = JSON.parse(localStorage.getItem(storageKey(calendarDay)) || '[]') as unknown;
      if (Array.isArray(stored)) return new Set(
        stored.filter((item): item is string => typeof item === 'string' && item.length < 200).slice(0, 100)
      );
    } catch { /* Storage is optional: review still works. */ }
    return new Set();
  }

  function storeHidden() {
    try { localStorage.setItem(storageKey(day), JSON.stringify([...hidden].slice(0, 100))); }
    catch { error = 'Não foi possível guardar os itens ocultados neste dispositivo.'; }
  }

  function syncCalendarDay() {
    const today = reviewDayKey(new Date());
    if (today !== day) {
      day = today;
      hidden = readHidden(today);
      loaded = false;
    }
  }

  async function refresh() {
    if (!expanded) return;
    syncCalendarDay();
    const request = ++generation;
    loading = true;
    error = '';
    try {
      const found = await reviewApi.list();
      if (alive && request === generation && expanded) {
        rows = found;
        loaded = true;
      }
    } catch (e) {
      if (alive && request === generation) error = 'Não foi possível consultar a revisão: ' + String(e);
    } finally {
      if (alive && request === generation) loading = false;
    }
  }

  function toggle() {
    expanded = !expanded;
    if (expanded) void refresh();
    else generation++;
  }

  function hideForToday(item: DailyReviewItem) {
    hidden = new Set([...hidden, reviewItemKey(item)]);
    storeHidden();
  }

  function showHidden() {
    hidden = new Set();
    storeHidden();
  }

  onMount(() => {
    alive = true;
    hidden = readHidden(day);
    const disposers: Array<() => void> = [];
    for (const name of ['tasks:changed','tasks:resume-changed','notes:changed','reminders:changed','focus:changed']) {
      void listen(name, () => { if (alive && expanded && !loading) void refresh(); })
        .then(unlisten => { if (alive) disposers.push(unlisten); else unlisten(); });
    }
    const midnightCheck = setInterval(() => {
      if (!alive || !expanded) return;
      const previous = day;
      syncCalendarDay();
      if (previous !== day) void refresh();
    }, 60_000);
    return () => {
      alive = false;
      generation++;
      clearInterval(midnightCheck);
      for (const dispose of disposers) dispose();
    };
  });
</script>

<section class="re-section re-daily-review" aria-label="Revisão diária">
  <div class="re-section-heading re-review-heading">
    <div>
      <span class="re-eyebrow">QUANDO VOCÊ QUISER</span>
      <h3><CalendarCheck2 size={18}/> Revisão diária</h3>
    </div>
    <button class="re-outline re-review-toggle" aria-expanded={expanded}
      onclick={toggle}>
      {#if expanded}Fechar <ChevronUp size={14}/>{:else}Revisar agora <ChevronDown size={14}/>{/if}
    </button>
  </div>
  <p class="re-review-intro">Um resumo dos lembretes, tarefas interrompidas e próximos passos que podem precisar de atenção. Nada é alterado automaticamente.</p>
  {#if expanded}
    <div class="re-review-body">
      {#if loading && !loaded}
        <p class="re-review-empty" role="status">Consultando suas informações...</p>
      {:else if error && !loaded}
        <p class="re-review-error" role="status">{error}</p>
        <button class="re-outline" disabled={loading} onclick={() => { void refresh(); }}><RotateCcw size={13}/> Tentar novamente</button>
      {:else}
        <div class="re-review-meta">
          <span>{visible.length ? visible.length + (visible.length === 1 ? ' destaque para conferir' : ' destaques para conferir') : 'Sem destaques para conferir agora'}</span>
          <button class="re-review-refresh" disabled={loading} onclick={() => { void refresh(); }}><RotateCcw size={13}/> Atualizar</button>
        </div>
        {#if error}<p class="re-review-error" role="status">{error}</p>{/if}
        {#if visible.length}
          <div class="re-review-list">
            {#each visible as item (reviewItemKey(item))}
              <div class="re-review-item">
                <div class="re-review-copy">
                  <span class="re-review-kind">{reviewLabels[item.kind]}</span>
                  <strong>{item.title}</strong>
                  <p>{item.description}</p>
                </div>
                <div class="re-review-actions">
                  <button class="re-outline" onclick={() => onOpen(item.target_type + ':' + item.target_id)}>
                    Abrir <ArrowUpRight size={14}/>
                  </button>
                  <button class="re-review-hide" title={'Ocultar ' + item.title + ' somente hoje'}
                    aria-label={'Ocultar ' + item.title + ' somente hoje'}
                    onclick={() => hideForToday(item)}><EyeOff size={14}/></button>
                </div>
              </div>
            {/each}
          </div>
        {:else}
          <div class="re-review-empty"><ClipboardList size={18}/><span>Sem nada urgente nesta revisão. Você pode voltar às suas tarefas normalmente.</span></div>
        {/if}
        {#if hidden.size > 0}
          <button class="re-review-show-hidden" onclick={showHidden}>Mostrar os {hidden.size} itens ocultados hoje</button>
        {/if}
        <p class="re-review-foot">Até 12 destaques por consulta. Ocultar vale apenas neste dispositivo e neste dia; seus dados e lembretes continuam intactos.</p>
      {/if}
    </div>
  {/if}
</section>
