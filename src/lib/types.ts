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
