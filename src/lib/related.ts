import type { ItemLink } from './types';
import type { WorkspaceEntry } from './workspace';

/** Only active items in the current workspace can be selected as targets. */
export function availableLinkTargets(
  entries: WorkspaceEntry[], sourceKey: string, links: ItemLink[]
): WorkspaceEntry[] {
  const connected = new Set(links.map(link => link.item_type + ':' + link.item_id));
  return entries.filter(entry => entry.key !== sourceKey && !connected.has(entry.key))
    .sort((a, b) => a.title.localeCompare(b.title, 'pt-BR') || a.key.localeCompare(b.key));
}

export function linkedEntryKey(link: ItemLink): string {
  return link.item_type + ':' + link.item_id;
}
