/**
 * Setup Wizard Store (Svelte 5 Runes)
 * Manages onboarding wizard state and API interactions
 */

import { toasts } from './toasts';

interface WizardData {
  fshareEmail: string;
  fsharePassword: string;
  downloadPath: string;
  maxConcurrent: number;
  jellyfinEnabled: boolean;
  jellyfinUrl: string;
  jellyfinApiKey: string;
}

class SetupStore {
  currentStep = $state(0);
  isComplete = $state(false);
  isLoading = $state(false);

  data = $state<WizardData>({
    fshareEmail: '',
    fsharePassword: '',
    downloadPath: '/downloads',
    maxConcurrent: 6,
    jellyfinEnabled: false,
    jellyfinUrl: 'http://localhost:8096',
    jellyfinApiKey: '',
  });

  fshareValidated = $state(false);
  jellyfinValidated = $state(false);

  async checkStatus(): Promise<boolean> {
    try {
      const res = await fetch('/api/setup/status');
      if (res.ok) {
        const data = await res.json();
        this.isComplete = data.complete;
        return data.complete;
      }
    } catch (e) {
      console.error('Failed to check setup status:', e);
    }
    return false;
  }

  async validateFshare(): Promise<boolean> {
    this.isLoading = true;
    try {
      const payload = {
        email: this.data.fshareEmail,
        password: this.data.fsharePassword,
      };

      // Use the same endpoint as the in-app FShare login so setup and settings behave identically.
      const res = await fetch('/api/accounts', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      });

      if (res.ok) {
        const result = await res.json();
        if (result.success) {
          await fetch(`/api/accounts/${encodeURIComponent(this.data.fshareEmail)}/refresh`, { method: 'POST' });
          this.fshareValidated = true;
          toasts.success(result.message || 'FShare account connected successfully!');
          return true;
        }
        toasts.error(result.message || 'Invalid credentials');
      }
    } catch (e) {
      toasts.error('Failed to connect to FShare');
      console.error('FShare validation error:', e);
    } finally {
      this.isLoading = false;
    }
    return false;
  }

  async testJellyfin(): Promise<boolean> {
    this.isLoading = true;
    try {
      const res = await fetch('/api/setup/jellyfin/test', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          url: this.data.jellyfinUrl,
          api_key: this.data.jellyfinApiKey,
        }),
      });

      if (res.ok) {
        const result = await res.json();
        if (result.success) {
          this.jellyfinValidated = true;
          const msg = result.version
            ? `Connected to Jellyfin v${result.version}`
            : 'Connected to Jellyfin';
          toasts.success(msg);
          return true;
        }
        toasts.error(result.message || 'Connection failed');
      }
    } catch (e) {
      toasts.error('Failed to connect to Jellyfin');
      console.error('Jellyfin test error:', e);
    } finally {
      this.isLoading = false;
    }
    return false;
  }

  async completeSetup(): Promise<boolean> {
    this.isLoading = true;
    try {
      const payload: any = {
        downloads: {
          directory: this.data.downloadPath,
          max_concurrent: this.data.maxConcurrent,
        },
      };

      if (this.data.fshareEmail && this.data.fsharePassword) {
        payload.fshare = {
          email: this.data.fshareEmail,
          password: this.data.fsharePassword,
        };
      }

      if (this.data.jellyfinEnabled && this.data.jellyfinUrl && this.data.jellyfinApiKey) {
        payload.jellyfin = {
          url: this.data.jellyfinUrl,
          api_key: this.data.jellyfinApiKey,
        };
      }

      const res = await fetch('/api/setup/complete', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      });

      if (res.ok) {
        const result = await res.json();
        if (result.success) {
          this.isComplete = true;
          toasts.success('Setup completed successfully!');
          return true;
        }
        toasts.error(result.message || 'Failed to complete setup');
      }
    } catch (e) {
      toasts.error('Failed to complete setup');
      console.error('Complete setup error:', e);
    } finally {
      this.isLoading = false;
    }
    return false;
  }

  nextStep() {
    if (this.currentStep < 1) this.currentStep++;
  }

  prevStep() {
    if (this.currentStep > 0) this.currentStep--;
  }

  goToStep(step: number) {
    if (step >= 0 && step <= 1) this.currentStep = step;
  }

  reset() {
    this.currentStep = 0;
    this.isComplete = false;
    this.fshareValidated = false;
    this.jellyfinValidated = false;
  }
}

export const setupStore = new SetupStore();
