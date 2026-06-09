<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    open: boolean;
    onClose: () => void;
    maxWidth?: string;
    maxHeight?: string;
    accent?: string;
    ariaLabel?: string;
    noPad?: boolean;
    header?: Snippet;
    children?: Snippet;
    footer?: Snippet;
  }

  let {
    open,
    onClose,
    maxWidth = "900px",
    maxHeight = "88vh",
    accent = "var(--color-primary, #a78bfa)",
    ariaLabel = "Dialog",
    noPad = false,
    header,
    children,
    footer,
  }: Props = $props();

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) onClose();
  }
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="modal-backdrop"
    onclick={handleBackdropClick}
    onkeydown={handleKeydown}
    role="dialog"
    aria-modal="true"
    aria-label={ariaLabel}
    tabindex="-1"
    style="--modal-accent: {accent}; --modal-max-width: {maxWidth}; --modal-max-height: {maxHeight};"
  >
    <div
      class="modal-panel"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      role="document"
    >
      {#if header}
        <div class="modal-header-banner">
          <div class="banner-vignette"></div>
          <div class="modal-header-content">
            {@render header()}
          </div>
        </div>
      {/if}

      <div class="modal-body custom-scrollbar" class:no-pad={noPad}>
        {#if children}
          {@render children()}
        {/if}
      </div>

      {#if footer}
        <div class="modal-footer">
          {@render footer()}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 9000;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1.5rem;
    background: rgba(3, 7, 18, 0.68);
    backdrop-filter: blur(18px) saturate(130%);
    -webkit-backdrop-filter: blur(18px) saturate(130%);
    animation: modal-fade-in 0.18s ease both;
  }
  @keyframes modal-fade-in { from { opacity: 0; } to { opacity: 1; } }

  .modal-panel {
    position: relative;
    display: flex;
    flex-direction: column;
    width: 100%;
    max-width: var(--modal-max-width, 900px);
    max-height: var(--modal-max-height, 88vh);
    overflow: hidden;
    border-radius: 30px;
    border: 1px solid color-mix(in srgb, var(--modal-accent, #a78bfa) 22%, rgba(255,255,255,0.12));
    background:
      radial-gradient(circle at 12% 0%, color-mix(in srgb, var(--modal-accent, #a78bfa) 12%, transparent), transparent 34%),
      linear-gradient(180deg, rgba(18, 30, 52, 0.96), rgba(10, 17, 30, 0.94));
    box-shadow: 0 34px 90px rgba(2, 6, 23, 0.48), inset 0 1px 0 rgba(255,255,255,0.05);
    animation: modal-slide-up 0.22s cubic-bezier(0.16, 1, 0.3, 1) both;
  }
  @keyframes modal-slide-up {
    from { opacity: 0; transform: translateY(16px) scale(0.98); }
    to { opacity: 1; transform: translateY(0) scale(1); }
  }

  .modal-header-banner {
    position: relative;
    flex-shrink: 0;
    overflow: hidden;
    background: linear-gradient(180deg, rgba(30, 41, 59, 0.62), rgba(15, 23, 42, 0.38));
    border-bottom: 1px solid rgba(191, 219, 254, 0.12);
  }
  .banner-vignette {
    position: absolute;
    inset: 0;
    background: radial-gradient(ellipse at 18% 0%, color-mix(in srgb, var(--modal-accent, #a78bfa) 10%, transparent), transparent 58%);
    pointer-events: none;
  }
  .modal-header-banner::before {
    content: "";
    position: absolute;
    left: 1.5rem;
    right: 1.5rem;
    top: 0;
    height: 1px;
    background: linear-gradient(90deg, transparent, color-mix(in srgb, var(--modal-accent, #a78bfa) 46%, transparent), transparent);
  }
  .modal-header-content {
    position: relative;
    z-index: 1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 1.15rem 1.75rem;
  }

  .modal-body { flex: 1; overflow-y: auto; padding: 1.5rem 1.75rem; min-height: 0; }
  .modal-body.no-pad { padding: 0; }
  .modal-footer { flex-shrink: 0; padding: 1rem 1.75rem 1.25rem; border-top: 1px solid rgba(191, 219, 254, 0.1); }

  :global(.modal-header-content .close-btn) {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border-radius: 999px;
    background: rgba(15, 23, 42, 0.7);
    border: 1px solid rgba(191, 219, 254, 0.14);
    color: #cbd5e1;
    cursor: pointer;
    transition: background 0.18s, color 0.18s, border-color 0.18s, transform 0.18s;
    padding: 0;
  }
  :global(.modal-header-content .close-btn:hover) {
    transform: translateY(-1px);
    background: rgba(30, 41, 59, 0.9);
    color: #fff;
    border-color: rgba(139, 92, 246, 0.28);
  }
  :global(.modal-header-content .close-btn .material-icons) { font-size: 1.15rem; line-height: 1; }

  .custom-scrollbar::-webkit-scrollbar { width: 6px; }
  .custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
  .custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(148, 163, 184, 0.18); border-radius: 999px; }
  .custom-scrollbar::-webkit-scrollbar-thumb:hover { background: rgba(148, 163, 184, 0.28); }
</style>
