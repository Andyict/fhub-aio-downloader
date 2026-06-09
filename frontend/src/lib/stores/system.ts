import { writable } from 'svelte/store';

export interface DownloadSettings {
  directory: string;
  max_concurrent: number;
  segments_per_download: number;
}

export interface IndexerSettings {
  enabled?: boolean;
  api_key: string;
  indexer_url: string;
}

export interface LogEntry {
  timestamp: string;
  level: string;
  message: string;
}

export const downloadSettings = writable<DownloadSettings>({
  directory: '/downloads',
  max_concurrent: 3,
  segments_per_download: 4,
});

export const indexerSettings = writable<IndexerSettings>({
  enabled: true,
  api_key: '',
  indexer_url: '',
});

export const systemLogs = writable<LogEntry[]>([]);
export const systemLoading = writable<boolean>(false);
export const systemError = writable<string | null>(null);
export const folderSourceUrl = writable<string>('');

const API_BASE = '/api';

export async function fetchDownloadSettings(): Promise<void> {
  try {
    const response = await fetch(`${API_BASE}/settings/downloads`);
    if (response.ok) downloadSettings.set(await response.json());
  } catch (err) {
    console.error('[SystemStore] Fetch download settings error:', err);
  }
}

export async function saveDownloadSettings(settings: DownloadSettings): Promise<{ success: boolean; message?: string }> {
  try {
    const response = await fetch(`${API_BASE}/settings/downloads`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(settings),
    });
    if (response.ok) {
      const data = await response.json();
      downloadSettings.set(settings);
      return { success: data.success, message: data.message };
    }
    return { success: false, message: 'Failed to save settings' };
  } catch (err) {
    console.error('[SystemStore] Save download settings error:', err);
    return { success: false, message: 'Network error' };
  }
}

export async function fetchIndexerSettings(): Promise<void> {
  try {
    const response = await fetch(`${API_BASE}/settings/indexer`);
    if (response.ok) indexerSettings.set(await response.json());
  } catch (err) {
    console.error('[SystemStore] Fetch API settings error:', err);
  }
}

export async function saveIndexerSettings(settings: IndexerSettings): Promise<{ success: boolean; message?: string }> {
  try {
    const response = await fetch(`${API_BASE}/settings/indexer`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(settings),
    });
    if (response.ok) {
      const data = await response.json();
      indexerSettings.set(settings);
      return { success: data.success, message: data.message };
    }
    return { success: false, message: 'Failed to save API settings' };
  } catch (err) {
    console.error('[SystemStore] Save API settings error:', err);
    return { success: false, message: 'Network error' };
  }
}

export async function generateIndexerApiKey(): Promise<string | null> {
  try {
    const response = await fetch(`${API_BASE}/settings/indexer/generate-key`);
    if (response.ok) {
      const data = await response.json();
      indexerSettings.update((state) => ({ ...state, api_key: data.api_key }));
      return data.api_key;
    }
  } catch (err) {
    console.error('[SystemStore] Generate API key error:', err);
  }
  return null;
}

export async function fetchLogs(lines: number = 50): Promise<void> {
  try {
    const response = await fetch(`${API_BASE}/system/logs?lines=${lines}`);
    if (response.ok) {
      const data = await response.json();
      systemLogs.set(data.logs || []);
    }
  } catch (err) {
    console.error('[SystemStore] Fetch logs error:', err);
  }
}

export function clearLogs(): void {
  systemLogs.set([]);
}

export async function fetchFolderSourceConfig(): Promise<void> {
  try {
    const response = await fetch(`${API_BASE}/folder-source/config`);
    if (response.ok) {
      const data = await response.json();
      folderSourceUrl.set(data.gist_url || '');
    }
  } catch (err) {
    console.error('[SystemStore] Fetch folder source config error:', err);
  }
}

export async function saveFolderSourceConfig(gistUrl: string): Promise<{ success: boolean; message?: string }> {
  try {
    const response = await fetch(`${API_BASE}/folder-source/config`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ gist_url: gistUrl }),
    });
    if (response.ok) {
      const data = await response.json();
      folderSourceUrl.set(gistUrl);
      return { success: data.success, message: data.message };
    }
    return { success: false, message: 'Failed to save folder source config' };
  } catch (err) {
    console.error('[SystemStore] Save folder source config error:', err);
    return { success: false, message: 'Network error' };
  }
}

export async function refreshFolderSourceCache(): Promise<{ success: boolean; message?: string }> {
  try {
    const response = await fetch(`${API_BASE}/folder-source/refresh`, { method: 'POST' });
    if (response.ok) return await response.json();
    return { success: false, message: 'Failed to refresh folder cache' };
  } catch (err) {
    console.error('[SystemStore] Refresh folder cache error:', err);
    return { success: false, message: 'Network error' };
  }
}

export const systemStore = {
  fetchDownloadSettings,
  saveDownloadSettings,
  fetchIndexerSettings,
  saveIndexerSettings,
  generateIndexerApiKey,
  fetchLogs,
  clearLogs,
  fetchFolderSourceConfig,
  saveFolderSourceConfig,
  refreshFolderSourceCache,
};
