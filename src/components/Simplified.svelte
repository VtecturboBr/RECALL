<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import {
    BrainCircuit, CircleDot, Archive, Search, Plus, Check, Circle, Bell,
    FileText, Pencil, CalendarClock, MonitorUp, Presentation, ChevronLeft,
    ChevronRight, X, Trash2, ArrowRight, Menu, RotateCcw, Target
  } from 'lucide-svelte';
  import { notesApi, tasksApi, inboxApi, formatDateTime, toLocalInput } from '../lib/api';
  import { openQuickCapture, openSticky } from '../lib/windows';
  import { buildWorkspace, nextTask, toSlides, searchWorkspace, taskPeriod, tasksByPeriod } from '../lib/workspace';
  import { parseTaskCapture } from '../lib/smartCapture';
  import type { Note, Task, InboxItem } from '../lib/types';

  type View = 'now' | 'archive';
  let view = $state<View>('now');
  let notes = $state<Note[]>([]);
  let tasks = $state<Task[]>([]);
  let ideas = $state<InboxItem[]>([]);
  let selectedKey = $state<string | null>(null);
  let search = $state('');
  let loading = $state(true);
  let error = $state('');
  let busy = $state(false);
  let sidebarVisible = $state(true);
  let focusedTaskId = $state<string | null>(null);
  let captureKind = $state<'idea' | 'task'>('idea');
  let clock = $state(new Date());
  let archiveFilter = $state<'all' | 'note' | 'task' | 'idea'>('all');
  const FOCUS_KEY = 'recall-selected-focus-task-v1';

  let newText = $state('');
  let draftNoteId: string | null = null;
  let noteTitle = $state('');
  let noteBody = $state('');
  let noteDirty = $state(false);
  let noteSaving = $state(false);
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  let saveInFlight: Promise<boolean> | null = null;

  let ideaText = $state('');
  let ideaDirty = $state(false);
  let ideaSaving = $state(false);
  let ideaSaveTimer: ReturnType<typeof setTimeout> | undefined;
  let ideaSaveInFlight: Promise<boolean> | null = null;

  let taskTitle = $state('');
  let taskDetails = $state('');
  let taskDue = $state('');
  let taskDirty = $state(false);
  let taskSaving = $state(false);
  let taskSaveTimer: ReturnType<typeof setTimeout> | undefined;
  let taskSaveInFlight: Promise<boolean> | null = null;
  let presenting = $state(false);
  let slideIndex = $state(0);
  let entries = $derived(buildWorkspace(notes, tasks, ideas));
  let selected = $derived(entries.find(item => item.key === selectedKey) ?? null);
  let matches = $derived(searchWorkspace(entries, search, archiveFilter));
  let todayTasks = $derived(tasksByPeriod(tasks, 'today', clock));
  let laterTasks = $derived(tasksByPeriod(tasks, 'later', clock));
  let somedayTasks = $derived(tasksByPeriod(tasks, 'someday', clock));
  let smartDraft = $derived(parseTaskCapture(newText, clock));
  let currentTask = $derived(
    todayTasks.find(task => task.id === focusedTaskId)
    ?? nextTask(todayTasks)
  );
  let otherTasks = $derived(todayTasks.filter(t => t.id !== currentTask?.id)
    .sort((a, b) => (a.due_at ?? Infinity) - (b.due_at ?? Infinity) || a.created_at - b.created_at));
  let dueNotes = $derived(notes.filter(n => n.deleted_at === null && n.remind_at !== null &&
    n.notified_at === null).sort((a,b) => (a.remind_at ?? Infinity) - (b.remind_at ?? Infinity)));
  let slides = $derived(toSlides(noteBody));

  async function refresh(): Promise<void> {
    try {
      const [n, t, i] = await Promise.all([notesApi.list(), tasksApi.list(), inboxApi.list()]);
      notes = n;
      tasks = t;
      ideas = i;
      error = '';
    } catch (e) {
      error = 'Não foi possível carregar seus dados: ' + String(e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    try { focusedTaskId = localStorage.getItem(FOCUS_KEY); } catch { /* No local storage available. */ }
    let alive = true;
    const dayClock = setInterval(() => { clock = new Date(); }, 60_000);
    const unlisteners: Array<() => void> = [];
    void refresh();
    for (const eventName of ['notes:changed', 'tasks:changed', 'inbox:changed']) {
      void listen(eventName, () => { if (alive) void refresh(); })
        .then(dispose => { if (alive) unlisteners.push(dispose); else dispose(); });
    }
    const onKey = (event: KeyboardEvent) => {
      if (event.ctrlKey && event.key.toLowerCase() === 'k') {
        event.preventDefault();
        void switchView('archive').then(() => {
          if (view === 'archive') requestAnimationFrame(() => document.querySelector<HTMLInputElement>('.rx-search input')?.focus());
        });
      }
      if (event.ctrlKey && event.key.toLowerCase() === 'n') {
        event.preventDefault(); void captureWindow();
      }
      if (event.key === 'Escape') presenting = false;
      if (presenting && event.key === 'ArrowRight') slideIndex = Math.min(slideIndex + 1, slides.length - 1);
      if (presenting && event.key === 'ArrowLeft') slideIndex = Math.max(slideIndex - 1, 0);
    };
    window.addEventListener('keydown', onKey);
    return () => {
      alive = false;
      clearInterval(dayClock);
      unlisteners.forEach(fn => fn());
      window.removeEventListener('keydown', onKey);
      if (saveTimer) clearTimeout(saveTimer);
      if (taskSaveTimer) clearTimeout(taskSaveTimer);
      if (ideaSaveTimer) clearTimeout(ideaSaveTimer);
      if (ideaDirty && selected?.type === 'idea') void saveIdea(selected.idea);
      if (noteDirty) void flushNote();
      if (taskDirty && selected?.type === 'task') void saveTask(selected.task);
    };
  });

  async function captureWindow() {
    try { await openQuickCapture(); }
    catch (e) { error = 'Falha ao abrir captura rápida: ' + String(e); }
  }

  async function captureInline() {
    if (!newText.trim() || busy) return;
    busy = true;
    try {
      const text = newText.trim();
      if (captureKind === 'task') {
        const draft = parseTaskCapture(text, new Date());
        if (!draft.title.trim() || draft.title.length > 250) throw new Error('Use um título de até 250 caracteres.');
        const task = await tasksApi.createPlanned(draft.title, draft.details, draft.bucket, draft.plannedFor, draft.dueAt);
        if (!currentTask && draft.bucket === 'today') chooseFocus(task.id);
      } else {
        await inboxApi.capture(text);
      }
      newText = '';
      await refresh();
    } catch (e) {
      error = 'A anotação não foi salva: ' + String(e);
    } finally { busy = false; }
  }

  function onNoteEdit(field: 'title' | 'body', text: string) {
    if (field === 'title') noteTitle = text;
    else noteBody = text;
    noteDirty = true;
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => { void flushNote(); }, 650);
  }

  async function flushNote(): Promise<boolean> {
    if (saveTimer) { clearTimeout(saveTimer); saveTimer = undefined; }
    if (saveInFlight) {
      const completed = await saveInFlight;
      if (!completed) return false;
    }
    if (!noteDirty || !draftNoteId) return true;
    const id = draftNoteId;
    const title = noteTitle;
    const body = noteBody;
    noteDirty = false;
    noteSaving = true;
    const operation: Promise<boolean> = notesApi.save(id, title, body)
      .then(saved => {
        notes = notes.map(n => n.id === saved.id ? saved : n);
        return true;
      })
      .catch(e => {
        noteDirty = true;
        error = 'Não foi possível salvar sua nota. O texto continua aqui: ' + String(e);
        return false;
      });
    saveInFlight = operation;
    const ok = await operation;
    if (saveInFlight === operation) saveInFlight = null;
    noteSaving = false;
    if (!ok) return false;
    if (noteDirty) return flushNote();
    return true;
  }

  async function openEntry(key: string) {
    if (key !== selectedKey && !(await flushDraft())) return;
    selectedKey = key;
    presenting = false;
    slideIndex = 0;
    const entry = entries.find(item => item.key === key);
    if (entry?.type === 'note') {
      draftNoteId = entry.note.id;
      noteTitle = entry.note.title;
      noteBody = entry.note.body;
      noteDirty = false;
    } else {
      draftNoteId = null;
      noteDirty = false;
    }
    if (entry?.type === 'idea') {
      ideaText = entry.idea.text;
      ideaDirty = false;
    }
    if (entry?.type === 'task') {
      taskTitle = entry.task.title;
      taskDetails = entry.task.details;
      taskDue = toLocalInput(entry.task.due_at);
      taskDirty = false;
    }
  }

  async function switchView(next: View) {
    if (next === view) return;
    if (!(await flushDraft())) return;
    presenting = false;
    view = next;
  }

  async function archiveEntry(key: string) {
    if (!(await flushDraft())) return;
    view = 'archive';
    await openEntry(key);
  }

  async function focusAndGo(id: string) {
    if (!(await flushDraft())) return;
    const task = tasks.find(t => t.id === id);
    if (!task) return;
    try {
      // Focusing a future task explicitly brings it to Today.
      if (taskPeriod(task, clock) !== 'today') {
        await tasksApi.plan(id, 'today', null);
        await refresh();
      }
      chooseFocus(id);
      await switchView('now');
    } catch (e) { error = 'Não foi possível mudar o foco: ' + String(e); }
  }

  async function changeTaskPeriod(task: Task, bucket: Task['bucket']) {
    if (busy || !(await flushDraft())) return;
    busy = true;
    try {
      await tasksApi.plan(task.id, bucket, bucket === 'later' && task.bucket === 'later' ? task.planned_for : null);
      if (bucket !== 'today' && focusedTaskId === task.id) clearFocus();
      await refresh();
    } catch (e) { error = 'Não foi possível organizar a tarefa: ' + String(e); }
    finally { busy = false; }
  }

  async function setPlannedDate(task: Task, value: string) {
    if (busy || !(await flushDraft())) return;
    const at = value ? new Date(value + 'T00:00:00').getTime() : null;
    if (at !== null && !Number.isFinite(at)) { error = 'Data de planejamento inválida.'; return; }
    busy = true;
    try {
      await tasksApi.plan(task.id, 'later', at);
      if (focusedTaskId === task.id) clearFocus();
      await refresh();
    } catch (e) { error = 'Não foi possível agendar esta tarefa: ' + String(e); }
    finally { busy = false; }
  }

  function chooseFocus(id: string) {
    focusedTaskId = id;
    try { localStorage.setItem(FOCUS_KEY, id); } catch { /* Focus is still available this session. */ }
  }
  function clearFocus() {
    focusedTaskId = null;
    try { localStorage.removeItem(FOCUS_KEY); } catch { /* Local storage unavailable. */ }
  }

  function onTaskEdit(field: 'title' | 'details' | 'due', value: string) {
    if (field === 'title') taskTitle = value;
    else if (field === 'details') taskDetails = value;
    else taskDue = value;
    taskDirty = true;
    if (taskSaveTimer) clearTimeout(taskSaveTimer);
    const id = selected?.type === 'task' ? selected.task.id : null;
    taskSaveTimer = setTimeout(() => {
      if (selected?.type === 'task' && selected.task.id === id) void saveTask(selected.task);
    }, 900);
  }

  function onIdeaEdit(value: string) {
    ideaText = value;
    ideaDirty = true;
    if (ideaSaveTimer) clearTimeout(ideaSaveTimer);
    const id = selected?.type === 'idea' ? selected.idea.id : null;
    ideaSaveTimer = setTimeout(() => {
      if (selected?.type === 'idea' && selected.idea.id === id) void saveIdea(selected.idea);
    }, 900);
  }

  async function saveIdea(idea: InboxItem): Promise<boolean> {
    if (ideaSaveTimer) { clearTimeout(ideaSaveTimer); ideaSaveTimer = undefined; }
    if (ideaSaveInFlight && !(await ideaSaveInFlight)) return false;
    if (!ideaDirty) return true;
    if (!ideaText.trim()) {
      error = 'A ideia não pode ficar vazia.';
      return false;
    }
    const id = idea.id;
    const value = ideaText;
    ideaDirty = false;
    ideaSaving = true;
    const write: Promise<boolean> = inboxApi.update(id, value)
      .then(updated => {
        ideas = ideas.map(i => i.id === updated.id ? updated : i);
        return true;
      })
      .catch(e => {
        ideaDirty = true;
        error = 'Não foi possível salvar a ideia. O texto continua aqui: ' + String(e);
        return false;
      });
    ideaSaveInFlight = write;
    const ok = await write;
    if (ideaSaveInFlight === write) ideaSaveInFlight = null;
    ideaSaving = false;
    if (!ok) return false;
    if (ideaDirty) return saveIdea(idea);
    return true;
  }

  async function flushDraft(): Promise<boolean> {
    if (selected?.type === 'task') return saveTask(selected.task);
    if (selected?.type === 'idea') return saveIdea(selected.idea);
    return flushNote();
  }

  async function completeTask(task: Task) {
    if (busy) return;
    if (selected?.type === 'task' && selected.task.id === task.id && !(await flushDraft())) return;
    busy = true;
    try {
      await tasksApi.complete(task.id, task.completed_at === null);
      if (task.completed_at === null && task.id === focusedTaskId) clearFocus();
      await refresh();
    } catch (e) { error = 'Falha ao alterar tarefa: ' + String(e); }
    finally { busy = false; }
  }

  async function saveTask(task: Task): Promise<boolean> {
    if (taskSaveTimer) { clearTimeout(taskSaveTimer); taskSaveTimer = undefined; }
    if (taskSaveInFlight) {
      if (!(await taskSaveInFlight)) return false;
    }
    if (!taskDirty) return true;
    if (!taskTitle.trim()) {
      error = 'O título da tarefa não pode ficar vazio.';
      return false;
    }
    const dueAt = taskDue ? new Date(taskDue).getTime() : null;
    if (dueAt !== null && !Number.isFinite(dueAt)) {
      error = 'Data inválida.';
      return false;
    }
    const id = task.id;
    const title = taskTitle;
    const details = taskDetails;
    taskDirty = false;
    taskSaving = true;
    const write: Promise<boolean> = tasksApi.update(id, title, details, dueAt)
      .then(updated => {
        tasks = tasks.map(t => t.id === updated.id ? updated : t);
        return true;
      })
      .catch(e => {
        taskDirty = true;
        error = 'Não foi possível salvar a tarefa. O texto continua aqui: ' + String(e);
        return false;
      });
    taskSaveInFlight = write;
    const saved = await write;
    if (taskSaveInFlight === write) taskSaveInFlight = null;
    taskSaving = false;
    if (!saved) return false;
    if (taskDirty) return saveTask(task);
    return true;
  }

  async function updateReminder(note: Note, value: string) {
    if (!(await flushDraft())) return;
    const at = value ? new Date(value).getTime() : null;
    if (at !== null && !Number.isFinite(at)) { error = 'Data inválida.'; return; }
    try { await notesApi.remind(note.id, at); await refresh(); }
    catch (e) { error = 'Falha ao salvar lembrete: ' + String(e); }
  }

  async function floatNote(note: Note) {
    if (!(await flushNote())) return;
    try { await openSticky(note.id); }
    catch (e) { error = 'Falha ao abrir cartão flutuante: ' + String(e); }
  }

  async function convertIdea(item: InboxItem, target: 'note' | 'task') {
    if (busy || !(await flushDraft())) return;
    busy = true;
    try {
      if (target === 'note') {
        const created = await inboxApi.toNote(item.id);
        await refresh();
        await openEntry('note:' + created.id);
      } else {
        const created = await inboxApi.toTask(item.id);
        await refresh();
        await openEntry('task:' + created.id);
      }
    } catch (e) { error = 'Falha ao organizar a ideia: ' + String(e); }
    finally { busy = false; }
  }

  async function deleteSelected() {
    if (!selected || busy || !confirm('Excluir este item?')) return;
    if (!(await flushDraft())) return;
    if (selected.type === 'task' && selected.task.id === focusedTaskId) clearFocus();
    busy = true;
    try {
      if (selected.type === 'note') await notesApi.delete(selected.note.id);
      else if (selected.type === 'task') await tasksApi.delete(selected.task.id);
      else await inboxApi.dismiss(selected.idea.id);
      selectedKey = null;
      draftNoteId = null;
      await refresh();
    } catch (e) { error = 'Falha ao excluir: ' + String(e); }
    finally { busy = false; }
  }
</script>

<svelte:head><title>RECALL — Memória externa</title></svelte:head>

<div class="rx-app" class:rx-collapsed={!sidebarVisible}>
  <aside class="rx-sidebar" aria-label="Navegação principal">
    <div class="rx-brand"><span class="rx-logo"><BrainCircuit size={21}/></span><strong>RECALL</strong></div>
    <nav class="rx-nav">
      <button class:rx-current={view === 'now'} onclick={() => { void switchView('now'); }}><CircleDot size={18}/> Agora</button>
      <button class:rx-current={view === 'archive'} onclick={() => { void switchView('archive'); }}><Archive size={18}/> Arquivo</button>
    </nav>
    <div class="rx-sidebar-foot"><span class="rx-local-dot"></span> Dados locais <span class="rx-build">α3</span></div>
  </aside>

  <main class="rx-main">
    <header class="rx-toolbar">
      <button class="rx-icon-button rx-menu" title="Mostrar ou ocultar menu" aria-label="Mostrar ou ocultar menu" onclick={() => sidebarVisible = !sidebarVisible}><Menu size={19}/></button>
      <span class="rx-page-name">{view === 'now' ? 'Agora' : 'Arquivo'}</span>
      <span class="rx-spacer"></span>
      <button class="rx-top-search" onclick={() => { void switchView('archive').then(() => { if (view === 'archive') document.querySelector<HTMLInputElement>('.rx-search input')?.focus(); }); }} title="Pesquisar (Ctrl+K)" aria-label="Pesquisar"><Search size={18}/></button>
      <button class="rx-add-button" onclick={() => { void captureWindow(); }}><Plus size={17}/> Capturar</button>
    </header>

    {#if error}
      <div class="rx-error" role="alert">{error}<button aria-label="Fechar erro" onclick={() => error = ''}><X size={16}/></button></div>
    {/if}

    {#if loading}
      <div class="rx-loading">Abrindo suas informações...</div>
    {:else if view === 'now'}
      <section class="rx-now" aria-label="Sua atenção agora">
        <div class="rx-intro">
          <p class="rx-overline">UMA COISA DE CADA VEZ</p>
          <h1>O que importa agora?</h1>
          <p>Sem precisar organizar o dia inteiro.</p>
        </div>

        {#if currentTask}
          <div class="rx-focus-card">
            <p class="rx-overline">PRÓXIMO PASSO</p>
            <h2>{currentTask.title}</h2>
            {#if currentTask.details}<p class="rx-description">{currentTask.details}</p>{/if}
            {#if currentTask.due_at}<p class="rx-date"><CalendarClock size={15}/> Prazo: {formatDateTime(currentTask.due_at)}</p>{/if}
            <div class="rx-card-buttons">
              <button class="rx-primary" disabled={busy} onclick={() => { void completeTask(currentTask); }}><Check size={17}/> Concluir</button>
              <button class="rx-subtle" onclick={() => { void archiveEntry('task:' + currentTask.id); }}><Pencil size={16}/> Detalhes</button>
            </div>
          </div>
        {:else}
          <div class="rx-empty-focus">
            <CircleDot size={26}/>
            <h2>Nada pendente agora.</h2>
            <p>Registre uma ideia ou crie uma tarefa no campo abaixo.</p>
          </div>
        {/if}

        {#if otherTasks.length > 0}
          <div class="rx-next">
            <h3>Em seguida</h3>
            {#each otherTasks.slice(0, 3) as task (task.id)}
              <div class="rx-next-row">
                <button title="Concluir" aria-label={'Concluir ' + task.title} disabled={busy} onclick={() => { void completeTask(task); }}><Circle size={19}/></button>
                <button class="rx-next-title" onclick={() => { void archiveEntry('task:' + task.id); }}>{task.title}</button>
                <button class="rx-focus-link" aria-label={'Focar na tarefa ' + task.title} title="Escolher como foco" onclick={() => chooseFocus(task.id)}><Target size={15}/> Focar</button>
              </div>
            {/each}
            {#if otherTasks.length > 3}<button class="rx-text-link" onclick={() => { void switchView('archive'); }}>Ver todas no arquivo <ArrowRight size={14}/></button>{/if}
          </div>
        {/if}

        {#if laterTasks.length > 0 || somedayTasks.length > 0}
          <div class="rx-time-summary" aria-label="Tarefas fora de hoje">
            {#if laterTasks.length > 0}<button onclick={() => { archiveFilter = 'task'; void switchView('archive'); }}><CalendarClock size={15}/> Depois <span>{laterTasks.length}</span></button>{/if}
            {#if somedayTasks.length > 0}<button onclick={() => { archiveFilter = 'task'; void switchView('archive'); }}><Archive size={15}/> Algum dia <span>{somedayTasks.length}</span></button>{/if}
          </div>
        {/if}

        {#if dueNotes.length > 0}
          <div class="rx-reminders">
            <h3><Bell size={15}/> Lembretes agendados</h3>
            {#each dueNotes.slice(0, 3) as note (note.id)}
              <button onclick={() => { void archiveEntry('note:' + note.id); }}><span>{note.title || 'Nota sem título'}</span><small>{formatDateTime(note.remind_at)}</small></button>
            {/each}
          </div>
        {/if}

        <form class="rx-capture-inline" onsubmit={(e) => { e.preventDefault(); void captureInline(); }}>
          <div class="rx-compose-head">
            <label for="rx-capture-now">Registrar algo</label>
            <div class="rx-compose-modes" role="group" aria-label="Tipo de registro">
              <button type="button" class:rx-active={captureKind === 'idea'} aria-pressed={captureKind === 'idea'} onclick={() => captureKind = 'idea'}>Ideia</button>
              <button type="button" class:rx-active={captureKind === 'task'} aria-pressed={captureKind === 'task'} onclick={() => captureKind = 'task'}>Tarefa</button>
            </div>
          </div>
          <div>
            <input id="rx-capture-now" maxlength="250" bind:value={newText} placeholder={captureKind === 'idea' ? 'Anote antes de esquecer...' : 'Qual é a próxima ação?'} />
            <button class="rx-capture-submit" type="submit" aria-label={captureKind === 'idea' ? 'Guardar ideia' : 'Criar tarefa'} title={captureKind === 'idea' ? 'Guardar ideia' : 'Criar tarefa'} disabled={!newText.trim() || busy}><ArrowRight size={18}/></button>
          </div>
          {#if captureKind === 'task' && smartDraft.recognized}
            <p class="rx-smart-preview"><CalendarClock size={14}/> {smartDraft.label} <span>· Prazo não envia notificação ainda</span></p>
          {/if}
          <p class="rx-compose-tip">Ex.: Revisar slides amanhã às 15h · Enter para salvar</p>
        </form>
      </section>
    {:else}
      <section class="rx-archive" aria-label="Todas as suas informações">
        <div class="rx-items">
          <div class="rx-items-top">
            <h1>Seu arquivo</h1>
            <p>Notas, tarefas e ideias no mesmo lugar.</p>
            <label class="rx-search"><Search size={17}/><input aria-label="Pesquisar tudo" type="search" placeholder="Buscar qualquer coisa..." bind:value={search}/>{#if search}<button type="button" aria-label="Limpar pesquisa" title="Limpar pesquisa" onclick={() => search = ''}><X size={15}/></button>{/if}</label>
            <div class="rx-filters" role="group" aria-label="Filtrar arquivo">
              <button class:rx-active={archiveFilter === 'all'} aria-pressed={archiveFilter === 'all'} onclick={() => archiveFilter = 'all'}>Tudo</button>
              <button class:rx-active={archiveFilter === 'note'} aria-pressed={archiveFilter === 'note'} onclick={() => archiveFilter = 'note'}>Notas</button>
              <button class:rx-active={archiveFilter === 'task'} aria-pressed={archiveFilter === 'task'} onclick={() => archiveFilter = 'task'}>Tarefas</button>
              <button class:rx-active={archiveFilter === 'idea'} aria-pressed={archiveFilter === 'idea'} onclick={() => archiveFilter = 'idea'}>Ideias</button>
            </div>
            <p class="rx-results">{matches.length} {matches.length === 1 ? 'resultado' : 'resultados'}</p>
          </div>
          <div class="rx-item-list">
            {#if matches.length === 0}
              <p class="rx-no-results">{search ? 'Nenhum resultado encontrado.' : 'Ainda não há nada aqui. Use Capturar para começar.'}</p>
            {:else}
              {#each matches as item (item.key)}
                <button class="rx-item" class:rx-selected={selectedKey === item.key} onclick={() => { void openEntry(item.key); }}>
                  <span class="rx-item-top">
                    {#if item.type === 'task'}<Circle size={15}/>
                    {:else if item.type === 'idea'}<Pencil size={15}/>
                    {:else}<FileText size={15}/>{/if}
                    <strong>{item.title}</strong>
                  </span>
                  <span class="rx-item-preview">{item.text || (item.type === 'task' ? 'Tarefa' : 'Nota')}</span>
                  <span class="rx-item-meta">
                    {item.type === 'idea' ? 'Ideia' : item.type === 'task' ? (item.task.completed_at ? 'Concluída' : item.task.bucket === 'someday' ? 'Tarefa · Algum dia' : taskPeriod(item.task, clock) === 'later' ? 'Tarefa · Depois' : 'Tarefa · Hoje') : (item.note.kind === 'presentation' ? 'Apresentação' : 'Nota')}
                    {#if item.type === 'note' && item.note.remind_at}<Bell size={12}/>{/if}
                  </span>
                </button>
              {/each}
            {/if}
          </div>
        </div>

        <div class="rx-inspector">
          {#if !selected}
            <div class="rx-selection-empty">
              <Archive size={28}/>
              <h2>Um lugar para tudo.</h2>
              <p>Escolha algo à esquerda para ver os detalhes ou use Capturar para guardar uma ideia.</p>
            </div>
          {:else if selected.type === 'idea'}
            <div class="rx-details">
              <div class="rx-detail-head"><span>IDEIA GUARDADA</span><button class="rx-icon-button" onclick={() => { void deleteSelected(); }} title="Descartar ideia" aria-label="Descartar ideia"><Trash2 size={17}/></button></div>
              <label class="rx-field"><span>Ideia</span><textarea class="rx-edit-idea" rows="9" maxlength="100000" value={ideaText} oninput={(e) => onIdeaEdit(e.currentTarget.value)} onblur={() => { void saveIdea(selected.idea); }} placeholder="Escreva o que você quer lembrar..."></textarea></label>
              <p class="rx-task-save" aria-live="polite"><Check size={13}/>{ideaSaving ? 'Salvando...' : ideaDirty ? 'Alterações pendentes' : 'Salvo automaticamente'}</p>
              <p class="rx-hint">Você pode editar e guardar aqui, sem transformar em outra coisa.</p>
              <div class="rx-actions">
                <button class="rx-primary" disabled={busy} onclick={() => { void convertIdea(selected.idea, 'note'); }}><FileText size={16}/> Editar como nota</button>
                <button class="rx-subtle" disabled={busy} onclick={() => { void convertIdea(selected.idea, 'task'); }}><Check size={16}/> Transformar em tarefa</button>
              </div>
            </div>
          {:else if selected.type === 'task'}
            <div class="rx-details">
              <div class="rx-detail-head"><span>TAREFA</span><button class="rx-icon-button" onclick={() => { void deleteSelected(); }} title="Excluir tarefa" aria-label="Excluir tarefa"><Trash2 size={17}/></button></div>
              <label class="rx-field"><span>Título</span><input maxlength="250" value={taskTitle} oninput={(e) => onTaskEdit('title', e.currentTarget.value)} onblur={() => { void saveTask(selected.task); }}/></label>
              <label class="rx-field"><span>Próximo passo ou detalhes</span><textarea rows="7" maxlength="100000" value={taskDetails} oninput={(e) => onTaskEdit('details', e.currentTarget.value)} onblur={() => { void saveTask(selected.task); }} placeholder="O que preciso fazer?"></textarea></label>
              <div class="rx-schedule" aria-label="Planejamento da tarefa">
                <span>Quando quer fazer?</span>
                <div role="group" aria-label="Período" class="rx-schedule-options">
                  <button class:rx-active={taskPeriod(selected.task, clock) === 'today'} aria-pressed={taskPeriod(selected.task, clock) === 'today'} disabled={busy} onclick={() => { void changeTaskPeriod(selected.task, 'today'); }}>Hoje</button>
                  <button class:rx-active={taskPeriod(selected.task, clock) === 'later'} aria-pressed={taskPeriod(selected.task, clock) === 'later'} disabled={busy} onclick={() => { void changeTaskPeriod(selected.task, 'later'); }}>Depois</button>
                  <button class:rx-active={taskPeriod(selected.task, clock) === 'someday'} aria-pressed={taskPeriod(selected.task, clock) === 'someday'} disabled={busy} onclick={() => { void changeTaskPeriod(selected.task, 'someday'); }}>Algum dia</button>
                </div>
              </div>
              {#if selected.task.bucket === 'later'}
                <label class="rx-field"><span>Voltar para Hoje em (opcional)</span><input type="date" value={selected.task.planned_for === null ? '' : toLocalInput(selected.task.planned_for).slice(0, 10)} onchange={(e) => { void setPlannedDate(selected.task, e.currentTarget.value); }}/></label>
              {/if}
              <label class="rx-field"><span>Prazo (não envia notificação ainda)</span><input type="datetime-local" value={taskDue} onchange={(e) => onTaskEdit('due', e.currentTarget.value)} onblur={() => { void saveTask(selected.task); }}/></label>
              <p class="rx-task-save" aria-live="polite"><Check size={13}/>{taskSaving ? 'Salvando...' : taskDirty ? 'Alterações pendentes' : 'Salvo automaticamente'}</p>
              <div class="rx-actions">
                <button class="rx-primary" disabled={busy} onclick={() => { void focusAndGo(selected.task.id); }}><Target size={16}/> {focusedTaskId === selected.task.id ? 'Em foco' : 'Focar agora'}</button>
                <button class="rx-subtle" disabled={busy} onclick={() => { void completeTask(selected.task); }}>
                  {#if selected.task.completed_at}<RotateCcw size={16}/> Reabrir{:else}<Circle size={16}/> Concluir tarefa{/if}
                </button>
              </div>
            </div>
          {:else}
            <div class="rx-details">
              <div class="rx-detail-head"><span>{selected.note.kind === 'presentation' ? 'APRESENTAÇÃO' : 'NOTA'}</span><button class="rx-icon-button" onclick={() => { void deleteSelected(); }} title="Excluir nota" aria-label="Excluir nota"><Trash2 size={17}/></button></div>
              <input class="rx-note-title" aria-label="Título da nota" placeholder="Sem título" value={noteTitle} oninput={(e) => onNoteEdit('title', e.currentTarget.value)} onblur={() => { void flushNote(); }}/>
              <textarea class="rx-note-body" aria-label="Texto da nota" placeholder="Escreva aqui..." value={noteBody} oninput={(e) => onNoteEdit('body', e.currentTarget.value)} onblur={() => { void flushNote(); }}></textarea>
              <div class="rx-save-indicator"><Check size={13}/>{noteSaving ? 'Salvando...' : noteDirty ? 'Alterações pendentes' : 'Salvo automaticamente'}</div>
              <div class="rx-note-options">
                <label class="rx-field"><span><Bell size={15}/> Lembrar desta nota</span><input type="datetime-local" value={toLocalInput(selected.note.remind_at)} onchange={(e) => { void updateReminder(selected.note, e.currentTarget.value); }}/></label>
                <div class="rx-actions">
                  <button class="rx-subtle" onclick={() => { void floatNote(selected.note); }}><MonitorUp size={16}/> Abrir cartão flutuante</button>
                  <button class="rx-subtle" onclick={() => { void flushNote().then(ok => { if (ok) { slideIndex = 0; presenting = true; } }); }}><Presentation size={16}/> Apresentar</button>
                </div>
                <p class="rx-hint">Opcional: separe cartões de apresentação com <code>---</code> em uma linha isolada.</p>
              </div>
            </div>
          {/if}
        </div>
      </section>
    {/if}

    {#if presenting && selected?.type === 'note'}
      <div class="rx-presentation" role="dialog" aria-modal="true" aria-label="Apresentação">
        <div class="rx-presentation-head"><span>{noteTitle || 'Apresentação'}</span><button aria-label="Fechar apresentação" onclick={() => presenting = false}><X size={20}/></button></div>
        <div class="rx-presentation-slide">
          <small>{Math.min(slideIndex + 1, Math.max(slides.length, 1))} / {Math.max(slides.length, 1)}</small>
          <p>{slides[slideIndex] || 'Escreva sua explicação na nota para apresentar em cartões.'}</p>
        </div>
        <div class="rx-presentation-foot">
          <button disabled={slideIndex === 0} onclick={() => slideIndex--}><ChevronLeft size={17}/> Anterior</button>
          <span>Use as setas para navegar</span>
          <button disabled={slideIndex >= slides.length - 1} onclick={() => slideIndex++}>Próximo <ChevronRight size={17}/></button>
        </div>
      </div>
    {/if}
  </main>
</div>
