<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { Check, Circle, Plus, Pencil, Trash2, X, CalendarClock, ChevronDown, ChevronUp } from 'lucide-svelte';
  import { tasksApi, formatDateTime, toLocalInput } from '../lib/api';
  import type { Task } from '../lib/types';

  let tasks = $state<Task[]>([]);
  let title = $state('');
  let loading = $state(true);
  let busy = $state(false);
  let error = $state('');
  let editingId = $state<string | null>(null);
  let editTitle = $state('');
  let editDetails = $state('');
  let editDue = $state('');
  let editRevision = $state(0);
  let showCompleted = $state(false);

  let pending = $derived(tasks.filter(task => task.completed_at === null));
  let finished = $derived(tasks.filter(task => task.completed_at !== null));
  let current = $derived(pending[0] ?? null);

  async function refresh() {
    try { tasks = await tasksApi.list(); error = ''; }
    catch (e) { error = 'Não foi possível carregar as tarefas: ' + String(e); }
    finally { loading = false; }
  }
  onMount(() => {
    let alive = true;
    let dispose: (() => void) | undefined;
    void refresh();
    void listen('tasks:changed', () => { if (alive) void refresh(); })
      .then(fn => { if (alive) dispose = fn; else fn(); });
    return () => { alive = false; dispose?.(); };
  });
  async function add() {
    if (!title.trim() || busy) return;
    busy = true;
    try {
      await tasksApi.create(title);
      title = '';
      await refresh();
    } catch (e) { error = 'Não foi possível criar: ' + String(e); }
    finally { busy = false; }
  }
  async function toggle(task: Task) {
    try { await tasksApi.complete(task.id, task.completed_at === null); await refresh(); }
    catch (e) { error = 'Não foi possível atualizar: ' + String(e); }
  }
  function beginEdit(task: Task) {
    editingId = task.id;
    editTitle = task.title;
    editDetails = task.details;
    editDue = toLocalInput(task.due_at);
    editRevision = task.revision;
  }
  async function saveEdit() {
    if (!editingId || !editTitle.trim() || busy) return;
    const due = editDue ? new Date(editDue).getTime() : null;
    if (due !== null && !Number.isFinite(due)) { error = 'Data inválida.'; return; }
    busy = true;
    try {
      await tasksApi.update(editingId, editTitle, editDetails, due, editRevision);
      editingId = null;
      await refresh();
    } catch (e) { error = 'Não foi possível salvar: ' + String(e); }
    finally { busy = false; }
  }
  async function remove(task: Task) {
    if (!confirm('Excluir esta tarefa?')) return;
    try {
      await tasksApi.delete(task.id);
      if (editingId === task.id) editingId = null;
      await refresh();
    } catch (e) { error = 'Não foi possível excluir: ' + String(e); }
  }
</script>

<section class="v05-page" aria-label="Tarefas e prioridades de hoje">
  <div class="v05-content">
    <header class="v05-intro">
      <p class="v05-eyebrow">UM PASSO DE CADA VEZ</p>
      <h2>O que importa agora?</h2>
      <p>Escreva o próximo passo. Você não precisa organizar o dia inteiro.</p>
    </header>

    <form class="v05-add" onsubmit={(event) => { event.preventDefault(); void add(); }}>
      <Plus size={18} aria-hidden="true"/>
      <input aria-label="Nova tarefa" maxlength="250" placeholder="O que você precisa fazer?" bind:value={title}/>
      <button type="submit" disabled={!title.trim() || busy}>Adicionar</button>
    </form>

    {#if error}<p class="v05-error" role="alert">{error}</p>{/if}
    {#if loading}<p class="v05-muted">Carregando...</p>
    {:else if !current}
      <div class="v05-empty">
        <Check size={24}/>
        <h3>Nenhuma tarefa pendente.</h3>
        <p>Quando lembrar de alguma coisa, escreva no campo acima.</p>
      </div>
    {:else}
      <section class="v05-current" aria-label="Próxima tarefa">
        <p class="v05-eyebrow">PRÓXIMO PASSO</p>
        <h3>{current.title}</h3>
        {#if current.details}<p class="v05-task-description">{current.details}</p>{/if}
        {#if current.due_at}<small><CalendarClock size={14}/> {formatDateTime(current.due_at)}</small>{/if}
        <div class="v05-card-actions">
          <button class="v05-primary" onclick={() => { void toggle(current); }}><Check size={16}/> Concluir</button>
          <button class="v05-icon-btn" aria-label="Editar tarefa atual" title="Editar" onclick={() => beginEdit(current)}><Pencil size={16}/></button>
        </div>
      </section>
    {/if}

    {#if pending.length > 1}
      <section class="v05-list-section">
        <h3>Depois <span>{pending.length - 1}</span></h3>
        {#each pending.slice(1) as task (task.id)}
          <div class="v05-task-row">
            <button class="v05-check" aria-label={'Concluir '+task.title} title="Concluir" onclick={() => { void toggle(task); }}><Circle size={19}/></button>
            <div class="v05-task-info">
              <strong>{task.title}</strong>
              {#if task.due_at}<small>{formatDateTime(task.due_at)}</small>{/if}
            </div>
            <button class="v05-icon-btn" aria-label={'Editar '+task.title} onclick={() => beginEdit(task)}><Pencil size={15}/></button>
          </div>
        {/each}
      </section>
    {/if}

    {#if editingId}
      <section class="v05-edit" aria-label="Editar tarefa">
        <div class="v05-edit-head"><strong>Editar tarefa</strong><button aria-label="Fechar edição" onclick={() => editingId = null}><X size={17}/></button></div>
        <label>Título<input maxlength="250" bind:value={editTitle}/></label>
        <label>Detalhes (opcional)<textarea rows="3" maxlength="100000" bind:value={editDetails}></textarea></label>
        <label>Lembrar até (opcional)<input type="datetime-local" bind:value={editDue}/></label>
        <div class="v05-edit-actions">
          <button class="v05-quiet" onclick={() => { const found = tasks.find(t => t.id === editingId); if (found) void remove(found); }}><Trash2 size={15}/> Excluir</button>
          <button class="v05-primary" disabled={!editTitle.trim() || busy} onclick={() => { void saveEdit(); }}><Check size={15}/> Salvar</button>
        </div>
      </section>
    {/if}

    {#if finished.length}
      <section class="v05-list-section">
        <button class="v05-collapse" onclick={() => showCompleted = !showCompleted} aria-expanded={showCompleted}>
          Concluídas ({finished.length}) {#if showCompleted}<ChevronUp size={16}/>{:else}<ChevronDown size={16}/>{/if}
        </button>
        {#if showCompleted}
          {#each finished as task (task.id)}
            <div class="v05-task-row v05-done">
              <button class="v05-check" aria-label={'Reabrir '+task.title} onclick={() => { void toggle(task); }}><Check size={17}/></button>
              <div class="v05-task-info"><strong>{task.title}</strong></div>
              <button class="v05-icon-btn" aria-label={'Excluir '+task.title} onclick={() => { void remove(task); }}><Trash2 size={15}/></button>
            </div>
          {/each}
        {/if}
      </section>
    {/if}
  </div>
</section>
