import { invoke } from '@tauri-apps/api/core';
import type { Note, Task, InboxItem } from './types';

export const notesApi = {
  list: () => invoke<Note[]>('list_notes'),
  get: (id: string) => invoke<Note | null>('get_note', { id }),
  create: (kind: 'note' | 'presentation') => invoke<Note>('create_note', { kind }),
  save: (id: string, title: string, body: string) => invoke<Note>('save_note', { id, title, body }),
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
  update: (id: string, title: string, details: string, dueAt: number | null) =>
    invoke<Task>('update_task', { id, title, details, dueAt }),
  complete: (id: string, completed: boolean) => invoke<Task>('set_task_completed', { id, completed }),
  delete: (id: string) => invoke<void>('delete_task', { id })
};

export const inboxApi = {
  list: () => invoke<InboxItem[]>('list_inbox'),
  capture: (text: string) => invoke<InboxItem>('capture_inbox', { text }),
  dismiss: (id: string) => invoke<void>('dismiss_inbox', { id }),
  toNote: (id: string) => invoke<Note>('inbox_to_note', { id }),
  toTask: (id: string) => invoke<Task>('inbox_to_task', { id })
};
