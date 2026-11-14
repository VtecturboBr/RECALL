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

/** Never clear an editor when newer keystrokes or a mode switch occurred mid-save. */
export function captureCanClearAfterSave(
  submittedText: string, submittedKind: 'idea' | 'task',
  currentText: string, currentKind: 'idea' | 'task'
): boolean {
  return submittedText === currentText && submittedKind === currentKind;
}
