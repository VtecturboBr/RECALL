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
  deleted_at: number | null;
}
