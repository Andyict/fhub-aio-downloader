<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    onclick?: (e: MouseEvent) => void;
    accent?: string;
    variant?: "primary" | "ghost" | "danger";
    icon?: string;
    size?: "sm" | "md" | "lg";
    disabled?: boolean;
    loading?: boolean;
    class?: string;
    type?: "button" | "submit" | "reset";
    children?: Snippet;
    title?: string;
    width?: string;
  }

  let {
    onclick,
    accent = "var(--color-primary, #a78bfa)",
    variant = "primary",
    icon,
    size = "md",
    disabled = false,
    loading = false,
    class: cls = "",
    type = "button",
    children,
    title,
    width,
  }: Props = $props();
</script>

<button
  {type}
  {title}
  {disabled}
  class="fhub-btn fhub-btn--{variant} fhub-btn--{size} {cls}"
  style="--btn-accent: {accent};{width ? ` --btn-width: ${width};` : ''}"
  onclick={!disabled && !loading ? onclick : undefined}
>
  <span class="btn-inner">
    {#if loading}
      <span class="material-icons btn-spin">autorenew</span>
    {:else if icon}
      <span class="material-icons btn-icon">{icon}</span>
    {/if}
    {#if children}
      <span class="btn-label">{@render children()}</span>
    {/if}
  </span>
</button>

<style>
  .fhub-btn {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: var(--btn-width, auto);
    border: 1px solid transparent;
    border-radius: 16px;
    outline: none;
    cursor: pointer;
    user-select: none;
    overflow: hidden;
    white-space: nowrap;
    font-family: Inter, system-ui, sans-serif;
    font-weight: 800;
    letter-spacing: -0.01em;
    text-transform: none;
    transition:
      transform 0.18s ease,
      background 0.18s ease,
      border-color 0.18s ease,
      box-shadow 0.18s ease,
      opacity 0.18s ease;
  }

  .fhub-btn--sm { font-size: 0.76rem; padding: 0.48rem 0.85rem; gap: 0.35rem; min-height: 34px; }
  .fhub-btn--md { font-size: 0.86rem; padding: 0.64rem 1.1rem; gap: 0.45rem; min-height: 42px; }
  .fhub-btn--lg { font-size: 0.96rem; padding: 0.82rem 1.35rem; gap: 0.55rem; min-height: 50px; }

  .fhub-btn--primary {
    background: linear-gradient(135deg, rgba(139, 92, 246, 0.22), rgba(255, 179, 92, 0.18));
    color: #f2fbff;
    border-color: color-mix(in srgb, var(--btn-accent, #a78bfa) 34%, rgba(255,255,255,0.12));
    box-shadow: 0 12px 28px rgba(249, 115, 22, 0.12), inset 0 1px 0 rgba(255,255,255,0.06);
  }

  .fhub-btn--ghost {
    background: rgba(15, 23, 42, 0.5);
    color: #cbd5e1;
    border-color: rgba(191, 219, 254, 0.14);
    box-shadow: inset 0 1px 0 rgba(255,255,255,0.04);
  }

  .fhub-btn--danger {
    background: rgba(248, 113, 113, 0.11);
    color: #fecaca;
    border-color: rgba(248, 113, 113, 0.26);
    box-shadow: 0 12px 28px rgba(127, 29, 29, 0.12), inset 0 1px 0 rgba(255,255,255,0.04);
  }

  .fhub-btn:hover:not(:disabled) {
    transform: translateY(-1px);
    border-color: color-mix(in srgb, var(--btn-accent, #a78bfa) 46%, rgba(255,255,255,0.16));
    box-shadow: 0 16px 36px rgba(249, 115, 22, 0.16), inset 0 1px 0 rgba(255,255,255,0.08);
  }

  .fhub-btn:active:not(:disabled) { transform: translateY(0) scale(0.98); }
  .fhub-btn:disabled { opacity: 0.45; cursor: not-allowed; transform: none !important; }

  .btn-inner { position: relative; z-index: 1; display: flex; align-items: center; gap: inherit; }
  .btn-icon { font-size: 1.08rem; line-height: 1; }
  .btn-label { line-height: 1; }
  .btn-spin { animation: btn-spin-rotate 1s linear infinite; font-size: inherit; }
  @keyframes btn-spin-rotate { to { transform: rotate(360deg); } }
</style>
