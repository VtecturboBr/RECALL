<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { Link2, Plus, X, FileText, ListTodo, Lightbulb, ArrowUpRight } from 'lucide-svelte';
  import { linksApi } from '../lib/api';
  import { availableLinkTargets, linkedEntryKey } from '../lib/related';
  import type { ItemLink } from '../lib/types';
  import type { WorkspaceEntry } from '../lib/workspace';

  let { itemType, itemId, entries, onOpen }: {
    itemType: ItemLink['item_type'];
    itemId: string;
    entries: WorkspaceEntry[];
    onOpen: (key: string) => void;
  } = $props();
  let links = $state<ItemLink[]>([]);
  let targetKey = $state('');
  let loading = $state(true);
  let busy = $state(false);
  let error = $state('');
  let active = false;
  let generation = 0;
  let candidates = $derived(availableLinkTargets(entries, itemType + ':' + itemId, links));

  async function load() {
    const request = ++generation;
    try {
      const found = await linksApi.list(itemType,itemId);
      if (!active || request !== generation || busy) return;
      links = found;
    } catch (e) {
      if (active && request === generation) error = 'Não foi possível consultar as conexões: ' + String(e);
    } finally {
      if (active && request === generation) loading = false;
    }
  }

  onMount(() => {
    active = true;
    void load();
    const disposers: Array<() => void> = [];
    for (const name of ['links:changed','notes:changed','tasks:changed','inbox:changed']) {
      void listen(name, () => { if (active && !busy) void load(); })
        .then(unlisten => { if (active) disposers.push(unlisten); else unlisten(); });
    }
    return () => {
      active = false;
      generation++;
      for (const dispose of disposers) dispose();
    };
  });

  async function add() {
    const target = candidates.find(candidate => candidate.key === targetKey);
    if (!target || busy) return;
    const id = target.type === 'note' ? target.note.id : target.type === 'task' ? target.task.id : target.idea.id;
    busy = true;
    error = '';
    try {
      await linksApi.add(itemType,itemId,target.type,id);
      targetKey = '';
    } catch (e) {
      error = 'Não foi possível conectar: ' + String(e);
    } finally {
      busy = false;
      await load();
    }
  }

  async function remove(linkId: string) {
    if (busy) return;
    busy = true;
    error = '';
    try {
      await linksApi.remove(linkId);
    } catch (e) {
      error = 'Não foi possível remover a conexão: ' + String(e);
    } finally {
      busy = false;
      await load();
    }
  }
</script>

<div class="re-linked" aria-label="Informações relacionadas">
  <div class="re-linked-title"><Link2 size={17}/><strong>Informações conectadas</strong></div>
  <p class="re-linked-hint">Ligue notas, tarefas e ideias para encontrar o contexto no mesmo lugar.</p>
  {#if loading}
    <p class="re-linked-status">Carregando relações...</p>
  {:else if links.length === 0}
    <p class="re-linked-status">Ainda não há conexões com este item.</p>
  {:else}
    <div class="re-linked-list">
      {#each links as link (link.id)}
        <div class="re-linked-item">
          <button class="re-linked-open" title={'Abrir ' + link.title} onclick={() => onOpen(linkedEntryKey(link))}>
            {#if link.item_type === 'note'}<FileText size={15}/>{:else if link.item_type === 'task'}<ListTodo size={15}/>{:else}<Lightbulb size={15}/>{/if}
            <span><strong>{link.title}</strong><small>{link.item_type === 'note' ? 'Nota' : link.item_type === 'task' ? 'Tarefa' : 'Ideia'} · {link.preview?.slice(0, 80) || 'Sem detalhes'}</small></span>
            <ArrowUpRight size={14}/>
          </button>
          <button class="re-linked-remove" disabled={busy} title={'Desconectar ' + link.title}
            aria-label={'Desconectar ' + link.title} onclick={() => { void remove(link.id); }}><X size={15}/></button>
        </div>
      {/each}
    </div>
  {/if}
  <div class="re-linked-controls">
    <label class="re-visually-hidden" for={'re-linked-target-' + itemType + itemId}>Escolher nota, tarefa ou ideia</label>
    <select id={'re-linked-target-' + itemType + itemId} value={targetKey}
      onchange={(event) => targetKey = event.currentTarget.value} disabled={busy || candidates.length === 0}>
      <option value="">Escolha uma informação...</option>
      {#each candidates as candidate (candidate.key)}
        <option value={candidate.key}>{candidate.type === 'note' ? 'Nota' : candidate.type === 'task' ? 'Tarefa' : 'Ideia'} · {candidate.title.slice(0, 70)}</option>
      {/each}
    </select>
    <button class="re-outline" disabled={busy || !candidates.some(item => item.key === targetKey)}
      onclick={() => { void add(); }}><Plus size={14}/> Conectar</button>
  </div>
  {#if error}<p class="re-linked-error" role="status">{error}</p>{/if}
</div>
