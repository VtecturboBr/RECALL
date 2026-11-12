import type { Note, Task, InboxItem } from './types';

export type WorkspaceEntry =
  | { type: 'note'; key: string; title: string; text: string; updatedAt: number; note: Note }
  | { type: 'task'; key: string; title: string; text: string; updatedAt: number; task: Task }
  | { type: 'idea'; key: string; title: string; text: string; updatedAt: number; idea: InboxItem };

export function buildWorkspace(notes: Note[], tasks: Task[], ideas: InboxItem[]): WorkspaceEntry[] {
  const entries: WorkspaceEntry[] = [
    ...notes.filter(note => note.deleted_at === null).map(note => ({
      type: 'note' as const,
      key: 'note:' + note.id,
      title: note.title.trim() || 'Sem título',
      text: note.body,
      updatedAt: note.updated_at,
      note
    })),
    ...tasks.filter(task => task.deleted_at === null).map(task => ({
      type: 'task' as const,
      key: 'task:' + task.id,
      title: task.title,
      text: task.details,
      updatedAt: task.updated_at,
      task
    })),
    ...ideas.filter(idea => idea.processed_at === null).map(idea => ({
      type: 'idea' as const,
      key: 'idea:' + idea.id,
      title: idea.text.split('\n')[0]?.trim().slice(0, 90) || 'Ideia',
      text: idea.text,
      updatedAt: idea.updated_at,
      idea
    }))
  ];
  return entries.sort((a, b) => b.updatedAt - a.updatedAt || a.key.localeCompare(b.key));
}

export function nextTask(tasks: Task[]): Task | null {
  return [...tasks]
    .filter(t => t.deleted_at === null && t.completed_at === null)
    .sort((a, b) => (a.due_at ?? Number.POSITIVE_INFINITY) - (b.due_at ?? Number.POSITIVE_INFINITY)
      || a.created_at - b.created_at)[0] ?? null;
}

export function toSlides(content: string): string[] {
  return content.split(/^\s*---\s*$/m).map(s => s.trim()).filter(Boolean);
}
