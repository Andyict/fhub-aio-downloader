import { derived, writable } from 'svelte/store';

export interface FshareAccount {
  email: string;
  password?: string;
  rank?: string;
  quota_used?: number;
  quota_total?: number;
  valid_until?: number;
  is_active?: boolean;
}

export interface SettingsState {
  accounts: FshareAccount[];
  integrations: Record<string, any>;
  loading: boolean;
  error: string | null;
}

const API_BASE = '/api';

function createSettingsStore() {
  const { subscribe, set, update } = writable<SettingsState>({
    accounts: [],
    integrations: {},
    loading: false,
    error: null,
  });

  return {
    subscribe,
    set,
    update,
    async fetchSettings() {
      update((state) => ({ ...state, loading: true }));
      try {
        const response = await fetch(`${API_BASE}/settings`);
        const data = response.ok ? await response.json() : {};
        update((state) => ({ ...state, integrations: data, loading: false, error: null }));
        return data;
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
        return null;
      }
    },
    async fetchAccounts() {
      try {
        const response = await fetch(`${API_BASE}/accounts`);
        const data = response.ok ? await response.json() : [];
        const accounts = Array.isArray(data) ? data : (data.accounts || []);
        update((state) => ({ ...state, accounts }));
        return accounts;
      } catch (_) {
        return [];
      }
    },
    async addAccount(accountOrEmail: any, password?: string) {
      const account = typeof accountOrEmail === 'string'
        ? { email: accountOrEmail, password }
        : accountOrEmail;
      const response = await fetch(`${API_BASE}/accounts`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(account),
      });
      const text = await response.text();
      let data: any = { success: response.ok };
      if (text) {
        try {
          data = JSON.parse(text);
        } catch (_) {
          data = { success: false, message: text };
        }
      }
      if (!response.ok) {
        data = {
          ...data,
          success: false,
          status: response.status,
          message: data.message || data.error || (response.status === 401 ? 'Bạn cần đăng nhập FHUB trước khi lưu tài khoản FShare.' : 'Đăng nhập FShare thất bại'),
        };
      }
      await this.fetchAccounts();
      return data;
    },
    async refreshAccount(id: string) {
      const accountId = encodeURIComponent(id);
      const response = await fetch(`${API_BASE}/accounts/${accountId}/refresh`, { method: 'POST' });
      const data = response.ok ? await response.json() : { success: false };
      await this.fetchAccounts();
      return data;
    },
    async hasAccounts() {
      const list = await this.fetchAccounts();
      return Array.isArray(list) && list.length > 0;
    },
    async removeAccount(id: string) {
      const accountId = encodeURIComponent(id);
      const response = await fetch(`${API_BASE}/accounts/${accountId}`, { method: 'DELETE' });
      const data = response.ok ? await response.json() : { success: false };
      await this.fetchAccounts();
      return data;
    },
  };
}

export const settingsStore = createSettingsStore();
export const accounts = derived(settingsStore, ($state) => $state.accounts);
export const hasAccounts = (state: SettingsState) => state.accounts.length > 0;
(settingsStore as any).hasAccounts = async () => {
  const list = await settingsStore.fetchAccounts();
  return Array.isArray(list) && list.length > 0;
};

export function formatExpiry(timestamp: number): string {
  if (!timestamp) return 'N/A';
  const ms = timestamp < 10_000_000_000 ? timestamp * 1000 : timestamp;
  return new Date(ms).toLocaleDateString();
}

export function formatQuota(bytes: number): string {
  if (!bytes) return '0 GB';
  return `${(bytes / (1024 ** 3)).toFixed(2)} GB`;
}

export function getQuotaPercentage(used: number, total: number): number {
  if (!total) return 0;
  return Math.min(100, Math.round((used / total) * 100));
}
