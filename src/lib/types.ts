export interface Whiteboard {
  id: string;
  name: string;
  created_at: number;
  updated_at: number;
}
export interface BoardConnection {
  id: string;
  board_id: string;
  from_id: string;
  to_id: string;
  created_at: number;
}

export type BoardKind = 'note' | 'link' | 'table' | 'image';
export interface BoardItem {
  id: string;
  board_id: string;
  kind: BoardKind;
  x: number;
  y: number;
  width: number;
  height: number;
  content: string;
  revision: number;
  created_at: number;
  updated_at: number;
}

export interface Note {
  id: string;
  title: string;
  body: string;
  kind: 'note' | 'presentation';
  pinned: boolean;
  remind_at: number | null;
  notified_at: number | null;
  created_at: number;
  updated_at: number;
  revision: number;
  deleted_at: number | null;
}


export interface Task {
  id: string;
  title: string;
  details: string;
  due_at: number | null;
  bucket: 'today' | 'later' | 'someday';
  planned_for: number | null;
  completed_at: number | null;
  created_at: number;
  updated_at: number;
  deleted_at: number | null;
  revision: number;
}

export interface FocusSession {
  id: string;
  task_id: string;
  started_at: number;
  last_tick_at: number;
  active_ms: number;
  ended_at: number | null;
  state: 'active' | 'paused' | 'finished' | 'interrupted';
  summary: string;
}

export interface TaskResume {
  task_id: string;
  last_context: string;
  next_step: string;
  updated_at: number;
  revision: number;
}

export type ReviewKind = 'reminder' | 'overdue_task' | 'interrupted_focus' | 'missing_step' | 'stale_task';

export interface DailyReviewItem {
  kind: ReviewKind;
  target_type: 'note' | 'task';
  target_id: string;
  title: string;
  description: string;
  occurred_at: number;
}

export interface ItemLink {
  id: string;
  item_type: 'note' | 'task' | 'idea';
  item_id: string;
  title: string;
  preview: string;
  created_at: number;
}

export interface InboxItem {
  id: string;
  text: string;
  created_at: number;
  updated_at: number;
  processed_at: number | null;
  revision: number;
}

export type ReminderRule = 'none' | 'daily' | 'weekly' | 'monthly';
export type ReminderState = 'scheduled' | 'snoozed' | 'attention' | 'done' | 'cancelled';
export interface Reminder {
  id: string;
  target_type: 'note' | 'task';
  target_id: string;
  title: string;
  details: string;
  due_at: number;
  anchor_at: number;
  repeat_rule: ReminderRule;
  state: ReminderState;
  alerted_at: number | null;
  dispatched_at: number | null;
  last_error: string | null;
  created_at: number;
  updated_at: number;
}
export interface ReminderEvent {
  id: number;
  reminder_id: string;
  target_type: 'note' | 'task';
  target_id: string;
  kind: string;
  occurred_at: number;
  scheduled_for: number;
}
