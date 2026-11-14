import { invoke } from '@tauri-apps/api/core';
import { shouldClearFocusDraft } from './focusDraftSafety';
import type { Note, Task, TaskResume, FocusSession, ItemLink, DailyReviewItem, InboxItem, Reminder, ReminderEvent, ReminderRule, BoardItem, BoardKind } from './types';

export const boardApi = {
  list: () => invoke<BoardItem[]>('list_board_items'),
  create: (kind: BoardKind,x: number,y: number,content: string,assetDataUrl?: string) =>
    invoke<BoardItem>('create_board_item',{kind,x,y,content,assetDataUrl}),
  update: (item: BoardItem) => invoke<BoardItem>('update_board_item',{
    id:item.id,x:item.x,y:item.y,width:item.width,height:item.height,
    content:item.content,expectedRevision:item.revision
  }),
  remove: (id: string,expectedRevision: number) =>
    invoke<void>('delete_board_item',{id,expectedRevision}),
  asset: (id: string) => invoke<string | null>('get_board_asset',{id})
};

export const notesApi = {
  list: () => invoke<Note[]>('list_notes'),
  get: (id: string) => invoke<Note | null>('get_note', { id }),
  create: (kind: 'note' | 'presentation') => invoke<Note>('create_note', { kind }),
  save: (id: string, title: string, body: string, expectedRevision = 0) => invoke<Note>('save_note', { id, title, body, expectedRevision }),
  copy: (title: string, body: string, kind: Note['kind']) => invoke<Note>('create_note_copy', { title, body, kind }),
  pin: (id: string, pinned: boolean) => invoke<Note>('set_note_pin', { id, pinned }),
  remind: (id: string, remindAt: number | null) => invoke<Note>('set_note_reminder', { id, remindAt }),
  delete: (id: string) => invoke<void>('delete_note', { id }),
  capture: (text: string) => invoke<Note>('capture_note', { text })
};

export function formatDateTime(timestamp: number | null): string {
  if (!timestamp) return '';
  return new Intl.DateTimeFormat('pt-BR', { day: '2-digit', month: 'short', hour: '2-digit', minute: '2-digit' }).format(timestamp);
}
export function toLocalInput(timestamp: number | null): string {
  if (!timestamp) return '';
  const d = new Date(timestamp);
  const p = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}`;
}


type FocusDraft = { id: string; raw: string; summary: string };
async function focusDraftForTask(taskId: string): Promise<FocusDraft | null> {
  const active = await invoke<FocusSession | null>('get_active_focus');
  if (!active || active.task_id !== taskId) return null;
  try {
    const raw = localStorage.getItem('recall-focus-summary:' + active.id);
    return raw === null ? null : { id: active.id, raw, summary: raw.slice(0, 2000) };
  } catch { return null; }
}
function clearFocusDraftIfUnchanged(draft: FocusDraft | null) {
  if (!draft) return;
  try {
    const key = 'recall-focus-summary:' + draft.id;
    if (shouldClearFocusDraft(draft.raw, localStorage.getItem(key))) localStorage.removeItem(key);
  } catch { /* Keep draft for manual recovery. */ }
}

export const tasksApi = {
  list: () => invoke<Task[]>('list_tasks'),
  create: (title: string) => invoke<Task>('create_task', { title }),
  createPlanned: (title: string, details: string, bucket: Task['bucket'], plannedFor: number | null, dueAt: number | null) =>
    invoke<Task>('create_planned_task', { title, details, bucket, plannedFor, dueAt }),
  plan: (id: string, bucket: Task['bucket'], plannedFor: number | null) =>
    invoke<Task>('set_task_plan', { id, bucket, plannedFor }),
  update: (id: string, title: string, details: string, dueAt: number | null, expectedRevision: number) =>
    invoke<Task>('update_task', { id, title, details, dueAt, expectedRevision }),
  getResume: (taskId: string) => invoke<TaskResume | null>('get_task_resume', { taskId }),
  saveResume: (taskId: string, lastContext: string, nextStep: string, expectedRevision: number) =>
    invoke<TaskResume>('save_task_resume', { taskId, lastContext, nextStep, expectedRevision }),
  complete: async (id: string, completed: boolean, focusSummary?: string) => {
    const draft = completed ? await focusDraftForTask(id) : null;
    const task = await invoke<Task>('set_task_completed', {
      id, completed, focusSummary: focusSummary ?? draft?.summary ?? null
    });
    if (completed) clearFocusDraftIfUnchanged(draft);
    return task;
  },
  delete: async (id: string, focusSummary?: string) => {
    const draft = await focusDraftForTask(id);
    await invoke<void>('delete_task', {
      id, focusSummary: focusSummary ?? draft?.summary ?? null
    });
    clearFocusDraftIfUnchanged(draft);
  }
};

export const focusApi = {
  active: () => invoke<FocusSession | null>('get_active_focus'),
  history: (taskId: string) => invoke<FocusSession[]>('list_focus_sessions', { taskId }),
  start: (taskId: string) => invoke<FocusSession>('start_focus_session', { taskId }),
  heartbeat: (id: string) => invoke<FocusSession>('heartbeat_focus_session', { id }),
  stop: (id: string, reason: 'paused' | 'finished', summary: string) =>
    invoke<FocusSession>('stop_focus_session', { id, reason, summary })
};

export const reviewApi = {
  list: (hiddenKeys: string[] = []) => invoke<DailyReviewItem[]>('list_daily_review', { hiddenKeys })
};

export const linksApi = {
  list: (itemType: ItemLink['item_type'], itemId: string) =>
    invoke<ItemLink[]>('list_item_links', { itemType, itemId }),
  add: (fromType: ItemLink['item_type'], fromId: string, toType: ItemLink['item_type'], toId: string) =>
    invoke<string>('add_item_link', { fromType, fromId, toType, toId }),
  remove: (id: string) => invoke<void>('remove_item_link', { id })
};

export const inboxApi = {
  list: () => invoke<InboxItem[]>('list_inbox'),
  capture: (text: string) => invoke<InboxItem>('capture_inbox', { text }),
  update: (id: string, text: string, expectedRevision: number) =>
    invoke<InboxItem>('update_inbox', { id, text, expectedRevision }),
  dismiss: (id: string) => invoke<void>('dismiss_inbox', { id }),
  toNote: (id: string) => invoke<Note>('inbox_to_note', { id }),
  toTask: (id: string) => invoke<Task>('inbox_to_task', { id })
};

export interface BackupInfo {
  file_name: string;
  created_at: number;
  size_bytes: number;
  file_path: string;
}
export const backupsApi = {
  create: () => invoke<BackupInfo>('create_data_backup'),
  list: () => invoke<BackupInfo[]>('list_data_backups')
};

export const remindersApi = {
  list: () => invoke<Reminder[]>('list_reminders'),
  history: () => invoke<ReminderEvent[]>('list_reminder_events'),
  schedule: (targetType: 'note' | 'task', targetId: string, dueAt: number, repeatRule: ReminderRule) =>
    invoke<Reminder>('schedule_reminder', { targetType, targetId, dueAt, repeatRule }),
  cancel: (targetType: 'note' | 'task', targetId: string) =>
    invoke<void>('cancel_reminder', { targetType, targetId }),
  snooze: (id: string, minutes: 5 | 15 | 30 | 60 | 180 | 1440) =>
    invoke<Reminder>('snooze_reminder', { id, minutes }),
  acknowledge: (id: string) =>
    invoke<Reminder>('acknowledge_reminder', { id })
};
