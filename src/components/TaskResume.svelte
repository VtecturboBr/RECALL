<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { Check, ChevronDown, ChevronUp, RotateCcw, ShieldAlert } from 'lucide-svelte';
  import { tasksApi, inboxApi } from '../lib/api';

  let { taskId, taskTitle }: { taskId: string; taskTitle: string } = $props();
  const CACHE_PREFIX = 'recall-resume-draft:';
  let expanded = $state(false);
  let loading = $state(true);
  let lastContext = $state('');
  let nextStep = $state('');
  let revision = $state(0);
  let dirty = $state(false);
  let saving = $state(false);
  let conflict = $state(false);
  let message = $state('');
  let active = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let savingPromise: Promise<boolean> | null = null;

  const cacheKey = () => CACHE_PREFIX + taskId;

  function preserveDraft() {
    try {
      localStorage.setItem(cacheKey(), JSON.stringify({
        lastContext, nextStep, revision, savedAt: Date.now()
      }));
    } catch { message = 'Não foi possível guardar o rascunho local. Use Salvar agora.'; }
  }

  function discardDraft() {
    try { localStorage.removeItem(cacheKey()); } catch { /* Storage may be unavailable. */ }
  }

  async function load() {
    try {
      const remote = await tasksApi.getResume(taskId);
      if (!active) return;
      lastContext = remote?.last_context ?? '';
      nextStep = remote?.next_step ?? '';
      revision = remote?.revision ?? 0;
      try {
        const cached = localStorage.getItem(cacheKey());
        if (cached) {
          const parsed: unknown = JSON.parse(cached);
          if (parsed && typeof parsed === 'object' && 'lastContext' in parsed
              && 'nextStep' in parsed && 'revision' in parsed
              && typeof parsed.lastContext === 'string'
              && typeof parsed.nextStep === 'string'
              && typeof parsed.revision === 'number'
              && parsed.lastContext.length <= 15000
              && parsed.nextStep.length <= 2000) {
            if (parsed.lastContext !== lastContext || parsed.nextStep !== nextStep) {
              lastContext = parsed.lastContext;
              nextStep = parsed.nextStep;
              dirty = true;
              revision = parsed.revision;
              expanded = true;
              if (remote && remote.revision !== parsed.revision) {
                conflict = true;
                message = 'Outra janela atualizou esta retomada. Seu rascunho foi preservado.';
              } else {
                message = 'Rascunho recuperado. Você pode continuar.';
                scheduleSave();
              }
            } else discardDraft();
          }
        }
      } catch { /* Keep an unreadable cache for manual recovery. */ }
    } catch (e) {
      if (active) message = 'Não foi possível carregar o ponto de retomada: ' + String(e);
    } finally {
      if (active) loading = false;
    }
  }

  onMount(() => {
    active = true;
    void load();
    let dispose: (() => void) | undefined;
    void listen<string>('tasks:resume-changed', (event) => {
      if (!active || event.payload !== taskId) return;
      // Never replace local work when another window emits a change.
      if (dirty || saving) {
        if (dirty && !saving) {
          conflict = true;
          message = 'Esta retomada mudou em outra janela. Seu texto continua aqui.';
        }
      } else void load();
    }).then(unlisten => {
      if (active) dispose = unlisten;
      else unlisten();
    });
    return () => {
      active = false;
      if (timer) clearTimeout(timer);
      dispose?.();
      if (dirty) preserveDraft();
    };
  });

  function scheduleSave() {
    if (timer) clearTimeout(timer);
    if (!conflict) timer = setTimeout(() => { void save(); }, 900);
  }

  function edit(field: 'context' | 'next', text: string) {
    if (field === 'context') lastContext = text;
    else nextStep = text;
    dirty = true;
    preserveDraft(); // Synchronous cache before the debounce or process shutdown.
    scheduleSave();
  }

  async function save(): Promise<boolean> {
    if (timer) { clearTimeout(timer); timer = undefined; }
    if (savingPromise && !(await savingPromise)) return false;
    if (!dirty) return true;
    if (conflict) return false;
    const context = lastContext;
    const next = nextStep;
    const expectedRevision = revision;
    dirty = false;
    saving = true;
    message = '';
    const operation: Promise<boolean> = tasksApi.saveResume(taskId, context, next, expectedRevision)
      .then(saved => {
        revision = saved.revision;
        if (!dirty) discardDraft();
        else preserveDraft();
        message = dirty ? 'Salvando alterações recentes...' : 'Ponto de retomada salvo.';
        return true;
      })
      .catch(e => {
        dirty = true;
        conflict = String(e).includes('CONFLITO_REVISAO');
        preserveDraft();
        message = conflict
          ? 'Conflito de revisão: outra janela alterou este ponto. Salve uma cópia para não perder nada.'
          : 'Falha ao salvar. Seu rascunho continua aqui: ' + String(e);
        return false;
      });
    savingPromise = operation;
    const ok = await operation;
    if (savingPromise === operation) savingPromise = null;
    saving = false;
    if (ok && dirty) return save();
    return ok;
  }

  async function saveCopy() {
    if (saving) return;
    try {
      await inboxApi.capture('Retomada da tarefa: ' + taskTitle +
        '\n\nOnde parei:\n' + lastContext + '\n\nPróximo passo:\n' + nextStep);
      message = 'Cópia salva como ideia no Arquivo. O item original não foi substituído.';
    } catch (e) {
      preserveDraft();
      message = 'Não foi possível criar a cópia: ' + String(e);
    }
  }
</script>

<section class="re-resume" aria-label="Onde parei">
  <button class="re-resume-toggle" aria-expanded={expanded} onclick={() => expanded = !expanded}>
    <RotateCcw size={15}/>
    <span>Onde parei?</span>
    {#if !expanded && nextStep}<small>Próximo: {nextStep.slice(0, 80)}</small>{/if}
    {#if expanded}<ChevronUp size={15}/>{:else}<ChevronDown size={15}/>{/if}
  </button>
  {#if expanded}
    <div class="re-resume-panel">
      <p>Registre só o suficiente para continuar depois.</p>
      {#if loading}
        <span class="re-resume-status">Carregando...</span>
      {:else}
        <label for={'resume-context-' + taskId}>O que eu estava fazendo?</label>
        <textarea id={'resume-context-' + taskId} rows="2" maxlength="15000"
          value={lastContext} placeholder="Ex.: terminei a estrutura, faltou revisar a parte final"
          oninput={(e) => edit('context', e.currentTarget.value)}
          onblur={() => { void save(); }}></textarea>
        <label for={'resume-next-' + taskId}>Qual é o próximo passo?</label>
        <input id={'resume-next-' + taskId} maxlength="2000"
          value={nextStep} placeholder="Ex.: revisar o último slide"
          oninput={(e) => edit('next', e.currentTarget.value)}
          onblur={() => { void save(); }}/>
        <div class="re-resume-actions">
          <button class="re-outline" disabled={!dirty || saving || conflict} onclick={() => { void save(); }}>
            <Check size={14}/>{saving ? 'Salvando...' : 'Salvar agora'}
          </button>
          {#if conflict}
            <button class="re-outline" disabled={saving} onclick={() => { void saveCopy(); }}>
              <ShieldAlert size={14}/> Salvar cópia como ideia
            </button>
          {/if}
          <span class="re-resume-status" aria-live="polite">{message || (dirty ? 'Rascunho guardado localmente' : 'Pronto para retomar')}</span>
        </div>
      {/if}
    </div>
  {/if}
</section>
