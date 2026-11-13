/** The async IPC result cannot erase typing added after a save started. */
export function shouldClearCaptureAfterSave(
  currentText: string, currentKind: 'task' | 'idea',
  submittedText: string, submittedKind: 'task' | 'idea'
): boolean {
  return currentText === submittedText && currentKind === submittedKind;
}
