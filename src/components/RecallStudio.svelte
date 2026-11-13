<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import {
    BrainCircuit, CircleDot, Archive, Search, Plus, Check, Circle, Bell,
    FileText, Pencil, CalendarClock, MonitorUp, Presentation, ChevronLeft,
    ChevronRight, X, Trash2, ArrowRight, Menu, RotateCcw, Target, ShieldCheck, AlarmClock, Repeat2, CheckCheck,
    Command, ArrowUpRight, CalendarDays, MoreHorizontal, Pin, Sparkles, LayoutGrid, Clock3, Inbox, Keyboard, PanelLeftClose, PanelLeftOpen, Coffee, CheckCircle2, Sun, NotebookPen
  } from 'lucide-svelte';
  import { notesApi, tasksApi, inboxApi, backupsApi, remindersApi, formatDateTime, toLocalInput } from '../lib/api';
  import { openQuickCapture, openSticky } from '../lib/windows';
  import TaskResume from './TaskResume.svelte';
  import { buildWorkspace, nextTask, toSlides, searchWorkspace, taskPeriod, tasksByPeriod } from '../lib/workspace';
  import { parseTaskCapture } from '../lib/smartCapture';
  import type { Note, Task, InboxItem, Reminder, ReminderEvent, ReminderRule } from '../lib/types';

  type View = 'now' | 'archive';
  let view = $state<View>('now');
  let notes = $state<Note[]>([]);
  let tasks = $state<Task[]>([]);
  let ideas = $state<InboxItem[]>([]);
  let reminders = $state<Reminder[]>([]);
  let reminderHistory = $state<ReminderEvent[]>([]);
  let reminderBusy = $state(false);
  let reminderAt = $state('');
  let reminderRepeat = $state<ReminderRule>('none');
  let showAllReminders = $state(false);
  const repeatLabel: Record<ReminderRule, string> = {
    none: 'Uma vez', daily: 'Diariamente', weekly: 'Semanalmente', monthly: 'Mensalmente'
  };
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
  let archivePeriod = $state<'all' | 'today' | 'later' | 'someday'>('all');
  let openTaskCount = $derived(tasks.filter(t => t.deleted_at === null && t.completed_at === null).length);
  let dateHeading = $derived(new Intl.DateTimeFormat('pt-BR', { weekday: 'long', day: 'numeric', month: 'long' }).format(clock));
  const FOCUS_KEY = 'recall-selected-focus-task-v1';

  let newText = $state('');
  const INLINE_DRAFT_KEY = 'recall-inline-capture-draft-v1';
  let captureDraftReady = $state(false);
  // The capture survives a normal restart and a process crash, independently of SQLite.
  $effect(() => {
    if (!captureDraftReady) return;
    const text = newText;
    const kind = captureKind;
    try {
      if (text.length > 0) {
        localStorage.setItem(INLINE_DRAFT_KEY, JSON.stringify({ text, kind, savedAt: Date.now() }));
      } else {
        localStorage.removeItem(INLINE_DRAFT_KEY);
      }
    } catch { /* Local storage may be unavailable; a failed DB write still leaves the composer intact. */ }
  });
  let draftNoteId: string | null = null;
  let noteTitle = $state('');
  let noteBody = $state('');
  let noteDirty = $state(false);
  let noteRevision = $state(0);
  let noteConflict = $state(false);
  let noteRemoteChanged = $state(false);
  let backupBusy = $state(false);
  let backupMessage = $state('');
  function draftKey(id: string) { return 'recall-note-draft:' + id; }
  function preserveNoteDraft() {
    if (!draftNoteId) return;
    try {
      localStorage.setItem(draftKey(draftNoteId),
        JSON.stringify({title:noteTitle,body:noteBody,revision:noteRevision}));
    } catch { /* Database save remains authoritative; storage may be full. */ }
  }
  function discardLocalDraftByKey(key: string) {
    try { localStorage.removeItem(key); } catch { /* Storage unavailable. */ }
  }
  function discardLocalDraft(id: string) { discardLocalDraftByKey(draftKey(id)); }
  let noteSaving = $state(false);
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  let saveInFlight: Promise<boolean> | null = null;

  let ideaText = $state('');
  let ideaDirty = $state(false);
  let ideaSaving = $state(false);
  let ideaConflict = $state(false);
  let ideaRevision = $state(0);
  let draftIdeaId: string | null = null;
  function ideaDraftKey(id: string) { return 'recall-idea-draft:' + id; }
  function preserveIdeaDraft() {
    if (!draftIdeaId) return;
    try { localStorage.setItem(ideaDraftKey(draftIdeaId), JSON.stringify({ text: ideaText, revision: ideaRevision })); }
    catch { /* Writing to SQLite remains authoritative. */ }
  }
  let ideaSaveTimer: ReturnType<typeof setTimeout> | undefined;
  let ideaSaveInFlight: Promise<boolean> | null = null;

  let taskTitle = $state('');
  let taskDetails = $state('');
  let taskDue = $state('');
  let taskDirty = $state(false);
  let taskSaving = $state(false);
  let taskConflict = $state(false);
  let taskRevision = $state(0);
  let draftTaskId: string | null = null;
  function taskDraftKey(id: string) { return 'recall-task-draft:' + id; }
  function preserveTaskDraft() {
    if (!draftTaskId) return;
    try { localStorage.setItem(taskDraftKey(draftTaskId), JSON.stringify({
      title: taskTitle, details: taskDetails, due: taskDue, revision: taskRevision
    })); } catch { /* Writing to SQLite remains authoritative. */ }
  }
  let taskSaveTimer: ReturnType<typeof setTimeout> | undefined;
  let taskSaveInFlight: Promise<boolean> | null = null;
  let presenting = $state(false);
  let slideIndex = $state(0);
  let entries = $derived(buildWorkspace(notes, tasks, ideas));
  let selected = $derived(entries.find(item => item.key === selectedKey) ?? null);
  let matches = $derived(searchWorkspace(entries, search, archiveFilter));
  let visibleEntries = $derived(matches.filter(item =>
    archivePeriod === 'all' || (item.type === 'task'
      && item.task.completed_at === null && taskPeriod(item.task, clock) === archivePeriod)
  ));
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
  let attentionReminders = $derived(reminders.filter(r => r.state === 'attention')
    .sort((a,b)=>a.due_at-b.due_at));
  let upcomingReminders = $derived(reminders.filter(r => r.state === 'scheduled' || r.state === 'snoozed')
    .sort((a,b)=>a.due_at-b.due_at));
  let selectedReminder = $derived.by(() => {
    const entry = selected;
    if (!entry || entry.type === 'idea') return null;
    const id = entry.type === 'note' ? entry.note.id : entry.task.id;
    return reminders.find(r => r.target_type === entry.type && r.target_id === id && r.state !== 'cancelled') ?? null;
  });
  let slides = $derived(toSlides(noteBody));
  function slidePositionKey(id: string) { return 'recall-presentation-position:' + id; }
  function moveSlide(amount: number) {
    slideIndex = Math.max(0, Math.min(slideIndex + amount, Math.max(0, slides.length - 1)));
    if (draftNoteId) {
      try { localStorage.setItem(slidePositionKey(draftNoteId), String(slideIndex)); } catch { /* Best-effort position cache. */ }
    }
  }
  async function openPresentation() {
    if (!(await flushNote()) || !draftNoteId) return;
    let restored = 0;
    try {
      const saved = Number(localStorage.getItem(slidePositionKey(draftNoteId)) ?? '0');
      if (Number.isInteger(saved) && saved >= 0) restored = saved;
    } catch { /* Start at first card when storage is unavailable. */ }
    slideIndex = Math.min(restored, Math.max(0, slides.length - 1));
    presenting = true;
  }


  let refreshGeneration = 0;
  async function refresh(): Promise<void> {
    const generation = ++refreshGeneration;
    try {
      const [n, t, i, r, h] = await Promise.all([
        notesApi.list(), tasksApi.list(), inboxApi.list(), remindersApi.list(), remindersApi.history()
      ]);
      // Do not let an older IPC response undo a newer user action.
      if (generation !== refreshGeneration) return;
      const activeNote = draftNoteId ? n.find(row => row.id === draftNoteId) : null;
      if (activeNote) {
        if (!noteDirty && !noteSaving) {
          noteTitle = activeNote.title;
          noteBody = activeNote.body;
          noteRevision = activeNote.revision;
          noteRemoteChanged = false;
        } else if (!noteSaving && activeNote.revision !== noteRevision) noteRemoteChanged = true;
      }
      const activeTask = draftTaskId ? t.find(row => row.id === draftTaskId) : null;
      if (activeTask) {
        if (!taskDirty && !taskSaving) {
          taskTitle = activeTask.title;
          taskDetails = activeTask.details;
          taskDue = toLocalInput(activeTask.due_at);
          taskRevision = activeTask.revision;
        } else if (!taskSaving && activeTask.revision !== taskRevision) taskConflict = true;
      }
      const activeIdea = draftIdeaId ? i.find(row => row.id === draftIdeaId) : null;
      if (activeIdea) {
        if (!ideaDirty && !ideaSaving) {
          ideaText = activeIdea.text;
          ideaRevision = activeIdea.revision;
        } else if (!ideaSaving && activeIdea.revision !== ideaRevision) ideaConflict = true;
      }
      notes = n;
      tasks = t;
      ideas = i;
      reminders = r;
      reminderHistory = h;
      error = '';
    } catch (e) {
      if (generation === refreshGeneration) error = 'Não foi possível carregar seus dados: ' + String(e);
    } finally {
      if (generation === refreshGeneration) loading = false;
    }
  }

  onMount(() => {
    try { focusedTaskId = localStorage.getItem(FOCUS_KEY); } catch { /* No local storage available. */ }
    try {
      const stored = localStorage.getItem(INLINE_DRAFT_KEY);
      if (stored) {
        const draft: unknown = JSON.parse(stored);
        if (draft && typeof draft === 'object' && 'text' in draft && typeof draft.text === 'string') {
          newText = draft.text.slice(0, 100000);
          if ('kind' in draft && (draft.kind === 'task' || draft.kind === 'idea')) captureKind = draft.kind;
        }
      }
    } catch { /* Invalid/blocked storage must not prevent starting RECALL. */ }
    captureDraftReady = true;
    let alive = true;
    const narrowViewport = window.matchMedia('(max-width: 680px)');
    if (narrowViewport.matches) sidebarVisible = false;
    const onNarrowChange = (event: MediaQueryListEvent) => {
      if (event.matches) sidebarVisible = false;
    };
    narrowViewport.addEventListener('change', onNarrowChange);
    const dayClock = setInterval(() => { clock = new Date(); }, 60_000);
    const unlisteners: Array<() => void> = [];
    void refresh();
    for (const eventName of ['notes:changed', 'tasks:changed', 'inbox:changed', 'reminders:changed']) {
      void listen(eventName, () => { if (alive) void refresh(); })
        .then(dispose => { if (alive) unlisteners.push(dispose); else dispose(); });
    }
    const onKey = (event: KeyboardEvent) => {
      if (event.ctrlKey && event.key.toLowerCase() === 'k') {
        event.preventDefault();
        void switchView('archive').then(() => {
          if (view === 'archive') requestAnimationFrame(() => document.querySelector<HTMLInputElement>('.re-search input')?.focus());
        });
      }
      if (event.ctrlKey && event.key.toLowerCase() === 'n') {
        event.preventDefault(); void captureWindow();
      }
      if (event.key === 'Escape') {
        presenting = false;
        if (narrowViewport.matches) sidebarVisible = false;
      }
      if (presenting && event.key === 'ArrowRight') moveSlide(1);
      if (presenting && event.key === 'ArrowLeft') moveSlide(-1);
    };
    window.addEventListener('keydown', onKey);
    return () => {
      alive = false;
      clearInterval(dayClock);
      narrowViewport.removeEventListener('change', onNarrowChange);
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
      const originalText = newText;
      const originalKind = captureKind;
      const text = originalText.trim();
      if (originalKind === 'task') {
        const draft = parseTaskCapture(text, new Date());
        if (!draft.title.trim() || draft.title.length > 250) throw new Error('Use um título de até 250 caracteres.');
        const task = await tasksApi.createPlanned(draft.title, draft.details, draft.bucket, draft.plannedFor, draft.dueAt);
        if (!currentTask && draft.bucket === 'today') chooseFocus(task.id);
      } else {
        await inboxApi.capture(text);
      }
      // Do not erase newer typing made while the asynchronous database write was in flight.
      // The reactive draft persists until the database confirms success.
      if (newText === originalText && captureKind === originalKind) newText = '';
      await refresh();
    } catch (e) {
      error = 'A anotação não foi salva: ' + String(e);
    } finally { busy = false; }
  }

  function onNoteEdit(field: 'title' | 'body', text: string) {
    if (field === 'title') noteTitle = text;
    else noteBody = text;
    noteDirty = true;
    preserveNoteDraft();
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
    const expectedRevision = noteRevision;
    noteDirty = false;
    noteSaving = true;
    const operation: Promise<boolean> = notesApi.save(id, title, body, expectedRevision)
      .then(saved => {
        notes = notes.map(n => n.id === saved.id ? saved : n);
        if (draftNoteId === id) {
          noteRevision = saved.revision;
          noteConflict = false;
          noteRemoteChanged = false;
          if (!noteDirty) discardLocalDraft(id);
        }
        return true;
      })
      .catch(e => {
        noteDirty = true;
        noteConflict = String(e).includes('CONFLITO_REVISAO');
        preserveNoteDraft();
        error = noteConflict
          ? 'Esta nota mudou em outra janela. Suas alterações foram preservadas; salve uma cópia antes de continuar.'
          : 'Não foi possível salvar sua nota. O rascunho continua no editor: ' + String(e);
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
    if (entry && entry.type !== 'idea') {
      const entryId = entry.type === 'note' ? entry.note.id : entry.task.id;
      const reminder = reminders.find(r => r.target_type === entry.type && r.target_id === entryId && r.state !== 'cancelled' && r.state !== 'done');
      reminderAt = reminder ? toLocalInput(reminder.due_at)
        : entry.type === 'task' ? toLocalInput(entry.task.due_at) : '';
      reminderRepeat = reminder?.repeat_rule ?? 'none';
    } else { reminderAt = ''; reminderRepeat = 'none'; }
    if (entry?.type === 'note') {
      draftNoteId = entry.note.id;
      noteTitle = entry.note.title;
      noteBody = entry.note.body;
      noteRevision = entry.note.revision;
      noteConflict = false;
      noteRemoteChanged = false;
      noteDirty = false;
      try {
        const cached = localStorage.getItem(draftKey(entry.note.id));
        if (cached) {
          const draft = JSON.parse(cached) as {title?:string;body?:string;revision?:number};
          if (typeof draft.title === 'string' && typeof draft.body === 'string'
              && (draft.title !== entry.note.title || draft.body !== entry.note.body)) {
            noteTitle = draft.title;
            noteBody = draft.body;
            noteRevision = typeof draft.revision === 'number' ? draft.revision : entry.note.revision;
            noteDirty = true;
            error = 'Rascunho local recuperado para esta nota. Confira antes de continuar.';
          } else { discardLocalDraft(entry.note.id); }
        }
      } catch { /* Invalid cached draft is left untouched. */ }
    } else {
      draftNoteId = null;
      noteDirty = false;
      noteConflict = false;
      noteRemoteChanged = false;
    }
    draftIdeaId = entry?.type === 'idea' ? entry.idea.id : null;
    draftTaskId = entry?.type === 'task' ? entry.task.id : null;
    ideaConflict = false;
    taskConflict = false;
    if (entry?.type === 'idea') {
      ideaText = entry.idea.text;
      ideaRevision = entry.idea.revision;
      ideaDirty = false;
      try {
        const cached = localStorage.getItem(ideaDraftKey(entry.idea.id));
        if (cached) {
          const draft = JSON.parse(cached) as { text?: unknown; revision?: unknown };
          if (typeof draft.text === 'string' && draft.text !== entry.idea.text) {
            ideaText = draft.text;
            ideaRevision = typeof draft.revision === 'number' ? draft.revision : entry.idea.revision;
            ideaDirty = true;
            error = 'Rascunho da ideia recuperado. Confira as alterações.';
          } else if (draft.text === entry.idea.text) discardLocalDraftByKey(ideaDraftKey(entry.idea.id));
        }
      } catch { /* Keep unreadable cache unchanged for manual recovery. */ }
    }
    if (entry?.type === 'task') {
      taskTitle = entry.task.title;
      taskDetails = entry.task.details;
      taskDue = toLocalInput(entry.task.due_at);
      taskRevision = entry.task.revision;
      taskDirty = false;
      try {
        const cached = localStorage.getItem(taskDraftKey(entry.task.id));
        if (cached) {
          const draft = JSON.parse(cached) as { title?: unknown; details?: unknown; due?: unknown; revision?: unknown };
          if (typeof draft.title === 'string' && typeof draft.details === 'string' && typeof draft.due === 'string') {
            if (draft.title !== taskTitle || draft.details !== taskDetails || draft.due !== taskDue) {
              taskTitle = draft.title;
              taskDetails = draft.details;
              taskDue = draft.due;
              taskRevision = typeof draft.revision === 'number' ? draft.revision : entry.task.revision;
              taskDirty = true;
              error = 'Rascunho da tarefa recuperado. Confira as alterações.';
            } else discardLocalDraftByKey(taskDraftKey(entry.task.id));
          }
        }
      } catch { /* Keep unreadable cache unchanged for manual recovery. */ }
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
    archivePeriod = 'all';
    archiveFilter = 'all';
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
      await tasksApi.plan(task.id, bucket, bucket === 'later' && taskPeriod(task, clock) === 'later' ? task.planned_for : null);
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
    preserveTaskDraft();
    if (taskSaveTimer) clearTimeout(taskSaveTimer);
    const id = selected?.type === 'task' ? selected.task.id : null;
    taskSaveTimer = setTimeout(() => {
      if (selected?.type === 'task' && selected.task.id === id) void saveTask(selected.task);
    }, 900);
  }

  function onIdeaEdit(value: string) {
    ideaText = value;
    ideaDirty = true;
    preserveIdeaDraft();
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
    const expectedRevision = ideaRevision;
    ideaDirty = false;
    ideaSaving = true;
    const write: Promise<boolean> = inboxApi.update(id, value, expectedRevision)
      .then(updated => {
        ideas = ideas.map(i => i.id === updated.id ? updated : i);
        if (draftIdeaId === id) {
          ideaRevision = updated.revision;
          ideaConflict = false;
          if (!ideaDirty) discardLocalDraftByKey(ideaDraftKey(id));
          else preserveIdeaDraft();
        }
        return true;
      })
      .catch(e => {
        ideaDirty = true;
        ideaConflict = String(e).includes('CONFLITO_REVISAO');
        preserveIdeaDraft();
        error = ideaConflict
          ? 'Conflito de edição: esta ideia foi alterada em outra janela. Seu texto está preservado.'
          : 'Não foi possível salvar a ideia. Rascunho preservado: ' + String(e);
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
    const expectedRevision = taskRevision;
    taskDirty = false;
    taskSaving = true;
    const write: Promise<boolean> = tasksApi.update(id, title, details, dueAt, expectedRevision)
      .then(updated => {
        tasks = tasks.map(t => t.id === updated.id ? updated : t);
        if (draftTaskId === id) {
          taskRevision = updated.revision;
          taskConflict = false;
          if (!taskDirty) discardLocalDraftByKey(taskDraftKey(id));
          else preserveTaskDraft();
        }
        return true;
      })
      .catch(e => {
        taskDirty = true;
        taskConflict = String(e).includes('CONFLITO_REVISAO');
        preserveTaskDraft();
        error = taskConflict
          ? 'Conflito de edição: esta tarefa foi alterada em outra janela. Seu texto está preservado.'
          : 'Não foi possível salvar a tarefa. Rascunho preservado: ' + String(e);
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

  async function saveConflictCopy() {
    if (!selected || selected.type !== 'note' || !draftNoteId || backupBusy) return;
    const originalId = draftNoteId;
    try {
      const saved = await notesApi.copy(noteTitle, noteBody, selected.note.kind);
      discardLocalDraft(originalId);
      noteDirty = false;
      noteConflict = false;
      noteRemoteChanged = false;
      draftNoteId = saved.id;
      noteRevision = saved.revision;
      notes = [saved, ...notes.filter(n => n.id !== saved.id)];
      selectedKey = 'note:' + saved.id;
      error = 'Cópia de segurança da edição criada como uma nova nota.';
    } catch (e) {
      preserveNoteDraft();
      error = 'Falha ao salvar cópia. O texto continua no editor: ' + String(e);
    }
  }

  async function saveTextConflictCopy() {
    const entry = selected;
    if (!entry || busy) return;
    busy = true;
    try {
      if (entry.type === 'idea') {
        const created = await inboxApi.capture(ideaText);
        discardLocalDraftByKey(ideaDraftKey(entry.idea.id));
        ideaDirty = false; ideaConflict = false;
        await refresh();
        await openEntry('idea:' + created.id);
      } else if (entry.type === 'task') {
        const dueAt = taskDue ? new Date(taskDue).getTime() : null;
        if (!taskTitle.trim() || (dueAt !== null && !Number.isFinite(dueAt))) {
          throw new Error('Verifique o título e a data antes de criar a cópia.');
        }
        const created = await tasksApi.createPlanned(
          taskTitle, taskDetails, entry.task.bucket, entry.task.planned_for, dueAt
        );
        discardLocalDraftByKey(taskDraftKey(entry.task.id));
        taskDirty = false; taskConflict = false;
        await refresh();
        search = ''; archiveFilter = 'all'; archivePeriod = 'all';
        await openEntry('task:' + created.id);
      }
      error = 'Cópia salva separadamente. O item original foi mantido.';
    } catch (e) {
      if (entry.type === 'idea') preserveIdeaDraft();
      if (entry.type === 'task') preserveTaskDraft();
      error = 'Falha ao criar uma cópia. Sua edição continua preservada: ' + String(e);
    } finally { busy = false; }
  }

  async function createBackup() {
    if (backupBusy || !(await flushDraft())) return;
    backupBusy = true;
    backupMessage = '';
    try {
      const file = await backupsApi.create();
      backupMessage = 'Backup verificado: ' + file.file_path;
    } catch (e) { error = 'Falha ao criar backup: ' + String(e); }
    finally { backupBusy = false; }
  }

  async function saveReminder() {
    const entry = selected;
    if (!entry || entry.type === 'idea' || reminderBusy || !(await flushDraft())) return;
    const timestamp = reminderAt ? new Date(reminderAt).getTime() : NaN;
    if (!Number.isFinite(timestamp) || timestamp < 1) {
      error = 'Escolha uma data e horário válidos para ser avisado.';
      return;
    }
    const id = entry.type === 'note' ? entry.note.id : entry.task.id;
    reminderBusy = true;
    try {
      await remindersApi.schedule(entry.type, id, timestamp, reminderRepeat);
      await refresh();
    } catch (e) { error = 'Não foi possível agendar: ' + String(e); }
    finally { reminderBusy = false; }
  }

  async function removeReminder() {
    const entry = selected;
    if (!entry || entry.type === 'idea' || reminderBusy || !(await flushDraft())) return;
    const id = entry.type === 'note' ? entry.note.id : entry.task.id;
    reminderBusy = true;
    try {
      await remindersApi.cancel(entry.type, id);
      reminderAt = ''; reminderRepeat = 'none';
      await refresh();
    } catch (e) { error = 'Não foi possível desativar lembrete: ' + String(e); }
    finally { reminderBusy = false; }
  }

  function updateReminderEditor(item: Reminder) {
    const entry = selected;
    if (!entry || entry.type === 'idea') return;
    const selectedId = entry.type === 'task' ? entry.task.id : entry.note.id;
    if (entry.type !== item.target_type || selectedId !== item.target_id) return;
    reminderAt = item.state === 'done' ? '' : toLocalInput(item.due_at);
    reminderRepeat = item.state === 'done' ? 'none' : item.repeat_rule;
  }

  async function snoozeReminder(item: Reminder, minutes: 5 | 15 | 30 | 60 | 180 | 1440) {
    if (reminderBusy || !(await flushDraft())) return;
    reminderBusy = true;
    try {
      const result = await remindersApi.snooze(item.id, minutes);
      updateReminderEditor(result);
      await refresh();
    } catch (e) { error = 'Não foi possível adiar: ' + String(e); }
    finally { reminderBusy = false; }
  }

  async function acknowledgeReminder(item: Reminder) {
    if (reminderBusy || !(await flushDraft())) return;
    reminderBusy = true;
    try {
      const result = await remindersApi.acknowledge(item.id);
      updateReminderEditor(result);
      await refresh();
    } catch (e) { error = 'Não foi possível concluir o lembrete: ' + String(e); }
    finally { reminderBusy = false; }
  }

  async function openReminder(item: Reminder) {
    await archiveEntry(item.target_type + ':' + item.target_id);
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
        search = ''; archiveFilter = 'all'; archivePeriod = 'all';
        await openEntry('note:' + created.id);
      } else {
        const created = await inboxApi.toTask(item.id);
        await refresh();
        search = ''; archiveFilter = 'all'; archivePeriod = 'all';
        await openEntry('task:' + created.id);
      }
    } catch (e) { error = 'Falha ao organizar a ideia: ' + String(e); }
    finally { busy = false; }
  }


  async function setCategory(next: 'all' | 'note' | 'task' | 'idea') {
    if (!(await flushDraft())) return;
    archiveFilter = next;
    archivePeriod = 'all';
    selectedKey = null;
  }

  async function openPeriod(next: 'all' | 'today' | 'later' | 'someday') {
    if (!(await flushDraft())) return;
    archiveFilter = 'task';
    archivePeriod = next;
    search = '';
    selectedKey = null;
    view = 'archive';
  }

  async function createDocument(kind: 'note' | 'presentation') {
    if (busy || !(await flushDraft())) return;
    busy = true;
    try {
      const created = await notesApi.create(kind);
      await refresh();
      archiveFilter = 'all';
      archivePeriod = 'all';
      search = '';
      view = 'archive';
      await openEntry('note:' + created.id);
    } catch (e) { error = 'Não foi possível criar sua nota: ' + String(e); }
    finally { busy = false; }
  }

  async function togglePin(note: Note) {
    if (busy || !(await flushDraft())) return;
    busy = true;
    try { await notesApi.pin(note.id, !note.pinned); await refresh(); }
    catch (e) { error = 'Falha ao fixar nota: ' + String(e); }
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

<svelte:head><title>RECALL — seu espaço de clareza</title></svelte:head>

<div class="re-app" class:re-collapsed={!sidebarVisible}>
  <button type="button" class="re-sidebar-backdrop" aria-label="Fechar menu lateral" onclick={() => sidebarVisible = false}></button>
  <aside class="re-sidebar" aria-label="Navegação do RECALL">
    <div class="re-brand">
      <div class="re-brand-mark" aria-hidden="true"><BrainCircuit size={22} strokeWidth={1.8}/></div>
      <div><strong>RECALL</strong><small>ESPAÇO PESSOAL</small></div>
    </div>

    <div class="re-navgroup-label">ESPAÇO</div>
    <nav class="re-nav" aria-label="Seções">
      <button class:active={view === 'now'} aria-current={view === 'now' ? 'page' : undefined} onclick={() => { void switchView('now'); }}>
        <CircleDot size={18}/><span>Agora</span>
      </button>
      <button class:active={view === 'archive'} aria-current={view === 'archive' ? 'page' : undefined} onclick={() => { void switchView('archive'); }}>
        <LayoutGrid size={18}/><span>Arquivo</span><span class="re-nav-number">{entries.length}</span>
      </button>
    </nav>

    <div class="re-sidebar-divider"></div>
    <div class="re-navgroup-label">ORGANIZAÇÃO</div>
    <nav class="re-nav re-nav-secondary" aria-label="Planejamento">
      <button onclick={() => { void openPeriod('today'); }}><Sun size={17}/><span>Hoje</span><span class="re-nav-number">{todayTasks.length}</span></button>
      <button onclick={() => { void openPeriod('later'); }}><CalendarDays size={17}/><span>Depois</span><span class="re-nav-number">{laterTasks.length}</span></button>
      <button onclick={() => { void openPeriod('someday'); }}><Coffee size={17}/><span>Algum dia</span><span class="re-nav-number">{somedayTasks.length}</span></button>
    </nav>

    <div class="re-side-bottom">
      <div class="re-shortcut"><Command size={14}/><span>Captura rápida</span><kbd>Ctrl ⇧ Espaço</kbd></div>
      <div class="re-system-state"><span class="re-status-dot"></span> Dados no dispositivo <span>α6</span></div>
    </div>
  </aside>

  <div class="re-workspace">
    <header class="re-topbar">
      <button class="re-icon re-navtoggle" aria-label={sidebarVisible ? 'Recolher menu' : 'Expandir menu'} title="Alternar menu" onclick={() => sidebarVisible = !sidebarVisible}>
        {#if sidebarVisible}<PanelLeftClose size={19}/>{:else}<PanelLeftOpen size={19}/>{/if}
      </button>
      <div class="re-breadcrumb"><span>WORKSPACE</span><ChevronRight size={13}/><strong>{view === 'now' ? 'Agora' : 'Arquivo'}</strong></div>
      <div class="re-top-actions">
        <button class="re-icon" title="Pesquisar no arquivo (Ctrl+K)" aria-label="Pesquisar" onclick={() => { void switchView('archive').then(() => requestAnimationFrame(() => document.querySelector<HTMLInputElement>('.re-search input')?.focus())); }}><Search size={18}/></button>
        <button class="re-icon" title="Criar backup dos dados" aria-label="Criar backup dos dados" disabled={backupBusy} onclick={() => { void createBackup(); }}><ShieldCheck size={19}/></button>
        <span class="re-toolbar-separator"></span>
        <button class="re-primary re-capture-trigger" onclick={() => { void captureWindow(); }}><Plus size={17}/> Capturar <span class="re-primary-key">Ctrl N</span></button>
      </div>
    </header>

    {#if error}
      <div class="re-banner re-banner-error" role="alert">
        <span>{error}</span><button aria-label="Fechar mensagem" onclick={() => error = ''}><X size={16}/></button>
      </div>
    {/if}
    {#if backupMessage}
      <div class="re-banner re-banner-success" role="status">
        <ShieldCheck size={17}/><span>{backupMessage}</span><button aria-label="Fechar confirmação" onclick={() => backupMessage = ''}><X size={16}/></button>
      </div>
    {/if}
    {#if (noteConflict || noteRemoteChanged) && selected?.type === 'note'}
      <div class="re-banner re-banner-conflict" role="alert">
        <span>Esta nota foi modificada em outra janela. Seu texto local não foi substituído.</span>
        <button class="re-small-action" onclick={() => { void saveConflictCopy(); }}>Salvar como outra nota</button>
      </div>
    {/if}
    {#if (ideaConflict && selected?.type === 'idea') || (taskConflict && selected?.type === 'task')}
      <div class="re-banner re-banner-conflict" role="alert">
        <span>Este item mudou em outra janela. O conteúdo que você escreveu está preservado.</span>
        <button class="re-small-action" disabled={busy} onclick={() => { void saveTextConflictCopy(); }}>Salvar como novo item</button>
      </div>
    {/if}

    {#if loading}
      <div class="re-loading"><div class="re-loading-mark"><BrainCircuit size={25}/></div><p>Abrindo seu espaço...</p></div>
    {:else if view === 'now'}
      <div class="re-scroll re-now">
        <div class="re-container">
          <div class="re-page-intro">
            <p class="re-eyebrow"><span class="re-eyebrow-dot"></span> SEU ESPAÇO DE CLAREZA</p>
            <h1>Uma coisa de cada vez<span class="re-title-dot">.</span></h1>
            <p class="re-subhead">O importante primeiro. O resto fica guardado.</p>
            <div class="re-date"><CalendarDays size={15}/>{dateHeading}</div>
          </div>

          <section class="re-focus" aria-label="Tarefa em foco">
            <div class="re-focus-top"><span><Target size={16}/> EM FOCO</span><span class="re-focus-number">01 / AGORA</span></div>
            {#if currentTask}
              <div class="re-focus-content">
                <p class="re-focus-label">Seu próximo passo</p>
                <h2>{currentTask.title}</h2>
                {#if currentTask.details}<p class="re-focus-details">{currentTask.details}</p>{/if}
                {#if currentTask.due_at}<p class="re-focus-due"><Clock3 size={15}/> Prazo: {formatDateTime(currentTask.due_at)}</p>{/if}
                <div class="re-focus-actions">
                  <button class="re-primary" disabled={busy} onclick={() => { void completeTask(currentTask); }}><Check size={17}/> Marcar como concluída</button>
                  <button class="re-outline" onclick={() => { void archiveEntry('task:' + currentTask.id); }}>Abrir detalhes <ArrowUpRight size={15}/></button>
                </div>
                {#key currentTask.id}
                  <TaskResume taskId={currentTask.id} taskTitle={currentTask.title}/>
                {/key}
              </div>
            {:else}
              <div class="re-focus-content re-focus-empty">
                <div class="re-empty-icon"><CheckCircle2 size={25}/></div>
                <h2>Tudo tranquilo por aqui.</h2>
                <p>Nenhuma tarefa para agora. Você pode registrar uma ideia ou escolher algo do Arquivo.</p>
                <button class="re-outline" onclick={() => { void switchView('archive'); }}>Explorar meu arquivo <ArrowUpRight size={15}/></button>
              </div>
            {/if}
          </section>

          <div class="re-now-columns">
            <section class="re-section re-next" aria-label="Próximas tarefas">
              <div class="re-section-heading"><div><span class="re-eyebrow">EM SEGUIDA</span><h3>Próximas ações</h3></div><span class="re-section-counter">{otherTasks.length}</span></div>
              {#if otherTasks.length}
                {#each otherTasks.slice(0, 5) as task (task.id)}
                  <div class="re-next-item">
                    <button class="re-circle-action" title={'Concluir ' + task.title} aria-label={'Concluir ' + task.title} disabled={busy} onclick={() => { void completeTask(task); }}><Circle size={18}/></button>
                    <button class="re-next-label" onclick={() => { void archiveEntry('task:' + task.id); }}>{task.title}</button>
                    <button class="re-icon re-focus-small" title="Definir como foco" aria-label={'Focar ' + task.title} onclick={() => chooseFocus(task.id)}><Target size={16}/></button>
                  </div>
                {/each}
                {#if otherTasks.length > 5}<button class="re-inline-link" onclick={() => { void openPeriod('today'); }}>Ver todas <ArrowRight size={14}/></button>{/if}
              {:else}
                <div class="re-subtle-empty"><CheckCircle2 size={18}/> Nada esperando na sequência.</div>
              {/if}
              {#if laterTasks.length || somedayTasks.length}
                <div class="re-period-chips">
                  {#if laterTasks.length}<button onclick={() => { void openPeriod('later'); }}><CalendarDays size={14}/> Depois <b>{laterTasks.length}</b></button>{/if}
                  {#if somedayTasks.length}<button onclick={() => { void openPeriod('someday'); }}><Coffee size={14}/> Algum dia <b>{somedayTasks.length}</b></button>{/if}
                </div>
              {/if}
            </section>

            <section class="re-section re-quick" aria-label="Captura">
              <div class="re-section-heading"><div><span class="re-eyebrow">ESPAÇO LIVRE</span><h3>Guarde o pensamento</h3></div><NotebookPen size={20}/></div>
              <form class="re-compose" onsubmit={(e) => { e.preventDefault(); void captureInline(); }}>
                <div class="re-kind-switch" role="group" aria-label="Tipo de captura">
                  <button type="button" class:chosen={captureKind === 'idea'} aria-pressed={captureKind === 'idea'} onclick={() => captureKind = 'idea'}>Ideia</button>
                  <button type="button" class:chosen={captureKind === 'task'} aria-pressed={captureKind === 'task'} onclick={() => captureKind = 'task'}>Tarefa</button>
                </div>
                <label for="re-newtext" class="re-visually-hidden">O que você quer guardar?</label>
                <div class="re-compose-box">
                  <textarea id="re-newtext" rows="3" maxlength="100000" bind:value={newText}
                    placeholder={captureKind === 'task' ? 'Qual é a próxima ação?' : 'Escreva antes de esquecer...'}></textarea>
                  <div class="re-compose-bottom">
                    <span>Sem pressão para organizar.</span>
                    <button type="submit" class="re-primary re-square" aria-label={captureKind === 'task' ? 'Criar tarefa' : 'Guardar ideia'} title="Guardar" disabled={busy || !newText.trim()}><ArrowRight size={19}/></button>
                  </div>
                </div>
                {#if captureKind === 'task' && smartDraft.recognized}<p class="re-compose-preview"><CalendarDays size={14}/>{smartDraft.label} · sem notificação automática</p>{/if}
              </form>
            </section>
          </div>

          <section class="re-section re-reminders" aria-label="Central de lembretes">
            <div class="re-section-heading re-reminder-heading">
              <div><span class="re-eyebrow">NÃO DEIXE PASSAR</span><h3><Bell size={18}/> Lembretes</h3></div>
              {#if attentionReminders.length}<span class="re-attention-count">{attentionReminders.length} pendente(s)</span>{:else}<span class="re-quiet-status"><Check size={14}/> Em dia</span>{/if}
            </div>
            {#if attentionReminders.length}
              <div class="re-attention-list">
                {#each (showAllReminders ? attentionReminders : attentionReminders.slice(0,3)) as item (item.id)}
                  <div class="re-alert-item">
                    <div class="re-alert-icon"><Bell size={17}/></div>
                    <div class="re-alert-body">
                      <button onclick={() => { void openReminder(item); }}><strong>{item.title}</strong><ArrowUpRight size={15}/></button>
                      <p>{item.target_type === 'task' ? 'Tarefa' : 'Nota'} · Venceu {formatDateTime(item.anchor_at)}</p>
                      {#if item.last_error}<small>O Windows não confirmou o envio do aviso.</small>{/if}
                    </div>
                    <div class="re-alert-actions">
                      <button disabled={reminderBusy} onclick={() => { void snoozeReminder(item, 15); }}><AlarmClock size={14}/> +15 min</button>
                      <button disabled={reminderBusy} onclick={() => { void snoozeReminder(item, 60); }}>+1 h</button>
                      <button class="re-alert-done" disabled={reminderBusy} onclick={() => { void acknowledgeReminder(item); }}><Check size={15}/> Resolver</button>
                    </div>
                  </div>
                {/each}
                {#if attentionReminders.length > 3}<button class="re-inline-link" onclick={() => showAllReminders = !showAllReminders}>{showAllReminders ? 'Mostrar menos' : 'Ver todos os ' + attentionReminders.length + ' lembretes'} <ChevronRight size={14}/></button>{/if}
              </div>
            {:else if !upcomingReminders.length}
              <p class="re-reminder-empty"><Bell size={17}/> Nenhum lembrete ativo. Você pode agendar avisos nos detalhes de notas ou tarefas.</p>
            {/if}
            {#if upcomingReminders.length}
              <div class="re-upcoming-heading">PRÓXIMOS AVISOS</div>
              <div class="re-upcoming-list">
                {#each upcomingReminders.slice(0,5) as item (item.id)}
                  <button onclick={() => { void openReminder(item); }}>
                    <span class="re-upcoming-name">{item.title} {#if item.repeat_rule !== 'none'}<Repeat2 size={13}/>{/if}</span>
                    <span class="re-upcoming-time">{item.state === 'snoozed' ? 'Adiado · ' : ''}{formatDateTime(item.due_at)}</span>
                    <ChevronRight size={14}/>
                  </button>
                {/each}
              </div>
            {/if}
            {#if reminderHistory.length}
              <details class="re-history">
                <summary>Histórico recente <ChevronRight size={14}/></summary>
                <div class="re-history-list">
                  {#each reminderHistory.slice(0,12) as event (event.id)}
                    <div><span>{event.kind === 'acknowledged' ? 'Resolvido' : event.kind === 'snoozed' ? 'Adiado' : event.kind === 'attention' ? 'Vencido' : event.kind === 'dispatched' ? 'Enviado ao Windows' : event.kind === 'dispatch_failed' ? 'Falha no envio' : event.kind === 'suppressed' ? 'Agrupado no resumo' : event.kind === 'cancelled' ? 'Desativado' : 'Agendado'}</span>
                      <span>{event.target_type === 'task' ? 'Tarefa' : 'Nota'} · {formatDateTime(event.occurred_at)}</span></div>
                  {/each}
                </div>
              </details>
            {/if}
          </section>
          <footer class="re-page-foot">RECALL <span>·</span> Um lugar para tirar as coisas da cabeça.</footer>
        </div>
      </div>
    {:else}
      <div class="re-library">
        <section class="re-collection" aria-label="Explorar o arquivo">
          <div class="re-collection-top">
            <div class="re-collection-title"><p class="re-eyebrow">TUDO EM SEU LUGAR</p><h1>Arquivo<span class="re-title-dot">.</span></h1><p>Ideias, notas e tarefas. Tudo aqui.</p></div>
            <button class="re-icon re-newdoc" title="Nova nota" aria-label="Criar nova nota" disabled={busy} onclick={() => { void createDocument('note'); }}><Plus size={20}/></button>
            <label class="re-search"><Search size={17}/><input aria-label="Pesquisar em todo o arquivo" placeholder="Pesquisar..." bind:value={search}/><kbd>Ctrl K</kbd></label>
            <div class="re-filterbar" role="group" aria-label="Filtrar itens">
              <button class:current={archiveFilter === 'all' && archivePeriod === 'all'} onclick={() => { void setCategory('all'); }}>Tudo</button>
              <button class:current={archiveFilter === 'note'} onclick={() => { void setCategory('note'); }}>Notas</button>
              <button class:current={archiveFilter === 'task'} onclick={() => { void setCategory('task'); }}>Tarefas</button>
              <button class:current={archiveFilter === 'idea'} onclick={() => { void setCategory('idea'); }}>Ideias</button>
            </div>
            {#if archiveFilter === 'task'}
              <div class="re-period-filter" role="group" aria-label="Filtrar tarefas por período">
                <button class:current={archivePeriod === 'all'} onclick={() => archivePeriod = 'all'}>Todas</button>
                <button class:current={archivePeriod === 'today'} onclick={() => archivePeriod = 'today'}>Hoje</button>
                <button class:current={archivePeriod === 'later'} onclick={() => archivePeriod = 'later'}>Depois</button>
                <button class:current={archivePeriod === 'someday'} onclick={() => archivePeriod = 'someday'}>Algum dia</button>
              </div>
            {/if}
            <div class="re-collection-count">{visibleEntries.length} {visibleEntries.length === 1 ? 'registro' : 'registros'}</div>
          </div>
          <div class="re-entry-list">
            {#if !visibleEntries.length}
              <div class="re-entry-empty"><Inbox size={23}/><p>Nada encontrado.</p><small>Experimente outra pesquisa ou filtro.</small></div>
            {:else}
              {#each visibleEntries as item (item.key)}
                <button class="re-entry" class:selected={selectedKey === item.key} onclick={() => { void openEntry(item.key); }}>
                  <span class="re-entry-icon">{#if item.type === 'task'}<CheckCircle2 size={17}/>{:else if item.type === 'idea'}<Sparkles size={17}/>{:else if item.note.kind === 'presentation'}<Presentation size={17}/>{:else}<FileText size={17}/>{/if}</span>
                  <span class="re-entry-copy">
                    <strong>{#if item.type === 'note' && item.note.pinned}<Pin size={13} aria-label="Nota fixada"/>{/if}{item.title}</strong>
                    <small>{item.text?.slice(0,120) || (item.type === 'task' ? 'Tarefa sem detalhes' : 'Sem conteúdo')}</small>
                    <span class="re-entry-sub">
                      {item.type === 'idea' ? 'Ideia' : item.type === 'task' ? (item.task.completed_at ? 'Concluída' : taskPeriod(item.task, clock) === 'today' ? 'Hoje' : taskPeriod(item.task, clock) === 'later' ? 'Depois' : 'Algum dia') : item.note.kind === 'presentation' ? 'Apresentação' : 'Nota'}
                      <span>·</span>
                      {new Intl.DateTimeFormat('pt-BR', {day:'2-digit',month:'short'}).format(item.updatedAt)}
                    </span>
                  </span>
                  <ChevronRight size={15} class="re-entry-arrow"/>
                </button>
              {/each}
            {/if}
          </div>
          <div class="re-collection-bottom">
            <button onclick={() => { void createDocument('note'); }} disabled={busy}><Plus size={16}/> Nova nota</button>
            <button onclick={() => { void createDocument('presentation'); }} disabled={busy}><Presentation size={15}/> Apresentação</button>
          </div>
        </section>

        <section class="re-inspector" aria-label="Editar registro selecionado">
          {#if !selected}
            <div class="re-no-selection">
              <div class="re-no-selection-mark"><FileText size={30} strokeWidth={1.2}/></div>
              <p class="re-eyebrow">SEU ESPAÇO, SEU RITMO</p>
              <h2>Um lugar para tudo.</h2>
              <p>Selecione algo à esquerda para continuar de onde estava ou crie uma nova nota.</p>
              <button class="re-outline" disabled={busy} onclick={() => { void createDocument('note'); }}><Plus size={16}/> Criar uma nota</button>
              <div class="re-no-selection-shortcut"><Keyboard size={14}/> Ctrl + K para pesquisar</div>
            </div>
          {:else if selected.type === 'idea'}
            <div class="re-editor">
              <div class="re-editor-toolbar"><span><Sparkles size={16}/> IDEIA GUARDADA</span>
                <details class="re-more-menu"><summary aria-label="Mais ações"><MoreHorizontal size={18}/></summary><button class="re-destructive" onclick={() => { void deleteSelected(); }}><Trash2 size={15}/> Descartar ideia</button></details>
              </div>
              <div class="re-editor-head"><h1>Uma ideia vale guardar.</h1><p>Edite livremente ou transforme em algo para fazer.</p></div>
              <label class="re-field"><span>CONTEÚDO</span><textarea class="re-idea-body" rows="12" maxlength="100000" value={ideaText} placeholder="O que veio à sua cabeça?" oninput={(e) => onIdeaEdit(e.currentTarget.value)} onblur={() => { void saveIdea(selected.idea); }}></textarea></label>
              <div class="re-save-state" aria-live="polite"><span class:pending={ideaDirty || ideaSaving}></span>{ideaSaving ? 'Salvando...' : ideaDirty ? 'Alterações pendentes' : 'Salvo automaticamente'}</div>
              <div class="re-editor-footer"><button class="re-primary" disabled={busy} onclick={() => { void convertIdea(selected.idea,'note'); }}><FileText size={16}/> Transformar em nota</button>
                <button class="re-outline" disabled={busy} onclick={() => { void convertIdea(selected.idea,'task'); }}><CheckCircle2 size={16}/> Criar tarefa</button></div>
            </div>
          {:else if selected.type === 'task'}
            <div class="re-editor">
              <div class="re-editor-toolbar"><span><CheckCircle2 size={16}/> TAREFA</span>
                <details class="re-more-menu"><summary aria-label="Mais ações"><MoreHorizontal size={18}/></summary><button class="re-destructive" onclick={() => { void deleteSelected(); }}><Trash2 size={15}/> Excluir tarefa</button></details>
              </div>
              <div class="re-editor-head"><p class="re-eyebrow">PRÓXIMA AÇÃO</p>
                <label for="re-task-title" class="re-visually-hidden">Título da tarefa</label>
                <input id="re-task-title" class="re-title-input" maxlength="250" placeholder="Qual é o próximo passo?" value={taskTitle} oninput={(e) => onTaskEdit('title',e.currentTarget.value)} onblur={() => { void saveTask(selected.task); }}/>
              </div>
              <label class="re-field"><span>DETALHES OU PRÓXIMO PASSO</span><textarea rows="7" maxlength="100000" value={taskDetails} placeholder="Descreva o próximo passo, se quiser..." oninput={(e) => onTaskEdit('details',e.currentTarget.value)} onblur={() => { void saveTask(selected.task); }}></textarea></label>
              <div class="re-card-section">
                <div class="re-card-section-title"><CalendarDays size={17}/><strong>Planejamento</strong></div>
                <p>Quando você quer ver esta tarefa?</p>
                <div class="re-segmented" role="group" aria-label="Período da tarefa">
                  <button class:chosen={taskPeriod(selected.task, clock) === 'today'} aria-pressed={taskPeriod(selected.task, clock) === 'today'} disabled={busy} onclick={() => { void changeTaskPeriod(selected.task,'today'); }}>Hoje</button>
                  <button class:chosen={taskPeriod(selected.task, clock) === 'later'} aria-pressed={taskPeriod(selected.task, clock) === 'later'} disabled={busy} onclick={() => { void changeTaskPeriod(selected.task,'later'); }}>Depois</button>
                  <button class:chosen={taskPeriod(selected.task, clock) === 'someday'} aria-pressed={taskPeriod(selected.task, clock) === 'someday'} disabled={busy} onclick={() => { void changeTaskPeriod(selected.task,'someday'); }}>Algum dia</button>
                </div>
                {#if selected.task.bucket === 'later'}
                  <label class="re-field re-date-field"><span>VOLTAR PARA HOJE EM (OPCIONAL)</span>
                    <input type="date" value={selected.task.planned_for === null ? '' : toLocalInput(selected.task.planned_for).slice(0,10)} onchange={(e) => { void setPlannedDate(selected.task,e.currentTarget.value); }}/>
                  </label>
                {/if}
                <label class="re-field re-date-field"><span>PRAZO INFORMATIVO</span><input type="datetime-local" value={taskDue} onchange={(e) => onTaskEdit('due',e.currentTarget.value)} onblur={() => { void saveTask(selected.task); }}/></label>
              </div>
              <div class="re-card-section">
                <div class="re-card-section-title"><Bell size={17}/><strong>Lembrete</strong></div>
                <p>Um lembrete é um aviso; não muda quando a tarefa aparece em Hoje.</p>
                <label class="re-field re-date-field"><span>ME LEMBRAR EM</span><input type="datetime-local" value={reminderAt} oninput={(e) => reminderAt = e.currentTarget.value}/></label>
                <label class="re-field re-date-field"><span>REPETIÇÃO</span><select value={reminderRepeat} onchange={(e) => reminderRepeat = e.currentTarget.value as ReminderRule}>
                  <option value="none">Uma vez</option><option value="daily">Todos os dias</option><option value="weekly">Toda semana</option><option value="monthly">Todo mês</option>
                </select></label>
                {#if selectedReminder}<div class="re-scheduled-hint"><Bell size={14}/>{selectedReminder.state === 'attention' ? 'Precisa de atenção' : selectedReminder.state === 'snoozed' ? 'Adiado' : selectedReminder.state === 'done' ? 'Resolvido' : 'Agendado'} · {repeatLabel[selectedReminder.repeat_rule]} · {formatDateTime(selectedReminder.due_at)}</div>{/if}
                <div class="re-field-actions"><button class="re-outline" disabled={reminderBusy || !reminderAt} onclick={() => { void saveReminder(); }}><Bell size={15}/> Salvar lembrete</button>
                  {#if selectedReminder && selectedReminder.state !== 'done'}<button class="re-quiet-button" disabled={reminderBusy} onclick={() => { void removeReminder(); }}>Desativar</button>{/if}
                  {#if selectedReminder?.state === 'attention'}<button class="re-quiet-button" disabled={reminderBusy} onclick={() => { void snoozeReminder(selectedReminder,15); }}>Adiar 15 min</button><button class="re-quiet-button" disabled={reminderBusy} onclick={() => { void acknowledgeReminder(selectedReminder); }}>Resolvido</button>{/if}
                </div>
              </div>
              <div class="re-save-state" aria-live="polite"><span class:pending={taskDirty || taskSaving}></span>{taskSaving ? 'Salvando...' : taskDirty ? 'Alterações pendentes' : 'Salvo automaticamente'}</div>
              <div class="re-editor-footer"><button class="re-primary" disabled={busy} onclick={() => { void focusAndGo(selected.task.id); }}><Target size={16}/>{focusedTaskId === selected.task.id ? 'Em foco' : 'Focar agora'}</button>
                <button class="re-outline" disabled={busy} onclick={() => { void completeTask(selected.task); }}>{#if selected.task.completed_at}<RotateCcw size={16}/> Reabrir{:else}<Check size={16}/> Concluir tarefa{/if}</button>
              </div>
            </div>
          {:else}
            <div class="re-editor">
              <div class="re-editor-toolbar"><span>{#if selected.note.kind === 'presentation'}<Presentation size={16}/> APRESENTAÇÃO{:else}<FileText size={16}/> NOTA{/if}</span>
                <div class="re-editor-tools">
                  <button class="re-icon" title={selected.note.pinned ? 'Desafixar nota' : 'Fixar nota'} aria-label={selected.note.pinned ? 'Desafixar nota' : 'Fixar nota'} onclick={() => { void togglePin(selected.note); }}><Pin size={17} fill={selected.note.pinned ? 'currentColor' : 'none'}/></button>
                  <details class="re-more-menu"><summary aria-label="Mais ações"><MoreHorizontal size={18}/></summary><button class="re-destructive" onclick={() => { void deleteSelected(); }}><Trash2 size={15}/> Excluir nota</button></details>
                </div>
              </div>
              <div class="re-editor-head">
                <label for="re-note-title" class="re-visually-hidden">Título da nota</label>
                <input id="re-note-title" class="re-title-input" maxlength="250" placeholder="Nota sem título" value={noteTitle} oninput={(e) => onNoteEdit('title',e.currentTarget.value)} onblur={() => { void flushNote(); }}/>
              </div>
              <label for="re-note-body" class="re-visually-hidden">Conteúdo da nota</label>
              <textarea id="re-note-body" class="re-note-body" maxlength="100000" placeholder="Comece a escrever..." value={noteBody} oninput={(e) => onNoteEdit('body',e.currentTarget.value)} onblur={() => { void flushNote(); }}></textarea>
              <div class="re-save-state" aria-live="polite"><span class:pending={noteDirty || noteSaving}></span>{noteConflict ? 'Conflito — salve uma cópia' : noteSaving ? 'Salvando...' : noteDirty ? 'Alterações pendentes' : 'Salvo automaticamente'}</div>
              <div class="re-card-section">
                <div class="re-card-section-title"><Bell size={17}/><strong>Lembrete</strong></div>
                <p>Você decide quando receber um aviso sobre esta nota.</p>
                <label class="re-field re-date-field"><span>ME LEMBRAR EM</span><input type="datetime-local" value={reminderAt} oninput={(e) => reminderAt = e.currentTarget.value}/></label>
                <label class="re-field re-date-field"><span>REPETIÇÃO</span><select value={reminderRepeat} onchange={(e) => reminderRepeat = e.currentTarget.value as ReminderRule}>
                  <option value="none">Uma vez</option><option value="daily">Todos os dias</option><option value="weekly">Toda semana</option><option value="monthly">Todo mês</option>
                </select></label>
                {#if selectedReminder}<div class="re-scheduled-hint"><Bell size={14}/>{selectedReminder.state === 'attention' ? 'Precisa de atenção' : selectedReminder.state === 'snoozed' ? 'Adiado' : selectedReminder.state === 'done' ? 'Resolvido' : 'Agendado'} · {repeatLabel[selectedReminder.repeat_rule]} · {formatDateTime(selectedReminder.due_at)}</div>{/if}
                <div class="re-field-actions"><button class="re-outline" disabled={reminderBusy || !reminderAt} onclick={() => { void saveReminder(); }}><Bell size={15}/> Salvar lembrete</button>
                  {#if selectedReminder && selectedReminder.state !== 'done'}<button class="re-quiet-button" disabled={reminderBusy} onclick={() => { void removeReminder(); }}>Desativar</button>{/if}
                  {#if selectedReminder?.state === 'attention'}<button class="re-quiet-button" disabled={reminderBusy} onclick={() => { void snoozeReminder(selectedReminder,15); }}>Adiar 15 min</button><button class="re-quiet-button" disabled={reminderBusy} onclick={() => { void acknowledgeReminder(selectedReminder); }}>Resolvido</button>{/if}
                </div>
              </div>
              <div class="re-editor-footer">
                <button class="re-outline" onclick={() => { void floatNote(selected.note); }}><MonitorUp size={16}/> Cartão flutuante</button>
                <button class="re-outline" onclick={() => { void openPresentation(); }}><Presentation size={16}/> Apresentar / continuar</button>
              </div>
              <p class="re-editor-tip">Dica: separe cartões de uma apresentação usando <code>---</code> em uma linha.</p>
            </div>
          {/if}
        </section>
      </div>
    {/if}

    {#if presenting && selected?.type === 'note'}
      <div class="re-presentation" role="dialog" aria-modal="true" aria-label="Apresentação">
        <div class="re-presentation-head"><span>RECALL / APRESENTAÇÃO</span><button aria-label="Fechar apresentação" onclick={() => presenting = false}><X size={22}/></button></div>
        <div class="re-presentation-content"><div class="re-slide-counter">{Math.min(slideIndex + 1, Math.max(slides.length, 1))} <span>/</span> {Math.max(slides.length, 1)}</div>
          <h2>{slides[slideIndex] || 'Escreva uma nota com --- entre os cartões.'}</h2>
        </div>
        <div class="re-presentation-bottom"><button disabled={slideIndex === 0} onclick={() => moveSlide(-1)}><ChevronLeft size={18}/> Anterior</button><span>Retoma do último cartão · ← → navegar · Esc fechar</span>
          <button onclick={() => moveSlide(-slideIndex)} disabled={slideIndex === 0}>Recomeçar</button>
          <button disabled={slideIndex >= slides.length - 1} onclick={() => moveSlide(1)}>Próximo <ChevronRight size={18}/></button></div>
      </div>
    {/if}
  </div>
</div>
