import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { invoke } from '@tauri-apps/api/core';

export const openQuickCapture = (): Promise<void> => invoke<void>('show_capture');

export async function openSticky(id: string): Promise<void> {
  const label = `note_${id.replace(/[^a-zA-Z0-9_-]/g, '_')}`;
  const existing = await WebviewWindow.getByLabel(label);
  if (existing) {
    await existing.show();
    await existing.setFocus();
    return;
  }
  const win = new WebviewWindow(label, {
    title: 'Nota flutuante',
    url: `/?sticky=${encodeURIComponent(id)}`,
    width: 360,
    height: 290,
    minWidth: 260,
    minHeight: 170,
    decorations: false,
    alwaysOnTop: false,
    resizable: true,
    skipTaskbar: false,
    center: true
  });
  await new Promise<void>((resolve, reject) => {
    win.once('tauri://created', () => resolve());
    win.once('tauri://error', (event) => reject(new Error(String(event.payload))));
  });
}
