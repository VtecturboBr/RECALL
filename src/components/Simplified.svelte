<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import {
    BrainCircuit, CircleDot, Archive, Search, Plus, Check, Circle, Bell,
    FileText, Pencil, CalendarClock, MonitorUp, Presentation, ChevronLeft,
    ChevronRight, X, Trash2, ArrowRight, Menu, RotateCcw
  } from 'lucide-svelte';
  import { notesApi, tasksApi, inboxApi, formatDateTime, toLocalInput } from '../lib/api';
  import { openQuickCapture, openSticky } from '../lib/windows';
  import { buildWorkspace, nextTask, toSlides } from '../lib/workspace';
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

  let newText = $state('');
  let draftNoteId: string | null = null;
  let noteTitle = $state('');
  let noteBody = $state('');
  let noteDirty = $state(false);
  let noteSaving = $state(false);
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  let saveInFlight: Promise<boolean> | null = null;

  let taskTitle = $state('');
  let taskDetails = $state('');
  let taskDue = $state('');
  let presenting = $state(false);
  let slideIndex = $state(0);
  let selected = $derived(entries.find(item => item.key === selectedKey) ?? null);
  let entries = $derived(buildWorkspace(notes, tasks, ideas));
  let matches = $derived(entries.filter(item =>
    (item.title + ' ' + item.text).toLocaleLowerCase('pt-BR').includes(search.trim().toLocaleLowerCase('pt-BR'))));
  let currentTask = $derived(nextTask(tasks));
  let otherTasks = $derived(tasks.filter(t =>
    t.deleted_at === null && t.completed_at === null && t.id !== currentTask?.id)
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
    let alive = true;
    const unlisteners: Array<() => void> = [];
    void refresh();
    for (const eventName of ['notes:changed', 'tasks:changed', 'inbox:changed']) {
      void listen(eventName, () => { if (alive) void refresh(); })
        .then(dispose => { if (alive) unlisteners.push(dispose); else dispose(); });
    }
    const onKey = (event: KeyboardEvent) => {
      if (event.ctrlKey && event.key.toLowerCase() === 'k') {
        event.preventDefault(); view = 'archive';
        requestAnimationFrame(() => document.querySelector<HTMLInputElement>('.rx-search input')?.focus());
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
      unlisteners.forEach(fn => fn());
      window.removeEventListener('keydown', onKey);
      if (saveTimer) clearTimeout(saveTimer);
      if (noteDirty) void flushNote();
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
      const idea = await inboxApi.capture(newText);
      newText = '';
      await refresh();
      if (view === 'archive') await openEntry('idea:' + idea.id);
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
    if (key !== selectedKey && !(await flushNote())) return;
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
    if (entry?.type === 'task') {
      taskTitle = entry.task.title;
      taskDetails = entry.task.details;
      taskDue = toLocalInput(entry.task.due_at);
    }
  }

  async function switchView(next: View) {
    if (next === view) return;
    if (!(await flushNote())) return;
    presenting = false;
    view = next;
  }

  async function archiveEntry(key: string) {
    if (!(await flushNote())) return;
    view = 'archive';
    await openEntry(key);
  }

  async function completeTask(task: Task) {
    if (busy) return;
    busy = true;
    try {
      await tasksApi.complete(task.id, task.completed_at === null);
      await refresh();
    } catch (e) { error = 'Falha ao alterar tarefa: ' + String(e); }
    finally { busy = false; }
  }

  async function saveTask(task: Task) {
    if (!taskTitle.trim() || busy) return;
    const dueAt = taskDue ? new Date(taskDue).getTime() : null;
    if (dueAt !== null && !Number.isFinite(dueAt)) { error = 'Data inválida.'; return; }
    busy = true;
    try {
      await tasksApi.update(task.id, taskTitle, taskDetails, dueAt);
      await refresh();
    } catch (e) { error = 'Falha ao salvar tarefa: ' + String(e); }
    finally { busy = false; }
  }

  async function updateReminder(note: Note, value: string) {
    if (!(await flushNote())) return;
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
    if (busy) return;
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
    if (!(await flushNote())) return;
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
    <div class="rx-sidebar-foot"><span class="rx-local-dot"></span> Salvo neste computador</div>
  </aside>

  <main class="rx-main">
    <header class="rx-toolbar">
      <button class="rx-icon-button rx-menu" title="Mostrar ou ocultar menu" aria-label="Mostrar ou ocultar menu" onclick={() => sidebarVisible = !sidebarVisible}><Menu size={19}/></button>
      <span class="rx-page-name">{view === 'now' ? 'Agora' : 'Arquivo'}</span>
      <span class="rx-spacer"></span>
      <button class="rx-top-search" onclick={() => { void switchView('archive').then(() => document.querySelector<HTMLInputElement>('.rx-search input')?.focus()); }} title="Pesquisar (Ctrl+K)" aria-label="Pesquisar"><Search size={18}/></button>
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
              <button class="rx-subtle" onclick={() => { void archiveEntry('task:' + currentTask.id); }}><Pencil size={16}/> Editar</button>
            </div>
          </div>
        {:else}
          <div class="rx-empty-focus">
            <CircleDot size={26}/>
            <h2>Nada pendente agora.</h2>
            <p>Guarde uma ideia abaixo. Você poderá transformá-la em tarefa quando precisar.</p>
          </div>
        {/if}

        {#if otherTasks.length > 0}
          <div class="rx-next">
            <h3>Depois</h3>
            {#each otherTasks.slice(0, 3) as task (task.id)}
              <div class="rx-next-row">
                <button title="Concluir" aria-label={'Concluir ' + task.title} disabled={busy} onclick={() => { void completeTask(task); }}><Circle size={19}/></button>
                <button class="rx-next-title" onclick={() => { void archiveEntry('task:' + task.id); }}>{task.title}</button>
              </div>
            {/each}
            {#if otherTasks.length > 3}<button class="rx-text-link" onclick={() => { void switchView('archive'); }}>Ver todas no arquivo <ArrowRight size={14}/></button>{/if}
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
          <label for="rx-capture-now">Não quero esquecer...</label>
          <div>
            <input id="rx-capture-now" maxlength="100000" bind:value={newText} placeholder="Escreva uma ideia e guarde..." />
            <button class="rx-capture-submit" type="submit" aria-label="Guardar ideia" disabled={!newText.trim() || busy}><ArrowRight size={18}/></button>
          </div>
        </form>
      </section>
    {:else}
      <section class="rx-archive" aria-label="Todas as suas informações">
        <div class="rx-items">
          <div class="rx-items-top">
            <h1>Seu arquivo</h1>
            <p>Notas, tarefas e ideias no mesmo lugar.</p>
            <label class="rx-search"><Search size={17}/><input aria-label="Pesquisar tudo" type="search" placeholder="Buscar qualquer coisa..." bind:value={search}/></label>
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
                    {item.type === 'idea' ? 'Ideia' : item.type === 'task' ? (item.task.completed_at ? 'Concluída' : 'Tarefa') : (item.note.kind === 'presentation' ? 'Apresentação' : 'Nota')}
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
              <h2>{selected.title}</h2>
              <div class="rx-idea-text">{selected.idea.text}</div>
              <p class="rx-hint">Sua ideia já está salva. Não precisa classificá-la agora.</p>
              <div class="rx-actions">
                <button class="rx-primary" disabled={busy} onclick={() => { void convertIdea(selected.idea, 'note'); }}><FileText size={16}/> Editar como nota</button>
                <button class="rx-subtle" disabled={busy} onclick={() => { void convertIdea(selected.idea, 'task'); }}><Check size={16}/> Transformar em tarefa</button>
              </div>
            </div>
          {:else if selected.type === 'task'}
            <div class="rx-details">
              <div class="rx-detail-head"><span>TAREFA</span><button class="rx-icon-button" onclick={() => { void deleteSelected(); }} title="Excluir tarefa" aria-label="Excluir tarefa"><Trash2 size={17}/></button></div>
              <label class="rx-field"><span>Título</span><input maxlength="250" bind:value={taskTitle}/></label>
              <label class="rx-field"><span>Próximo passo ou detalhes</span><textarea rows="7" maxlength="100000" bind:value={taskDetails} placeholder="O que preciso fazer?"></textarea></label>
              <label class="rx-field"><span>Prazo (ainda não envia notificação)</span><input type="datetime-local" bind:value={taskDue}/></label>
              <div class="rx-actions">
                <button class="rx-primary" disabled={busy || !taskTitle.trim()} onclick={() => { void saveTask(selected.task); }}><Check size={16}/> Salvar alterações</button>
                <button class="rx-subtle" disabled={busy} onclick={() => { void completeTask(selected.task); }}>
                  {#if selected.task.completed_at}<RotateCcw size={16}/> Reabrir{:else}<Circle size={16}/> Concluir tarefa{/if}
                </button>
              </div>
            </div>
          {:else}
            <div class="rx-details">
              <div class="rx-detail-head"><span>NOTA</span><button class="rx-icon-button" onclick={() => { void deleteSelected(); }} title="Excluir nota" aria-label="Excluir nota"><Trash2 size={17}/></button></div>
              <input class="rx-note-title" aria-label="Título da nota" placeholder="Sem título" value={noteTitle} oninput={(e) => onNoteEdit('title', e.currentTarget.value)} onblur={() => { void flushNote(); }}/>
              <textarea class="rx-note-body" aria-label="Texto da nota" placeholder="Escreva aqui..." value={noteBody} oninput={(e) => onNoteEdit('body', e.currentTarget.value)} onblur={() => { void flushNote(); }}></textarea>
              <div class="rx-save-indicator"><Check size={13}/>{noteSaving ? 'Salvando...' : noteDirty ? 'Alterações pendentes' : 'Salvo automaticamente'}</div>
              <div class="rx-note-options">
                <label class="rx-field"><span><Bell size={15}/> Lembrar desta nota</span><input type="datetime-local" value={toLocalInput(selected.note.remind_at)} onchange={(e) => { void updateReminder(selected.note, e.currentTarget.value); }}/></label>
                <div class="rx-actions">
                  <button class="rx-subtle" onclick={() => { void floatNote(selected.note); }}><MonitorUp size={16}/> Fixar no desktop</button>
                  <button class="rx-subtle" onclick={() => { void flushNote().then(ok => { if (ok) { slideIndex = 0; presenting = true; } }); }}><Presentation size={16}/> Apresentar</button>
                </div>
                <p class="rx-hint">Para separar uma apresentação em cartões, escreva <code>---</code> em uma linha isolada.</p>
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
