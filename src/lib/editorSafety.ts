/** Empty titles are valid drafts; persist a stable, human-readable placeholder. */
export const UNNAMED_TASK = 'Nova tarefa';

export function taskTitleForSave(value: string): string {
  return value.trim() || UNNAMED_TASK;
}

/** An empty edit cannot overwrite an existing idea in SQLite. */
export function isEmptyIdeaEdit(value: string): boolean {
  return value.trim().length === 0;
}

/** An empty cached idea draft must never hide nonempty persisted content. */
export function restoreIdeaDraft(cached: string, persisted: string): boolean {
  return !isEmptyIdeaEdit(cached) && cached !== persisted;
}
