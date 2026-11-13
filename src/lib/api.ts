import { invoke } from '@tauri-apps/api/core';
import type { Note, Task, TaskResume, InboxItem, Reminder, ReminderEvent, ReminderRule } from './types';

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
  complete: (id: string, completed: boolean) => invoke<Task>('set_task_completed', { id, completed }),
  delete: (id: string) => invoke<void>('delete_task', { id })
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
