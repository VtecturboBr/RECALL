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
}

export interface InboxItem {
  id: string;
  text: string;
  created_at: number;
  updated_at: number;
  processed_at: number | null;
}
